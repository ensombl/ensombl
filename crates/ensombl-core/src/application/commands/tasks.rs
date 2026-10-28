//! Orchestrated tasks: creating them, steering them, and the dispatcher that
//! runs queued tasks headlessly in their own worktrees.
//!
//! Each run is one agent process. Its events go to the run log (so they can be
//! replayed after a restart) and out on the event bus; the ones that matter
//! for the task (session, permission prompts, the end of the turn, exit) move
//! it through its status machine.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::application::cqrs::{Command, CommandHandler};
use crate::application::ports::{
    AgentConnection, AgentLaunch, AgentRuntime, Clock, EventBus, GitWorktrees, RunLog,
    SessionRepository, ShellEnvironment, TaskGit, TaskStore,
};
use crate::application::{Error, Result};
use crate::domain::agent_run::{PermissionDecision, PermissionMode, RunEvent};
use crate::domain::orchestration::{
    Run, RunOutcome, Task, TaskEvent, TaskId, TaskStatus, branch_name, schedule, transition,
};
use crate::domain::terminal::AgentKind;
use crate::domain::workspace::ProjectId;

/// What an orchestrated agent is told about its situation. Ensombl owns the
/// branch: it commits when the run ends and merges on approval.
pub const TASK_INSTRUCTIONS: &str = "You are working on a task for Ensombl, alone, in your own git worktree on your own branch. Make the changes the task asks for and check they work. Don't commit, push, merge or switch branches: Ensombl commits your work when you finish and merges it once a person approves it. End with a short summary of what you changed.";

/// Topic for task changes; the payload is the [`Task`].
pub const TASK_CHANGED: &str = "tasks://changed";
/// Topic for live run events; the payload is a [`RunEventPayload`].
pub const RUN_EVENT: &str = "tasks://run";

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RunEventPayload {
    pub task_id: TaskId,
    pub run_id: String,
    pub event: RunEvent,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateTask {
    pub project_id: ProjectId,
    pub title: String,
    pub prompt: String,
    pub agent: AgentKind,
    pub base_branch: String,
    #[serde(default)]
    pub depends_on: Vec<TaskId>,
    pub permission: PermissionMode,
    /// Queue it right away instead of leaving it in the backlog.
    #[serde(default)]
    pub queue: bool,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct QueueTask {
    pub task_id: TaskId,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CancelTask {
    pub task_id: TaskId,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RetryTask {
    pub task_id: TaskId,
}

/// Answers the permission prompt or question a task is blocked on.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RespondToTask {
    pub task_id: TaskId,
    pub request_id: String,
    pub decision: PermissionDecision,
}

/// Another turn for a task in review, continuing the agent's session.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SendFollowUp {
    pub task_id: TaskId,
    pub prompt: String,
}

/// Merges a reviewed task into its base branch and cleans up its worktree.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ApproveTask {
    pub task_id: TaskId,
}

/// Brings a task's branch up to date with its base (merging the base in).
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateTaskBranch {
    pub task_id: TaskId,
}

/// Forgets a task, throwing away its worktree and branch.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeleteTask {
    pub task_id: TaskId,
}

impl Command for CreateTask {
    type Output = Task;
}
impl Command for QueueTask {
    type Output = Task;
}
impl Command for CancelTask {
    type Output = Task;
}
impl Command for RetryTask {
    type Output = Task;
}
impl Command for RespondToTask {
    type Output = ();
}
impl Command for SendFollowUp {
    type Output = Task;
}
impl Command for ApproveTask {
    type Output = Task;
}
impl Command for DeleteTask {
    type Output = ();
}
impl Command for UpdateTaskBranch {
    type Output = ();
}

/// Everything the orchestrator talks to.
pub struct TaskPorts {
    pub tasks: Arc<dyn TaskStore>,
    pub log: Arc<dyn RunLog>,
    pub runtimes: HashMap<AgentKind, Arc<dyn AgentRuntime>>,
    pub sessions: Arc<dyn SessionRepository>,
    pub shell: Arc<dyn ShellEnvironment>,
    pub git: Arc<dyn GitWorktrees>,
    pub task_git: Arc<dyn TaskGit>,
    pub bus: Arc<dyn EventBus>,
    pub clock: Arc<dyn Clock>,
    /// Task worktrees go under `<worktrees_dir>/<project>/task-<id>`.
    pub worktrees_dir: PathBuf,
    /// How many tasks may run at once.
    pub concurrency: usize,
}

/// A run whose agent process is alive.
struct Live {
    run: Run,
    connection: Box<dyn AgentConnection>,
}

struct Inner {
    ports: TaskPorts,
    live: Mutex<HashMap<TaskId, Live>>,
    /// Serialises read-modify-write of tasks between commands and run events.
    edit: Mutex<()>,
    ids: AtomicU64,
}

pub struct TaskCommands {
    inner: Arc<Inner>,
}

impl TaskCommands {
    pub fn new(ports: TaskPorts) -> Self {
        Self {
            inner: Arc::new(Inner {
                ports,
                live: Mutex::default(),
                edit: Mutex::new(()),
                ids: AtomicU64::new(0),
            }),
        }
    }

    /// Runs left active by a previous process can't be resumed: their agent
    /// is gone. Marks them failed so they can be retried.
    pub fn recover(&self) -> Result<()> {
        let inner = &self.inner;
        for task in inner.ports.tasks.list()? {
            if task.status.is_active() {
                inner.apply(
                    &task.id,
                    TaskEvent::Failed {
                        error: "interrupted: Ensombl stopped while this was running".into(),
                    },
                )?;
            }
        }
        Ok(())
    }

    /// Starts whatever queued tasks fit.
    pub fn dispatch(&self) {
        Inner::dispatch(&self.inner);
    }

    /// Stops every running agent.
    pub fn shutdown(&self) {
        for (_, live) in self.inner.live.lock().unwrap().drain() {
            live.connection.kill();
        }
    }
}

impl Inner {
    fn now(&self) -> u64 {
        self.ports.clock.now()
    }

    fn new_id(&self) -> String {
        let n = self.ids.fetch_add(1, Ordering::Relaxed);
        format!("{}{}", radix36(self.now()), radix36(n))
    }

    fn get(&self, id: &str) -> Result<Task> {
        self.ports
            .tasks
            .get(id)?
            .ok_or_else(|| Error::Invalid(format!("no task {id}")))
    }

    fn save(&self, task: &Task) -> Result<()> {
        self.ports.tasks.put(task)?;
        self.ports
            .bus
            .publish(TASK_CHANGED, serde_json::to_value(task).unwrap_or_default());
        Ok(())
    }

    /// Applies an event to a stored task and saves the result.
    fn apply(&self, id: &str, event: TaskEvent) -> Result<Task> {
        let _guard = self.edit.lock().unwrap();
        let task = self.get(id)?;
        let next =
            transition(&task, event, self.now()).map_err(|e| Error::Invalid(e.to_string()))?;
        self.save(&next)?;
        Ok(next)
    }

    fn project_root(&self, project_id: &str) -> Result<(String, PathBuf)> {
        let session = self.ports.sessions.get()?;
        let project = session
            .project(project_id)
            .ok_or_else(|| Error::Invalid(format!("no project {project_id}")))?;
        Ok((project.name.clone(), PathBuf::from(&project.root)))
    }

    fn dispatch(self: &Arc<Self>) {
        let ready = {
            let _guard = self.edit.lock().unwrap();
            match self.ports.tasks.list() {
                Ok(tasks) => schedule(&tasks, self.ports.concurrency),
                Err(e) => {
                    eprintln!("dispatch: {e}");
                    return;
                }
            }
        };
        for id in ready {
            if let Err(error) = self.start(&id) {
                self.end_run(
                    &id,
                    RunOutcome::Failed {
                        error: error.to_string(),
                    },
                );
                if let Err(e) = self.apply(
                    &id,
                    TaskEvent::Failed {
                        error: error.to_string(),
                    },
                ) {
                    eprintln!("task {id}: {e}");
                }
            }
        }
    }

    /// Creates (or reuses) the task's worktree and starts its agent.
    fn start(self: &Arc<Self>, id: &str) -> Result<()> {
        let task = self.get(id)?;
        let (project, root) = self.project_root(&task.project_id)?;
        let worktree = match task.worktree.as_deref().filter(|w| Path::new(w).is_dir()) {
            Some(existing) => PathBuf::from(existing),
            None => {
                let path = self
                    .ports
                    .worktrees_dir
                    .join(&project)
                    .join(format!("task-{}", task.id));
                let exists = self.ports.git.branches(&root)?.contains(&task.branch);
                self.ports
                    .git
                    .add(&root, &path, &task.branch, Some(&task.base_branch), !exists)?;
                path
            }
        };
        let runtime = self
            .ports
            .runtimes
            .get(&task.agent)
            .ok_or_else(|| Error::Invalid(format!("no headless runner for {:?} yet", task.agent)))?
            .clone();
        let program = match task.agent {
            AgentKind::Claude => "claude",
            AgentKind::Codex => "codex",
            other => return Err(Error::Invalid(format!("{other:?} can't run tasks"))),
        };
        let program = self.ports.shell.find(program).ok_or_else(|| {
            Error::Invalid(format!("{program} isn't installed or isn't on your PATH"))
        })?;
        let prompt = task
            .next_prompt
            .clone()
            .unwrap_or_else(|| task.prompt.clone());
        let resume = task.next_prompt.as_ref().and(task.session_id.clone());

        let task = self.apply(
            id,
            TaskEvent::Start {
                worktree: worktree.to_string_lossy().into(),
            },
        )?;
        let run = Run {
            id: self.new_id(),
            task_id: task.id.clone(),
            prompt: prompt.clone(),
            started_at: self.now(),
            ended_at: None,
            outcome: None,
            cost_usd: None,
            input_tokens: None,
            output_tokens: None,
        };
        self.ports.tasks.put_run(&run)?;

        let weak: Weak<Inner> = Arc::downgrade(self);
        let (task_id, run_id) = (task.id.clone(), run.id.clone());
        let connection = runtime.start(
            AgentLaunch {
                program,
                args: match task.agent {
                    AgentKind::Claude => {
                        vec!["--append-system-prompt".into(), TASK_INSTRUCTIONS.into()]
                    }
                    _ => vec![],
                },
                cwd: worktree,
                env: self.ports.shell.env(),
                resume,
                model: None,
                permission_mode: task.permission,
            },
            Box::new(move |event| {
                if let Some(inner) = weak.upgrade() {
                    inner.on_event(&task_id, &run_id, event);
                }
            }),
        )?;
        connection.send(&prompt)?;
        self.live
            .lock()
            .unwrap()
            .insert(task.id.clone(), Live { run, connection });
        Ok(())
    }

    /// Ends a live run: records its outcome and stops its process.
    fn end_run(&self, task_id: &str, outcome: RunOutcome) -> Option<Run> {
        let live = self.live.lock().unwrap().remove(task_id)?;
        live.connection.kill();
        let mut run = live.run;
        run.ended_at = Some(self.now());
        run.outcome = Some(outcome);
        let _ = self.ports.tasks.put_run(&run);
        Some(run)
    }

    fn on_event(self: &Arc<Self>, task_id: &str, run_id: &str, event: RunEvent) {
        let _ = self.ports.log.append(run_id, &event);
        self.ports.bus.publish(
            RUN_EVENT,
            serde_json::to_value(RunEventPayload {
                task_id: task_id.into(),
                run_id: run_id.into(),
                event: event.clone(),
            })
            .unwrap_or_default(),
        );
        // Events of a run that was already ended (cancelled) change nothing.
        let current = self
            .live
            .lock()
            .unwrap()
            .get(task_id)
            .is_some_and(|l| l.run.id == run_id);
        if !current {
            return;
        }
        let result = match event {
            RunEvent::Session { session_id, .. } => self
                .apply(task_id, TaskEvent::Session { session_id })
                .map(|_| ()),
            RunEvent::Permission { .. } => self.apply(task_id, TaskEvent::NeedInput).map(|_| ()),
            RunEvent::PermissionResolved { .. } => match self.get(task_id) {
                Ok(t) if t.status == TaskStatus::NeedsInput => {
                    self.apply(task_id, TaskEvent::InputGiven).map(|_| ())
                }
                _ => Ok(()),
            },
            RunEvent::TurnComplete {
                is_error,
                result,
                cost_usd,
                input_tokens,
                output_tokens,
                ..
            } => {
                if let Some(live) = self.live.lock().unwrap().get_mut(task_id) {
                    live.run.cost_usd = cost_usd;
                    live.run.input_tokens = input_tokens;
                    live.run.output_tokens = output_tokens;
                }
                let finished = if is_error {
                    Err(Error::Invalid(
                        result
                            .clone()
                            .unwrap_or_else(|| "the agent reported an error".into()),
                    ))
                } else {
                    self.finish(task_id, result)
                };
                let ended = match finished {
                    Ok(()) => {
                        self.end_run(task_id, RunOutcome::Completed);
                        Ok(())
                    }
                    Err(error) => self.fail(task_id, error.to_string()),
                };
                // A slot is free either way.
                self.dispatch();
                ended
            }
            RunEvent::Exited { code } => {
                let error = format!("the agent exited before finishing (code {code:?})");
                let failed = self.fail(task_id, error);
                self.dispatch();
                failed
            }
            _ => Ok(()),
        };
        if let Err(e) = result {
            eprintln!("task {task_id}: {e}");
        }
    }

    /// The turn ended well: commit the work and hand it over for review.
    fn finish(&self, task_id: &str, summary: Option<String>) -> Result<()> {
        let task = self.get(task_id)?;
        if let Some(worktree) = &task.worktree {
            self.ports
                .task_git
                .commit_all(Path::new(worktree), &task.title)?;
        }
        self.apply(task_id, TaskEvent::Finished { summary })?;
        Ok(())
    }

    fn fail(&self, task_id: &str, error: String) -> Result<()> {
        self.end_run(
            task_id,
            RunOutcome::Failed {
                error: error.clone(),
            },
        );
        self.apply(task_id, TaskEvent::Failed { error })?;
        Ok(())
    }

    /// Removes a task's worktree and branch, as far as they exist.
    fn clean_up(&self, task: &Task, force: bool) -> Result<()> {
        let (_, root) = self.project_root(&task.project_id)?;
        if let Some(worktree) = task.worktree.as_deref().filter(|w| Path::new(w).is_dir()) {
            self.ports.git.remove(&root, Path::new(worktree), force)?;
        }
        if self.ports.git.branches(&root)?.contains(&task.branch) {
            self.ports.task_git.delete_branch(&root, &task.branch)?;
        }
        Ok(())
    }
}

fn radix36(mut n: u64) -> String {
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = Vec::new();
    loop {
        out.push(DIGITS[(n % 36) as usize]);
        n /= 36;
        if n == 0 {
            break;
        }
    }
    out.reverse();
    String::from_utf8(out).unwrap()
}

impl CommandHandler<CreateTask> for TaskCommands {
    fn handle(&self, cmd: CreateTask) -> Result<Task> {
        let inner = &self.inner;
        let title = cmd.title.trim();
        if title.is_empty() || cmd.prompt.trim().is_empty() {
            return Err(Error::Invalid("a task needs a title and a prompt".into()));
        }
        inner.project_root(&cmd.project_id)?;
        for dep in &cmd.depends_on {
            inner.get(dep)?;
        }
        let id = inner.new_id();
        let now = inner.now();
        let mut task = Task {
            branch: branch_name(title, &id),
            id,
            project_id: cmd.project_id,
            title: title.into(),
            prompt: cmd.prompt,
            agent: cmd.agent,
            base_branch: cmd.base_branch,
            worktree: None,
            status: TaskStatus::Backlog,
            depends_on: cmd.depends_on,
            permission: cmd.permission,
            session_id: None,
            next_prompt: None,
            summary: None,
            error: None,
            created_at: now,
            updated_at: now,
        };
        if cmd.queue {
            task = transition(&task, TaskEvent::Queue, now)
                .map_err(|e| Error::Invalid(e.to_string()))?;
        }
        inner.save(&task)?;
        Inner::dispatch(inner);
        inner.get(&task.id)
    }
}

impl CommandHandler<QueueTask> for TaskCommands {
    fn handle(&self, cmd: QueueTask) -> Result<Task> {
        self.inner.apply(&cmd.task_id, TaskEvent::Queue)?;
        Inner::dispatch(&self.inner);
        self.inner.get(&cmd.task_id)
    }
}

impl CommandHandler<RetryTask> for TaskCommands {
    fn handle(&self, cmd: RetryTask) -> Result<Task> {
        self.inner.apply(&cmd.task_id, TaskEvent::Retry)?;
        Inner::dispatch(&self.inner);
        self.inner.get(&cmd.task_id)
    }
}

impl CommandHandler<CancelTask> for TaskCommands {
    fn handle(&self, cmd: CancelTask) -> Result<Task> {
        let inner = &self.inner;
        // Check first, then stop the agent before recording the cancellation,
        // so its exit can't be mistaken for a failure.
        transition(&inner.get(&cmd.task_id)?, TaskEvent::Cancel, inner.now())
            .map_err(|e| Error::Invalid(e.to_string()))?;
        inner.end_run(&cmd.task_id, RunOutcome::Cancelled);
        let task = inner.apply(&cmd.task_id, TaskEvent::Cancel)?;
        Inner::dispatch(inner);
        Ok(task)
    }
}

impl CommandHandler<RespondToTask> for TaskCommands {
    fn handle(&self, cmd: RespondToTask) -> Result<()> {
        let live = self.inner.live.lock().unwrap();
        let run = live
            .get(&cmd.task_id)
            .ok_or_else(|| Error::Invalid(format!("task {} isn't running", cmd.task_id)))?;
        run.connection.respond(&cmd.request_id, &cmd.decision)
    }
}

impl CommandHandler<SendFollowUp> for TaskCommands {
    fn handle(&self, cmd: SendFollowUp) -> Result<Task> {
        if cmd.prompt.trim().is_empty() {
            return Err(Error::Invalid("the follow-up is empty".into()));
        }
        self.inner
            .apply(&cmd.task_id, TaskEvent::FollowUp { prompt: cmd.prompt })?;
        Inner::dispatch(&self.inner);
        self.inner.get(&cmd.task_id)
    }
}

impl CommandHandler<ApproveTask> for TaskCommands {
    fn handle(&self, cmd: ApproveTask) -> Result<Task> {
        let inner = &self.inner;
        let task = inner.get(&cmd.task_id)?;
        if task.status != TaskStatus::Review {
            return Err(Error::Invalid(format!(
                "only a task in review can be approved, this one is {:?}",
                task.status
            )));
        }
        let (_, root) = inner.project_root(&task.project_id)?;
        // Keep anything added by hand after the agent finished (take over).
        if let Some(worktree) = task.worktree.as_deref().filter(|w| Path::new(w).is_dir()) {
            inner
                .ports
                .task_git
                .commit_all(Path::new(worktree), &task.title)?;
        }
        inner.ports.task_git.merge(
            &root,
            &task.base_branch,
            &task.branch,
            &format!("Merge task: {}", task.title),
        )?;
        inner.clean_up(&task, false)?;
        let done = inner.apply(&task.id, TaskEvent::Approve)?;
        // Other work in review on the same base catches up with this merge,
        // where it can without conflicts; the rest shows as behind.
        for other in inner.ports.tasks.list()? {
            if other.status == TaskStatus::Review
                && other.base_branch == task.base_branch
                && let Some(worktree) = other.worktree.as_deref().filter(|w| Path::new(w).is_dir())
            {
                let _ = inner
                    .ports
                    .task_git
                    .update_from_base(Path::new(worktree), &other.base_branch);
            }
        }
        // Tasks waiting on this one can start now.
        Inner::dispatch(inner);
        Ok(done)
    }
}

impl CommandHandler<UpdateTaskBranch> for TaskCommands {
    fn handle(&self, cmd: UpdateTaskBranch) -> Result<()> {
        let task = self.inner.get(&cmd.task_id)?;
        if task.status.is_active() {
            return Err(Error::Invalid(
                "wait for the agent to stop before updating its branch".into(),
            ));
        }
        let worktree = task
            .worktree
            .as_deref()
            .filter(|w| Path::new(w).is_dir())
            .ok_or_else(|| Error::Invalid("the task has no worktree".into()))?;
        self.inner
            .ports
            .task_git
            .commit_all(Path::new(worktree), &task.title)?;
        self.inner
            .ports
            .task_git
            .update_from_base(Path::new(worktree), &task.base_branch)?;
        self.inner.save(&task)
    }
}

impl CommandHandler<DeleteTask> for TaskCommands {
    fn handle(&self, cmd: DeleteTask) -> Result<()> {
        let inner = &self.inner;
        let task = inner.get(&cmd.task_id)?;
        inner.end_run(&task.id, RunOutcome::Cancelled);
        inner.clean_up(&task, true)?;
        inner.ports.tasks.delete(&task.id)?;
        inner.ports.bus.publish(
            TASK_CHANGED,
            serde_json::json!({ "id": task.id, "deleted": true }),
        );
        Inner::dispatch(inner);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::cqrs::QueryHandler;
    use crate::application::ports::{GitWorktreeInfo, RunEventHandler};
    use crate::application::queries::tasks::{GetTask, TaskQueries};
    use crate::domain::workspace::Session;
    use crate::infrastructure::jsonl_run_log::JsonlRunLog;
    use crate::infrastructure::sqlite_tasks::SqliteTasks;

    #[derive(Default)]
    struct FakeRun {
        launch: Option<AgentLaunch>,
        handler: Option<Arc<RunEventHandler>>,
        sent: Vec<String>,
        responses: Vec<String>,
        killed: bool,
    }

    #[derive(Default)]
    struct FakeRuntime {
        runs: Arc<Mutex<Vec<FakeRun>>>,
    }

    struct FakeConnection {
        runs: Arc<Mutex<Vec<FakeRun>>>,
        index: usize,
    }

    impl AgentConnection for FakeConnection {
        fn send(&self, text: &str) -> Result<()> {
            self.runs.lock().unwrap()[self.index].sent.push(text.into());
            Ok(())
        }
        fn respond(&self, request_id: &str, _: &PermissionDecision) -> Result<()> {
            self.runs.lock().unwrap()[self.index]
                .responses
                .push(request_id.into());
            Ok(())
        }
        fn interrupt(&self) -> Result<()> {
            Ok(())
        }
        fn set_permission_mode(&self, _: PermissionMode) -> Result<()> {
            Ok(())
        }
        fn set_model(&self, _: &str) -> Result<()> {
            Ok(())
        }
        fn kill(&self) {
            self.runs.lock().unwrap()[self.index].killed = true;
        }
    }

    impl AgentRuntime for FakeRuntime {
        fn start(
            &self,
            launch: AgentLaunch,
            events: RunEventHandler,
        ) -> Result<Box<dyn AgentConnection>> {
            let mut runs = self.runs.lock().unwrap();
            runs.push(FakeRun {
                launch: Some(launch),
                handler: Some(Arc::new(events)),
                ..Default::default()
            });
            Ok(Box::new(FakeConnection {
                runs: self.runs.clone(),
                index: runs.len() - 1,
            }))
        }
    }

    /// Git that creates real (empty) directories for worktrees and records the rest.
    #[derive(Default)]
    struct FakeGit {
        branches: Mutex<Vec<String>>,
        calls: Mutex<Vec<String>>,
    }

    impl GitWorktrees for FakeGit {
        fn toplevel(&self, path: &Path) -> Result<PathBuf> {
            Ok(path.into())
        }
        fn list(&self, _: &Path) -> Result<Vec<GitWorktreeInfo>> {
            Ok(vec![])
        }
        fn branches(&self, _: &Path) -> Result<Vec<String>> {
            Ok(self.branches.lock().unwrap().clone())
        }
        fn add(
            &self,
            _: &Path,
            path: &Path,
            branch: &str,
            base: Option<&str>,
            create: bool,
        ) -> Result<()> {
            std::fs::create_dir_all(path)?;
            if create {
                self.branches.lock().unwrap().push(branch.into());
            }
            self.calls.lock().unwrap().push(format!(
                "add {branch} from {} create={create}",
                base.unwrap_or("-")
            ));
            Ok(())
        }
        fn remove(&self, _: &Path, path: &Path, force: bool) -> Result<()> {
            std::fs::remove_dir_all(path)?;
            self.calls
                .lock()
                .unwrap()
                .push(format!("remove force={force}"));
            Ok(())
        }
    }

    impl TaskGit for FakeGit {
        fn commit_all(&self, _: &Path, message: &str) -> Result<bool> {
            self.calls.lock().unwrap().push(format!("commit {message}"));
            Ok(true)
        }
        fn diff(&self, _: &Path, _: &str, _: &str) -> Result<String> {
            Ok(String::new())
        }
        fn changed_files(&self, _: &Path, _: &str, _: &str) -> Result<Vec<String>> {
            Ok(vec![])
        }
        fn conflicts(&self, _: &Path, _: &str, _: &str) -> Result<Vec<String>> {
            Ok(vec![])
        }
        fn merge(&self, _: &Path, base: &str, branch: &str, _: &str) -> Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("merge {branch} into {base}"));
            Ok(())
        }
        fn delete_branch(&self, _: &Path, branch: &str) -> Result<()> {
            self.branches.lock().unwrap().retain(|b| b != branch);
            self.calls.lock().unwrap().push(format!("delete {branch}"));
            Ok(())
        }
        fn uncommitted(&self, _: &Path) -> Result<Vec<String>> {
            Ok(vec![])
        }
        fn behind(&self, _: &Path, _: &str, _: &str) -> Result<usize> {
            Ok(0)
        }
        fn update_from_base(&self, worktree: &Path, base: &str) -> Result<()> {
            let task = worktree.file_name().unwrap().to_string_lossy();
            self.calls
                .lock()
                .unwrap()
                .push(format!("update {task} from {base}"));
            Ok(())
        }
    }

    struct FakeShell;
    impl ShellEnvironment for FakeShell {
        fn shell(&self) -> PathBuf {
            "/bin/sh".into()
        }
        fn find(&self, program: &str) -> Option<PathBuf> {
            Some(PathBuf::from("/usr/local/bin").join(program))
        }
        fn env(&self) -> Vec<(String, String)> {
            vec![]
        }
    }

    struct Sessions(Session);
    impl SessionRepository for Sessions {
        fn get(&self) -> Result<Session> {
            Ok(self.0.clone())
        }
        fn update(&self, f: &mut dyn FnMut(&mut Session) -> Result<()>) -> Result<()> {
            f(&mut self.0.clone())
        }
    }

    #[derive(Default)]
    struct Bus(Mutex<Vec<String>>);
    impl EventBus for Bus {
        fn publish(&self, topic: &str, _: serde_json::Value) {
            self.0.lock().unwrap().push(topic.into());
        }
    }

    struct Ticks(AtomicU64);
    impl Clock for Ticks {
        fn now(&self) -> u64 {
            self.0.fetch_add(1, Ordering::Relaxed)
        }
    }

    struct Harness {
        cmds: TaskCommands,
        queries: TaskQueries,
        runtime: Arc<Mutex<Vec<FakeRun>>>,
        git: Arc<FakeGit>,
        tasks: Arc<SqliteTasks>,
        _dirs: tempfile::TempDir,
    }

    impl Harness {
        fn new(concurrency: usize) -> Self {
            let dirs = tempfile::tempdir().unwrap();
            let mut session = Session::default();
            session.add_project(
                "p".into(),
                "proj".into(),
                dirs.path().join("repo").to_string_lossy().into(),
            );
            let sessions = Arc::new(Sessions(session));
            let runtime = FakeRuntime::default();
            let runs = runtime.runs.clone();
            let git = Arc::new(FakeGit::default());
            let tasks = Arc::new(SqliteTasks::in_memory().unwrap());
            let log = Arc::new(JsonlRunLog::new(dirs.path().join("runs")));
            let mut runtimes: HashMap<AgentKind, Arc<dyn AgentRuntime>> = HashMap::new();
            runtimes.insert(AgentKind::Claude, Arc::new(runtime));
            let cmds = TaskCommands::new(TaskPorts {
                tasks: tasks.clone(),
                log: log.clone(),
                runtimes,
                sessions: sessions.clone(),
                shell: Arc::new(FakeShell),
                git: git.clone(),
                task_git: git.clone(),
                bus: Arc::new(Bus::default()),
                clock: Arc::new(Ticks(AtomicU64::new(1))),
                worktrees_dir: dirs.path().join("worktrees"),
                concurrency,
            });
            let queries = TaskQueries::new(tasks.clone(), log, sessions, git.clone());
            Self {
                cmds,
                queries,
                runtime: runs,
                git,
                tasks,
                _dirs: dirs,
            }
        }

        fn create(&self, title: &str, depends_on: Vec<TaskId>) -> Task {
            self.cmds
                .handle(CreateTask {
                    project_id: "p".into(),
                    title: title.into(),
                    prompt: format!("please {title}"),
                    agent: AgentKind::Claude,
                    base_branch: "main".into(),
                    depends_on,
                    permission: PermissionMode::AcceptEdits,
                    queue: true,
                })
                .unwrap()
        }

        fn status(&self, id: &str) -> TaskStatus {
            self.tasks.get(id).unwrap().unwrap().status
        }

        /// Sends an event from the `n`th agent process started.
        fn emit(&self, n: usize, event: RunEvent) {
            let handler = self.runtime.lock().unwrap()[n].handler.clone().unwrap();
            handler(event);
        }

        fn started(&self) -> usize {
            self.runtime.lock().unwrap().len()
        }

        fn finish(&self, n: usize, result: &str) {
            self.emit(
                n,
                RunEvent::TurnComplete {
                    is_error: false,
                    result: Some(result.into()),
                    duration_ms: None,
                    cost_usd: Some(0.25),
                    input_tokens: Some(10.0),
                    output_tokens: Some(20.0),
                },
            );
        }
    }

    fn session(id: &str) -> RunEvent {
        RunEvent::Session {
            session_id: id.into(),
            model: None,
            permission_mode: None,
        }
    }

    #[test]
    fn a_queued_task_runs_in_its_own_worktree_and_lands_in_review() {
        let h = Harness::new(2);
        let task = h.create("Add login", vec![]);
        assert_eq!(task.status, TaskStatus::Running);
        let worktree = task.worktree.clone().unwrap();
        assert!(worktree.ends_with(&format!("worktrees/proj/task-{}", task.id)));
        {
            let runs = h.runtime.lock().unwrap();
            let launch = runs[0].launch.as_ref().unwrap();
            assert_eq!(launch.cwd, PathBuf::from(&worktree));
            assert_eq!(launch.program, PathBuf::from("/usr/local/bin/claude"));
            assert_eq!(launch.permission_mode, PermissionMode::AcceptEdits);
            assert_eq!(launch.args, ["--append-system-prompt", TASK_INSTRUCTIONS]);
            assert_eq!(runs[0].sent, ["please Add login"]);
        }

        h.emit(0, session("s1"));
        h.finish(0, "Added a login page");
        let detail = h
            .queries
            .handle(GetTask {
                task_id: task.id.clone(),
            })
            .unwrap();
        assert_eq!(detail.task.status, TaskStatus::Review);
        assert_eq!(detail.task.session_id.as_deref(), Some("s1"));
        assert_eq!(detail.task.summary.as_deref(), Some("Added a login page"));
        assert_eq!(detail.runs.len(), 1);
        assert_eq!(detail.runs[0].outcome, Some(RunOutcome::Completed));
        assert_eq!(detail.runs[0].cost_usd, Some(0.25));
        assert!(h.runtime.lock().unwrap()[0].killed);
        assert!(
            h.git
                .calls
                .lock()
                .unwrap()
                .contains(&"commit Add login".to_string())
        );
        // The log replays what happened.
        let log = h
            .queries
            .handle(crate::application::queries::tasks::GetRunLog {
                run_id: detail.runs[0].id.clone(),
            })
            .unwrap();
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn respects_the_concurrency_limit() {
        let h = Harness::new(1);
        let a = h.create("first", vec![]);
        let b = h.create("second", vec![]);
        assert_eq!(h.status(&a.id), TaskStatus::Running);
        assert_eq!(h.status(&b.id), TaskStatus::Queued);
        h.finish(0, "done");
        assert_eq!(h.status(&b.id), TaskStatus::Running);
        assert_eq!(h.started(), 2);
    }

    #[test]
    fn dependents_start_once_their_dependency_is_approved() {
        let h = Harness::new(3);
        let parent = h.create("parent", vec![]);
        let child = h.create("child", vec![parent.id.clone()]);
        assert_eq!(h.status(&child.id), TaskStatus::Queued);
        h.finish(0, "done");
        assert_eq!(h.status(&child.id), TaskStatus::Queued);

        let done = h
            .cmds
            .handle(ApproveTask {
                task_id: parent.id.clone(),
            })
            .unwrap();
        assert_eq!(done.status, TaskStatus::Done);
        let calls = h.git.calls.lock().unwrap().clone();
        assert!(calls.contains(&format!("merge {} into main", parent.branch)));
        assert!(calls.contains(&format!("delete {}", parent.branch)));
        assert!(!Path::new(parent.worktree.as_deref().unwrap()).exists());
        assert_eq!(h.status(&child.id), TaskStatus::Running);
    }

    #[test]
    fn approving_updates_other_reviewed_tasks_on_the_same_base() {
        let h = Harness::new(2);
        let a = h.create("a", vec![]);
        let b = h.create("b", vec![]);
        h.finish(0, "done");
        h.finish(1, "done");
        h.cmds
            .handle(ApproveTask {
                task_id: a.id.clone(),
            })
            .unwrap();
        let calls = h.git.calls.lock().unwrap().clone();
        assert!(
            calls.contains(&format!("update task-{} from main", b.id)),
            "{calls:?}"
        );
    }

    #[test]
    fn permission_prompts_block_until_answered() {
        let h = Harness::new(1);
        let task = h.create("risky", vec![]);
        h.emit(
            0,
            RunEvent::Permission {
                request_id: "r1".into(),
                tool_name: "Bash".into(),
                tool_use_id: None,
                input: serde_json::json!({ "command": "rm -rf dist" }),
                description: None,
            },
        );
        let detail = h
            .queries
            .handle(GetTask {
                task_id: task.id.clone(),
            })
            .unwrap();
        assert_eq!(detail.task.status, TaskStatus::NeedsInput);
        assert_eq!(detail.pending.unwrap().request_id, "r1");

        h.cmds
            .handle(RespondToTask {
                task_id: task.id.clone(),
                request_id: "r1".into(),
                decision: PermissionDecision {
                    allow: true,
                    remember: false,
                    updated_input: None,
                    message: None,
                },
            })
            .unwrap();
        assert_eq!(h.runtime.lock().unwrap()[0].responses, ["r1"]);
        h.emit(
            0,
            RunEvent::PermissionResolved {
                request_id: "r1".into(),
                allowed: true,
            },
        );
        assert_eq!(h.status(&task.id), TaskStatus::Running);
    }

    #[test]
    fn a_crashed_agent_fails_the_task_and_retry_reuses_the_worktree() {
        let h = Harness::new(1);
        let task = h.create("flaky", vec![]);
        h.emit(0, RunEvent::Exited { code: Some(1) });
        let failed = h.tasks.get(&task.id).unwrap().unwrap();
        assert_eq!(failed.status, TaskStatus::Failed);
        assert!(failed.error.unwrap().contains("exited before finishing"));

        let retried = h
            .cmds
            .handle(RetryTask {
                task_id: task.id.clone(),
            })
            .unwrap();
        assert_eq!(retried.status, TaskStatus::Running);
        assert_eq!(retried.worktree, task.worktree);
        let adds = h
            .git
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|c| c.starts_with("add"))
            .count();
        assert_eq!(adds, 1);
    }

    #[test]
    fn follow_ups_resume_the_agent_session() {
        let h = Harness::new(1);
        let task = h.create("feature", vec![]);
        h.emit(0, session("s1"));
        h.finish(0, "done");
        let next = h
            .cmds
            .handle(SendFollowUp {
                task_id: task.id.clone(),
                prompt: "add tests".into(),
            })
            .unwrap();
        assert_eq!(next.status, TaskStatus::Running);
        let runs = h.runtime.lock().unwrap();
        assert_eq!(
            runs[1].launch.as_ref().unwrap().resume.as_deref(),
            Some("s1")
        );
        assert_eq!(runs[1].sent, ["add tests"]);
    }

    #[test]
    fn cancelling_stops_the_agent_and_ignores_its_exit() {
        let h = Harness::new(1);
        let task = h.create("long", vec![]);
        let cancelled = h
            .cmds
            .handle(CancelTask {
                task_id: task.id.clone(),
            })
            .unwrap();
        assert_eq!(cancelled.status, TaskStatus::Cancelled);
        assert!(h.runtime.lock().unwrap()[0].killed);
        h.emit(0, RunEvent::Exited { code: None });
        assert_eq!(h.status(&task.id), TaskStatus::Cancelled);
        let runs = h.tasks.runs(&task.id).unwrap();
        assert_eq!(runs[0].outcome, Some(RunOutcome::Cancelled));
    }

    #[test]
    fn deleting_throws_away_the_worktree_and_branch() {
        let h = Harness::new(1);
        let task = h.create("scrap", vec![]);
        h.cmds
            .handle(DeleteTask {
                task_id: task.id.clone(),
            })
            .unwrap();
        assert!(h.tasks.get(&task.id).unwrap().is_none());
        assert!(!Path::new(task.worktree.as_deref().unwrap()).exists());
        assert!(h.git.branches.lock().unwrap().is_empty());
    }

    #[test]
    fn recovery_fails_tasks_left_running_by_a_previous_process() {
        let h = Harness::new(1);
        let task = h.create("orphan", vec![]);
        h.cmds.recover().unwrap();
        assert_eq!(h.status(&task.id), TaskStatus::Failed);
    }

    #[test]
    fn rejects_tasks_without_a_prompt_or_with_unknown_dependencies() {
        let h = Harness::new(1);
        let base = CreateTask {
            project_id: "p".into(),
            title: "t".into(),
            prompt: " ".into(),
            agent: AgentKind::Claude,
            base_branch: "main".into(),
            depends_on: vec![],
            permission: PermissionMode::Default,
            queue: false,
        };
        assert!(h.cmds.handle(base).is_err());
        let unknown = CreateTask {
            project_id: "p".into(),
            title: "t".into(),
            prompt: "p".into(),
            agent: AgentKind::Claude,
            base_branch: "main".into(),
            depends_on: vec!["ghost".into()],
            permission: PermissionMode::Default,
            queue: false,
        };
        assert!(h.cmds.handle(unknown).is_err());
    }
}

/// End to end against the real `claude` CLI and git:
/// `cargo test -p ensombl-core real_claude_task -- --ignored --nocapture`
#[cfg(test)]
mod smoke {
    use super::*;
    use crate::domain::workspace::Session;
    use crate::infrastructure::claude_headless::ClaudeCode;
    use crate::infrastructure::git_cli::GitCli;
    use crate::infrastructure::jsonl_run_log::JsonlRunLog;
    use crate::infrastructure::login_shell::LoginShell;
    use crate::infrastructure::sqlite_tasks::SqliteTasks;
    use crate::infrastructure::system_clock::SystemClock;
    use std::process::Command as Process;
    use std::time::{Duration, Instant};

    struct Sessions(Session);
    impl SessionRepository for Sessions {
        fn get(&self) -> Result<Session> {
            Ok(self.0.clone())
        }
        fn update(&self, f: &mut dyn FnMut(&mut Session) -> Result<()>) -> Result<()> {
            f(&mut self.0.clone())
        }
    }

    struct Print;
    impl EventBus for Print {
        fn publish(&self, topic: &str, payload: serde_json::Value) {
            if topic == TASK_CHANGED {
                println!("task -> {}", payload["status"]);
            } else if payload["event"]["type"] == "permission"
                || payload["event"]["type"] == "notice"
            {
                println!("{}", payload["event"]);
            }
        }
    }

    fn git(dir: &Path, args: &[&str]) {
        let ok = Process::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .status()
            .unwrap();
        assert!(ok.success(), "git {args:?}");
    }

    #[test]
    #[ignore]
    fn real_claude_task() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "-q", "-b", "main"]);
        std::fs::write(repo.join("README.md"), "# demo\n").unwrap();
        git(&repo, &["add", "."]);
        git(
            &repo,
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "commit",
                "-qm",
                "init",
            ],
        );

        let mut session = Session::default();
        session.add_project("p".into(), "demo".into(), repo.to_string_lossy().into());
        let tasks = Arc::new(SqliteTasks::in_memory().unwrap());
        let mut runtimes: HashMap<AgentKind, Arc<dyn AgentRuntime>> = HashMap::new();
        runtimes.insert(AgentKind::Claude, Arc::new(ClaudeCode));
        let cmds = TaskCommands::new(TaskPorts {
            tasks: tasks.clone(),
            log: Arc::new(JsonlRunLog::new(dir.path().join("runs"))),
            runtimes,
            sessions: Arc::new(Sessions(session)),
            shell: Arc::new(LoginShell::detect()),
            git: Arc::new(GitCli),
            task_git: Arc::new(GitCli),
            bus: Arc::new(Print),
            clock: Arc::new(SystemClock),
            worktrees_dir: dir.path().join("worktrees"),
            concurrency: 1,
        });
        let task = cmds
            .handle(CreateTask {
                project_id: "p".into(),
                title: "Add notes".into(),
                prompt: "Create a file notes.txt containing exactly the word hi, using the Write tool. Then reply done.".into(),
                agent: AgentKind::Claude,
                base_branch: "main".into(),
                depends_on: vec![],
                permission: PermissionMode::AcceptEdits,
                queue: true,
            })
            .unwrap();

        let deadline = Instant::now() + Duration::from_secs(240);
        let queries = crate::application::queries::tasks::TaskQueries::new(
            tasks.clone(),
            Arc::new(JsonlRunLog::new(dir.path().join("runs"))),
            Arc::new(Sessions(Session::default())),
            Arc::new(GitCli),
        );
        let finished = loop {
            let t = tasks.get(&task.id).unwrap().unwrap();
            if !t.status.is_active() {
                break t;
            }
            if t.status == TaskStatus::NeedsInput {
                use crate::application::cqrs::QueryHandler;
                let detail = queries
                    .handle(crate::application::queries::tasks::GetTask {
                        task_id: t.id.clone(),
                    })
                    .unwrap();
                if let Some(pending) = detail.pending {
                    println!("allowing {}", pending.tool_name);
                    cmds.handle(RespondToTask {
                        task_id: t.id.clone(),
                        request_id: pending.request_id,
                        decision: PermissionDecision {
                            allow: true,
                            remember: false,
                            updated_input: None,
                            message: None,
                        },
                    })
                    .unwrap();
                }
            }
            assert!(Instant::now() < deadline, "timed out in {:?}", t.status);
            std::thread::sleep(Duration::from_millis(500));
        };
        println!(
            "summary: {:?} error: {:?}",
            finished.summary, finished.error
        );
        assert_eq!(finished.status, TaskStatus::Review);
        let runs = tasks.runs(&task.id).unwrap();
        println!("cost: {:?}", runs[0].cost_usd);

        cmds.handle(ApproveTask {
            task_id: task.id.clone(),
        })
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(repo.join("notes.txt"))
                .unwrap()
                .trim(),
            "hi"
        );
        assert!(!Path::new(finished.worktree.as_deref().unwrap()).exists());
    }
}
