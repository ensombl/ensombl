//! Orchestrated work: a *task* is a prompt an agent works on unattended in
//! its own worktree and branch; each attempt at it is a *run*. Pure state and
//! rules with no IO: the status machine and the scheduler.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::agent_run::PermissionMode;
use super::terminal::AgentKind;
use super::workspace::ProjectId;

pub type TaskId = String;
pub type RunId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TaskStatus {
    /// Written down, not yet asked to run.
    Backlog,
    /// Waiting for a free slot and for its dependencies.
    Queued,
    Running,
    /// The agent is blocked on a permission or a question.
    NeedsInput,
    /// The agent finished; its branch waits for approval.
    Review,
    /// Approved and merged.
    Done,
    Failed,
    Cancelled,
}

impl TaskStatus {
    /// Occupies one of the concurrent slots.
    pub fn is_active(self) -> bool {
        matches!(self, Self::Running | Self::NeedsInput)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Task {
    pub id: TaskId,
    pub project_id: ProjectId,
    pub title: String,
    pub prompt: String,
    pub agent: AgentKind,
    /// Branch the work starts from and merges back into.
    pub base_branch: String,
    /// The task's own branch.
    pub branch: String,
    /// Path of the task's worktree, once created.
    pub worktree: Option<String>,
    pub status: TaskStatus,
    /// Tasks that must be done before this one starts.
    pub depends_on: Vec<TaskId>,
    pub permission: PermissionMode,
    /// The agent's session, so follow-ups and take-overs continue it.
    pub session_id: Option<String>,
    /// What the next run sends instead of `prompt` (a follow-up).
    pub next_prompt: Option<String>,
    /// The agent's final message from the last run.
    pub summary: Option<String>,
    pub error: Option<String>,
    #[ts(type = "number")]
    pub created_at: u64,
    #[ts(type = "number")]
    pub updated_at: u64,
}

/// Something that happens to a task.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskEvent {
    Queue,
    Start {
        worktree: String,
    },
    Session {
        session_id: String,
    },
    NeedInput,
    InputGiven,
    Finished {
        summary: Option<String>,
    },
    Failed {
        error: String,
    },
    Cancel,
    Retry,
    /// Another turn on a reviewed task, in the same session.
    FollowUp {
        prompt: String,
    },
    Approve,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("a {from:?} task can't {event}")]
pub struct InvalidTransition {
    pub from: TaskStatus,
    pub event: &'static str,
}

impl TaskEvent {
    fn name(&self) -> &'static str {
        match self {
            Self::Queue => "be queued",
            Self::Start { .. } => "start",
            Self::Session { .. } => "record a session",
            Self::NeedInput => "wait for input",
            Self::InputGiven => "take input",
            Self::Finished { .. } => "finish",
            Self::Failed { .. } => "fail",
            Self::Cancel => "be cancelled",
            Self::Retry => "be retried",
            Self::FollowUp { .. } => "take a follow-up",
            Self::Approve => "be approved",
        }
    }
}

/// Applies `event` to `task` at time `now`, or explains why it can't happen.
pub fn transition(task: &Task, event: TaskEvent, now: u64) -> Result<Task, InvalidTransition> {
    use TaskStatus::*;
    let invalid = |event: &TaskEvent| InvalidTransition {
        from: task.status,
        event: event.name(),
    };
    let mut next = task.clone();
    next.updated_at = now;
    match (task.status, &event) {
        (Backlog, TaskEvent::Queue) => next.status = Queued,
        (Queued, TaskEvent::Start { worktree }) => {
            next.status = Running;
            next.worktree = Some(worktree.clone());
            next.error = None;
        }
        (Running | NeedsInput, TaskEvent::Session { session_id }) => {
            next.session_id = Some(session_id.clone());
        }
        (Running, TaskEvent::NeedInput) => next.status = NeedsInput,
        (NeedsInput, TaskEvent::InputGiven) => next.status = Running,
        (Running | NeedsInput, TaskEvent::Finished { summary }) => {
            next.status = Review;
            next.summary = summary.clone();
            next.next_prompt = None;
        }
        // Queued tasks fail when they can't even start (no worktree, no CLI).
        (Queued | Running | NeedsInput, TaskEvent::Failed { error }) => {
            next.status = Failed;
            next.error = Some(error.clone());
        }
        (Backlog | Queued | Running | NeedsInput | Review, TaskEvent::Cancel) => {
            next.status = Cancelled;
        }
        (Failed | Cancelled, TaskEvent::Retry) => next.status = Queued,
        (Review, TaskEvent::FollowUp { prompt }) => {
            next.status = Queued;
            next.next_prompt = Some(prompt.clone());
        }
        (Review, TaskEvent::Approve) => next.status = Done,
        _ => return Err(invalid(&event)),
    }
    Ok(next)
}

/// Queued tasks to start now, oldest first: their dependencies are done and
/// at most `limit` tasks are active at once.
pub fn schedule(tasks: &[Task], limit: usize) -> Vec<TaskId> {
    let active = tasks.iter().filter(|t| t.status.is_active()).count();
    let done = |id: &TaskId| {
        tasks
            .iter()
            .any(|t| &t.id == id && t.status == TaskStatus::Done)
    };
    let mut ready: Vec<&Task> = tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Queued && t.depends_on.iter().all(done))
        .collect();
    ready.sort_by_key(|t| t.created_at);
    ready
        .into_iter()
        .take(limit.saturating_sub(active))
        .map(|t| t.id.clone())
        .collect()
}

/// A branch name for a task: `ensombl/<slug>-<id>`.
pub fn branch_name(title: &str, id: &str) -> String {
    let mut slug = String::new();
    for c in title.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
        if slug.len() >= 40 {
            break;
        }
    }
    let slug = slug.trim_end_matches('-');
    if slug.is_empty() {
        format!("ensombl/task-{id}")
    } else {
        format!("ensombl/{slug}-{id}")
    }
}

/// The prompt a run starts with. Tasks that build on others hear what those
/// did: in place of `{{deps}}` when the prompt asks for it, otherwise first.
pub fn compose_prompt(prompt: &str, base: &str, deps: &[(String, Option<String>)]) -> String {
    if deps.is_empty() {
        return prompt.replace("{{deps}}", "");
    }
    let list = deps
        .iter()
        .map(|(title, summary)| {
            format!(
                "- {title}: {}",
                summary.as_deref().unwrap_or("(no summary)").trim()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    if prompt.contains("{{deps}}") {
        return prompt.replace("{{deps}}", &list);
    }
    format!(
        "This builds on earlier tasks, already merged into {base}:\n{list}\n\nYour task:\n{prompt}"
    )
}

/// Task worktrees aren't tabs; they're addressed as `task:<id>`.
pub fn task_worktree_id(task_id: &str) -> String {
    format!("task:{task_id}")
}

/// The task a `task:<id>` worktree id belongs to.
pub fn task_of_worktree(worktree_id: &str) -> Option<&str> {
    worktree_id.strip_prefix("task:")
}

/// How a run ended.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", tag = "type")]
#[ts(export)]
pub enum RunOutcome {
    Completed,
    Failed { error: String },
    Cancelled,
}

/// One attempt at a task: one agent process and its event log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Run {
    pub id: RunId,
    pub task_id: TaskId,
    pub prompt: String,
    #[ts(type = "number")]
    pub started_at: u64,
    #[ts(type = "number | null")]
    pub ended_at: Option<u64>,
    pub outcome: Option<RunOutcome>,
    pub cost_usd: Option<f64>,
    pub input_tokens: Option<f64>,
    pub output_tokens: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: &str, status: TaskStatus, created_at: u64) -> Task {
        Task {
            id: id.into(),
            project_id: "p".into(),
            title: "Add a thing".into(),
            prompt: "do it".into(),
            agent: AgentKind::Claude,
            base_branch: "main".into(),
            branch: branch_name("Add a thing", id),
            worktree: None,
            status,
            depends_on: vec![],
            permission: PermissionMode::AcceptEdits,
            session_id: None,
            next_prompt: None,
            summary: None,
            error: None,
            created_at,
            updated_at: created_at,
        }
    }

    fn apply(t: &Task, events: Vec<TaskEvent>) -> Result<Task, InvalidTransition> {
        events
            .into_iter()
            .try_fold(t.clone(), |t, e| transition(&t, e, 9))
    }

    #[test]
    fn a_task_runs_to_review_and_is_approved() {
        let t = apply(
            &task("a", TaskStatus::Backlog, 0),
            vec![
                TaskEvent::Queue,
                TaskEvent::Start {
                    worktree: "/w/a".into(),
                },
                TaskEvent::Session {
                    session_id: "s1".into(),
                },
                TaskEvent::NeedInput,
                TaskEvent::InputGiven,
                TaskEvent::Finished {
                    summary: Some("done".into()),
                },
            ],
        )
        .unwrap();
        assert_eq!(t.status, TaskStatus::Review);
        assert_eq!(t.worktree.as_deref(), Some("/w/a"));
        assert_eq!(t.session_id.as_deref(), Some("s1"));
        assert_eq!(t.summary.as_deref(), Some("done"));
        assert_eq!(t.updated_at, 9);
        assert_eq!(
            transition(&t, TaskEvent::Approve, 10).unwrap().status,
            TaskStatus::Done
        );
    }

    #[test]
    fn follow_ups_requeue_with_their_own_prompt() {
        let t = task("a", TaskStatus::Review, 0);
        let t = transition(
            &t,
            TaskEvent::FollowUp {
                prompt: "also test it".into(),
            },
            1,
        )
        .unwrap();
        assert_eq!(t.status, TaskStatus::Queued);
        assert_eq!(t.next_prompt.as_deref(), Some("also test it"));
        let t = apply(
            &t,
            vec![
                TaskEvent::Start {
                    worktree: "/w/a".into(),
                },
                TaskEvent::Finished { summary: None },
            ],
        )
        .unwrap();
        assert_eq!(t.next_prompt, None);
    }

    #[test]
    fn failures_and_cancellations_can_be_retried() {
        let failed = apply(
            &task("a", TaskStatus::Queued, 0),
            vec![
                TaskEvent::Start {
                    worktree: "/w".into(),
                },
                TaskEvent::Failed {
                    error: "boom".into(),
                },
            ],
        )
        .unwrap();
        assert_eq!(failed.error.as_deref(), Some("boom"));
        let retried = transition(&failed, TaskEvent::Retry, 1).unwrap();
        assert_eq!(retried.status, TaskStatus::Queued);
        let restarted = transition(
            &retried,
            TaskEvent::Start {
                worktree: "/w".into(),
            },
            2,
        )
        .unwrap();
        assert_eq!(restarted.error, None);
    }

    #[test]
    fn queued_tasks_can_fail_to_start() {
        let t = transition(
            &task("a", TaskStatus::Queued, 0),
            TaskEvent::Failed {
                error: "claude isn't installed".into(),
            },
            1,
        )
        .unwrap();
        assert_eq!(t.status, TaskStatus::Failed);
    }

    #[test]
    fn rejects_impossible_transitions() {
        let done = task("a", TaskStatus::Done, 0);
        for event in [
            TaskEvent::Queue,
            TaskEvent::Cancel,
            TaskEvent::Approve,
            TaskEvent::Retry,
        ] {
            assert!(transition(&done, event, 1).is_err());
        }
        let err =
            transition(&task("a", TaskStatus::Backlog, 0), TaskEvent::Approve, 1).unwrap_err();
        assert_eq!(err.to_string(), "a Backlog task can't be approved");
    }

    #[test]
    fn schedules_oldest_ready_tasks_within_the_limit() {
        let mut blocked = task("blocked", TaskStatus::Queued, 0);
        blocked.depends_on = vec!["dep".into()];
        let tasks = vec![
            task("running", TaskStatus::Running, 0),
            blocked,
            task("dep", TaskStatus::Review, 0),
            task("newer", TaskStatus::Queued, 5),
            task("older", TaskStatus::Queued, 3),
            task("backlog", TaskStatus::Backlog, 1),
        ];
        assert_eq!(schedule(&tasks, 3), ["older", "newer"]);
        assert_eq!(schedule(&tasks, 2), ["older"]);
        assert!(schedule(&tasks, 1).is_empty());
    }

    #[test]
    fn dependencies_unblock_once_done() {
        let mut child = task("child", TaskStatus::Queued, 0);
        child.depends_on = vec!["parent".into()];
        let parent = task("parent", TaskStatus::Done, 0);
        assert_eq!(schedule(&[child.clone(), parent], 1), ["child"]);
        // A dependency that doesn't exist never unblocks.
        assert!(schedule(&[child], 1).is_empty());
    }

    #[test]
    fn prompts_carry_what_dependencies_did() {
        assert_eq!(compose_prompt("do it", "main", &[]), "do it");
        let deps = [("Add API".to_string(), Some("Added /claims".to_string()))];
        assert_eq!(
            compose_prompt("do it", "main", &deps),
            "This builds on earlier tasks, already merged into main:\n- Add API: Added /claims\n\nYour task:\ndo it"
        );
        assert_eq!(
            compose_prompt("Before: {{deps}}. Now go.", "main", &deps),
            "Before: - Add API: Added /claims. Now go."
        );
    }

    #[test]
    fn task_worktree_ids_round_trip() {
        assert_eq!(task_of_worktree(&task_worktree_id("k3")), Some("k3"));
        assert_eq!(task_of_worktree("wt-1"), None);
    }

    #[test]
    fn branch_names_are_slugged() {
        assert_eq!(
            branch_name("Fix: the login page!", "k3"),
            "ensombl/fix-the-login-page-k3"
        );
        assert_eq!(branch_name("???", "k3"), "ensombl/task-k3");
    }
}
