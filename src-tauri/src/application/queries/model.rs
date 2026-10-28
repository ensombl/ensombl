//! Read access to a worktree's architecture model.

use std::sync::Arc;

use serde::Deserialize;
use ts_rs::TS;

use crate::application::Result;
use crate::application::cqrs::{Query, QueryHandler};
use crate::application::index_registry::IndexRegistry;
use crate::domain::model::ArchModel;
use crate::domain::workspace::WorktreeId;

/// The linked model for a worktree; `None` until it has been indexed.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GetModel {
    pub worktree_id: WorktreeId,
}

impl Query for GetModel {
    type Output = Option<ArchModel>;
}

pub struct ModelQueries {
    registry: Arc<IndexRegistry>,
}

impl ModelQueries {
    pub fn new(registry: Arc<IndexRegistry>) -> Self {
        Self { registry }
    }
}

impl QueryHandler<GetModel> for ModelQueries {
    fn handle(&self, q: GetModel) -> Result<Option<ArchModel>> {
        Ok(self.registry.model(&q.worktree_id))
    }
}
