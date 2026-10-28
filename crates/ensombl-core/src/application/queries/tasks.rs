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
