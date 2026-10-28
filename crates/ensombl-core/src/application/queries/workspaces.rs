//! Read models for projects, worktrees and tabs.

use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::application::commands::workspaces::to_worktree;
use crate::application::cqrs::{Query, QueryHandler};
use crate::application::ports::{GitWorktrees, SessionRepository};
use crate::application::{Error, Result};
use crate::domain::workspace::{Project, ProjectId, Worktree, WorktreeId};

/// The tab strip: one group per project, holding its open worktree tabs in order.
pub struct GetSession;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct SessionView {
    pub groups: Vec<TabGroupView>,
    pub active: Option<WorktreeId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct TabGroupView {
    pub project: Project,
    pub tabs: Vec<TabView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct TabView {
    pub worktree: Worktree,
    pub label: String,
}

/// Every worktree git knows about for a project, open or not.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ListWorktrees {
    pub project_id: ProjectId,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorktreeListing {
    pub worktree: Worktree,
    pub label: String,
    pub is_open: bool,
}

/// Local branches of a project, for picking a base when creating a worktree.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ListBranches {
    pub project_id: ProjectId,
}

impl Query for GetSession {
    type Output = SessionView;
}
impl Query for ListWorktrees {
    type Output = Vec<WorktreeListing>;
}
impl Query for ListBranches {
    type Output = Vec<String>;
}

pub struct WorkspaceQueries {
    git: Arc<dyn GitWorktrees>,
    sessions: Arc<dyn SessionRepository>,
}

impl WorkspaceQueries {
    pub fn new(git: Arc<dyn GitWorktrees>, sessions: Arc<dyn SessionRepository>) -> Self {
        Self { git, sessions }
    }

    fn root(&self, project_id: &str) -> Result<String> {
        self.sessions
            .get()?
            .project(project_id)
            .map(|p| p.root.clone())
            .ok_or_else(|| Error::Invalid(format!("unknown project {project_id}")))
    }
}

impl QueryHandler<GetSession> for WorkspaceQueries {
    fn handle(&self, _: GetSession) -> Result<SessionView> {
        let session = self.sessions.get()?;
        let groups = session
            .projects
            .iter()
            .map(|project| TabGroupView {
                project: project.clone(),
                tabs: session
                    .tabs
                    .iter()
                    .filter(|t| t.project_id == project.id)
                    .map(|t| TabView {
                        worktree: t.clone(),
                        label: t.label(),
                    })
                    .collect(),
            })
            .collect();
        Ok(SessionView {
            groups,
            active: session.active,
        })
    }
}

impl QueryHandler<ListWorktrees> for WorkspaceQueries {
    fn handle(&self, q: ListWorktrees) -> Result<Vec<WorktreeListing>> {
        let root = self.root(&q.project_id)?;
        let open: Vec<_> = self
            .sessions
            .get()?
            .tabs
            .into_iter()
            .map(|t| t.id)
            .collect();
        Ok(self
            .git
            .list(Path::new(&root))?
            .into_iter()
            .map(|info| {
                let worktree = to_worktree(&q.project_id, info);
                WorktreeListing {
                    is_open: open.contains(&worktree.id),
                    label: worktree.label(),
                    worktree,
                }
            })
            .collect())
    }
}

impl QueryHandler<ListBranches> for WorkspaceQueries {
    fn handle(&self, q: ListBranches) -> Result<Vec<String>> {
        let root = self.root(&q.project_id)?;
        self.git.branches(Path::new(&root))
    }
}
