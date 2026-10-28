//! The worktree's files, for the file explorer.

use std::sync::Arc;

use serde::Deserialize;
use ts_rs::TS;

use crate::application::Result;
use crate::application::cqrs::{Query, QueryHandler};
use crate::application::ports::{FileLister, SessionRepository};
use crate::application::worktrees::worktree_root;
use crate::domain::workspace::WorktreeId;

/// Enough for big monorepos without shipping an unbounded list over IPC.
const MAX_FILES: usize = 50_000;

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ListFiles {
    pub worktree_id: WorktreeId,
}

impl Query for ListFiles {
    type Output = Vec<String>;
}

pub struct FileQueries {
    sessions: Arc<dyn SessionRepository>,
    lister: Arc<dyn FileLister>,
}

impl FileQueries {
    pub fn new(sessions: Arc<dyn SessionRepository>, lister: Arc<dyn FileLister>) -> Self {
        Self { sessions, lister }
    }
}

impl QueryHandler<ListFiles> for FileQueries {
    fn handle(&self, q: ListFiles) -> Result<Vec<String>> {
        let root = worktree_root(self.sessions.as_ref(), &q.worktree_id)?;
        self.lister.list(&root, MAX_FILES)
    }
}
