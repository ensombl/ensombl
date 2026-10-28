//! Reading orchestrated tasks: the list, one task with its runs and what it
//! is waiting on, a run's full event log, and the work a task produced.

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::application::cqrs::{Query, QueryHandler};
use crate::application::ports::{RunLog, SessionRepository, TaskGit, TaskStore};
use crate::application::{Error, Result};
use crate::domain::agent_run::RunEvent;
use crate::domain::orchestration::{Run, Task, TaskId, TaskStatus};
use crate::domain::workspace::ProjectId;

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ListTasks {
    /// Only this project's tasks; all of them when absent.
    #[ts(optional)]
    pub project_id: Option<ProjectId>,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GetTask {
    pub task_id: TaskId,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GetRunLog {
    pub run_id: String,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GetTaskDiff {
    pub task_id: TaskId,
}

/// How a project's active tasks relate to each other and their base.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GetCoordination {
    pub project_id: ProjectId,
}

/// Files two tasks are both changing.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Overlap {
    pub task_id: TaskId,
    pub files: Vec<String>,
}

/// Where a running or reviewed task stands against the others.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TaskCoordination {
    pub task_id: TaskId,
    /// Commits on the base since the task branched (or last caught up).
    pub behind: u32,
    /// Files that would conflict merging into the base now.
    pub conflicts: Vec<String>,
    /// Other active tasks touching the same files.
    pub overlaps: Vec<Overlap>,
}

/// The prompt a blocked task waits on.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PendingInput {
    pub request_id: String,
    pub tool_name: String,
    #[ts(type = "unknown")]
    pub input: Value,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TaskDetail {
    pub task: Task,
    /// Oldest first.
    pub runs: Vec<Run>,
    pub pending: Option<PendingInput>,
}

impl Query for ListTasks {
    type Output = Vec<Task>;
}
impl Query for GetTask {
    type Output = TaskDetail;
}
impl Query for GetRunLog {
    type Output = Vec<RunEvent>;
}
impl Query for GetTaskDiff {
    type Output = String;
}
impl Query for GetCoordination {
    type Output = Vec<TaskCoordination>;
}

/// The last permission request in `events` that hasn't been resolved.
pub fn pending_input(events: &[RunEvent]) -> Option<PendingInput> {
    let mut pending: Option<PendingInput> = None;
    for event in events {
        match event {
            RunEvent::Permission {
                request_id,
                tool_name,
                input,
                description,
                ..
            } => {
                pending = Some(PendingInput {
                    request_id: request_id.clone(),
                    tool_name: tool_name.clone(),
                    input: input.clone(),
                    description: description.clone(),
                });
            }
            RunEvent::PermissionResolved { request_id, .. }
                if pending
                    .as_ref()
                    .is_some_and(|p| &p.request_id == request_id) =>
            {
                pending = None;
            }
            _ => {}
        }
    }
    pending
}

pub struct TaskQueries {
    tasks: Arc<dyn TaskStore>,
    log: Arc<dyn RunLog>,
    sessions: Arc<dyn SessionRepository>,
    git: Arc<dyn TaskGit>,
}

impl TaskQueries {
    pub fn new(
        tasks: Arc<dyn TaskStore>,
        log: Arc<dyn RunLog>,
        sessions: Arc<dyn SessionRepository>,
        git: Arc<dyn TaskGit>,
    ) -> Self {
        Self {
            tasks,
            log,
            sessions,
            git,
        }
    }

    fn task(&self, id: &str) -> Result<Task> {
        self.tasks
            .get(id)?
            .ok_or_else(|| Error::Invalid(format!("no task {id}")))
    }
}

impl TaskQueries {
    fn project_root(&self, project_id: &str) -> Result<PathBuf> {
        self.sessions
            .get()?
            .project(project_id)
            .map(|p| PathBuf::from(&p.root))
            .ok_or_else(|| Error::Invalid(format!("no project {project_id}")))
    }
}

/// Pairs up tasks that touch the same files.
pub fn overlaps(changed: &[(TaskId, Vec<String>)]) -> Vec<(TaskId, Vec<Overlap>)> {
    changed
        .iter()
        .map(|(id, files)| {
            let found = changed
                .iter()
                .filter(|(other, _)| other != id)
                .filter_map(|(other, theirs)| {
                    let mut shared: Vec<String> = files
                        .iter()
                        .filter(|f| theirs.contains(f))
                        .cloned()
                        .collect();
                    shared.sort();
                    shared.dedup();
                    (!shared.is_empty()).then(|| Overlap {
                        task_id: other.clone(),
                        files: shared,
                    })
                })
                .collect();
            (id.clone(), found)
        })
        .collect()
}

impl QueryHandler<GetCoordination> for TaskQueries {
    fn handle(&self, q: GetCoordination) -> Result<Vec<TaskCoordination>> {
        let root = self.project_root(&q.project_id)?;
        let active: Vec<Task> = self
            .tasks
            .list()?
            .into_iter()
            .filter(|t| t.project_id == q.project_id)
            .filter(|t| t.status.is_active() || t.status == TaskStatus::Review)
            .collect();
        let branches = active.iter().map(|t| {
            let mut files = self
                .git
                .changed_files(&root, &t.base_branch, &t.branch)
                .unwrap_or_default();
            if let Some(w) = t
                .worktree
                .as_deref()
                .filter(|w| std::path::Path::new(w).is_dir())
            {
                files.extend(
                    self.git
                        .uncommitted(std::path::Path::new(w))
                        .unwrap_or_default(),
                );
            }
            (t.id.clone(), files)
        });
        let changed: Vec<_> = branches.collect();
        Ok(overlaps(&changed)
            .into_iter()
            .zip(&active)
            .map(|((task_id, overlaps), t)| TaskCoordination {
                task_id,
                behind: self
                    .git
                    .behind(&root, &t.base_branch, &t.branch)
                    .unwrap_or(0) as u32,
                conflicts: self
                    .git
                    .conflicts(&root, &t.base_branch, &t.branch)
                    .unwrap_or_default(),
                overlaps,
            })
            .collect())
    }
}

impl QueryHandler<ListTasks> for TaskQueries {
    fn handle(&self, q: ListTasks) -> Result<Vec<Task>> {
        let mut tasks = self.tasks.list()?;
        if let Some(project) = q.project_id {
            tasks.retain(|t| t.project_id == project);
        }
        Ok(tasks)
    }
}

impl QueryHandler<GetTask> for TaskQueries {
    fn handle(&self, q: GetTask) -> Result<TaskDetail> {
        let task = self.task(&q.task_id)?;
        let runs = self.tasks.runs(&task.id)?;
        let pending = match (task.status, runs.last()) {
            (TaskStatus::NeedsInput, Some(run)) => pending_input(&self.log.read(&run.id)?),
            _ => None,
        };
        Ok(TaskDetail {
            task,
            runs,
            pending,
        })
    }
}

impl QueryHandler<GetRunLog> for TaskQueries {
    fn handle(&self, q: GetRunLog) -> Result<Vec<RunEvent>> {
        self.log.read(&q.run_id)
    }
}

impl QueryHandler<GetTaskDiff> for TaskQueries {
    fn handle(&self, q: GetTaskDiff) -> Result<String> {
        let task = self.task(&q.task_id)?;
        let root = self
            .sessions
            .get()?
            .project(&task.project_id)
            .map(|p| PathBuf::from(&p.root))
            .ok_or_else(|| Error::Invalid(format!("no project {}", task.project_id)))?;
        self.git.diff(&root, &task.base_branch, &task.branch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn permission(id: &str) -> RunEvent {
        RunEvent::Permission {
            request_id: id.into(),
            tool_name: "Bash".into(),
            tool_use_id: None,
            input: json!({ "command": "rm -rf build" }),
            description: None,
        }
    }

    #[test]
    fn finds_tasks_touching_the_same_files() {
        let found = overlaps(&[
            ("a".into(), vec!["x.ts".into(), "y.ts".into()]),
            ("b".into(), vec!["y.ts".into(), "y.ts".into()]),
            ("c".into(), vec!["z.ts".into()]),
        ]);
        assert_eq!(
            found[0].1,
            [Overlap {
                task_id: "b".into(),
                files: vec!["y.ts".into()]
            }]
        );
        assert_eq!(found[1].1[0].task_id, "a");
        assert!(found[2].1.is_empty());
    }

    #[test]
    fn finds_the_unanswered_permission() {
        assert_eq!(pending_input(&[]), None);
        let events = [permission("r1")];
        assert_eq!(pending_input(&events).unwrap().request_id, "r1");
        let answered = [
            permission("r1"),
            RunEvent::PermissionResolved {
                request_id: "r1".into(),
                allowed: true,
            },
        ];
        assert_eq!(pending_input(&answered), None);
        let second = [
            permission("r1"),
            RunEvent::PermissionResolved {
                request_id: "r1".into(),
                allowed: true,
            },
            permission("r2"),
        ];
        assert_eq!(pending_input(&second).unwrap().tool_name, "Bash");
    }
}
