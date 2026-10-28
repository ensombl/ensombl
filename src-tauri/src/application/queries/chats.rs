//! Read models for native chats.

use std::sync::Arc;

use serde::Deserialize;
use ts_rs::TS;

use crate::application::Result;
use crate::application::commands::chats::ChatRegistry;
use crate::application::cqrs::{Query, QueryHandler};
use crate::domain::chat::ChatInfo;
use crate::domain::workspace::WorktreeId;

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ListChats {
    pub worktree_id: WorktreeId,
}

impl Query for ListChats {
    type Output = Vec<ChatInfo>;
}

pub struct ChatQueries {
    registry: Arc<ChatRegistry>,
}

impl ChatQueries {
    pub fn new(registry: Arc<ChatRegistry>) -> Self {
        Self { registry }
    }
}

impl QueryHandler<ListChats> for ChatQueries {
    fn handle(&self, q: ListChats) -> Result<Vec<ChatInfo>> {
        Ok(self.registry.list(&q.worktree_id))
    }
}
