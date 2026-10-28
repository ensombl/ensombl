//! The system view of a worktree, with the user's corrections applied.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::application::Result;
use crate::application::cqrs::{Query, QueryHandler};
use crate::application::index_registry::IndexRegistry;
use crate::application::ports::ArchitectureStore;
use crate::domain::system::boundaries::{self, RULES, Violation};
use crate::domain::system::flow::{self, FlowStep};
use crate::domain::system::model::SystemModel;
use crate::domain::system::overrides::{self, ArchitectureFile};
use crate::domain::workspace::WorktreeId;

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GetSystem {
    pub worktree_id: WorktreeId,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RuleInfo {
    pub id: String,
    pub message: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SystemView {
    pub model: SystemModel,
    pub violations: Vec<Violation>,
    pub rules: Vec<RuleInfo>,
    /// The raw corrections, so edits can preserve fields they don't touch.
    pub overrides: ArchitectureFile,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GetFlow {
    pub worktree_id: WorktreeId,
    pub entry_id: String,
    pub depth: u32,
}

impl Query for GetSystem {
    type Output = Option<SystemView>;
}
impl Query for GetFlow {
    type Output = Vec<FlowStep>;
}

pub struct ArchitectureQueries {
    registry: Arc<IndexRegistry>,
    store: Arc<dyn ArchitectureStore>,
}

impl ArchitectureQueries {
    pub fn new(registry: Arc<IndexRegistry>, store: Arc<dyn ArchitectureStore>) -> Self {
        Self { registry, store }
    }

    fn applied(&self, worktree: &str) -> Option<(SystemModel, ArchitectureFile)> {
        let root = self.registry.root(worktree)?;
        let model = self.registry.system(worktree)?;
        let file = self.store.load(&root);
        Some((overrides::apply(&model, &file), file))
    }
}

impl QueryHandler<GetSystem> for ArchitectureQueries {
    fn handle(&self, q: GetSystem) -> Result<Option<SystemView>> {
        Ok(self.applied(&q.worktree_id).map(|(model, file)| {
            let disabled = &file.disabled_rules;
            let violations = boundaries::check(&model, disabled);
            let rules = RULES
                .iter()
                .map(|(id, message)| RuleInfo {
                    id: id.to_string(),
                    message: message.to_string(),
                    enabled: !disabled.iter().any(|d| d == id),
                })
                .collect();
            SystemView {
                model,
                violations,
                rules,
                overrides: file,
            }
        }))
    }
}

impl QueryHandler<GetFlow> for ArchitectureQueries {
    fn handle(&self, q: GetFlow) -> Result<Vec<FlowStep>> {
        Ok(self
            .applied(&q.worktree_id)
            .map(|(model, _)| flow::trace(&model, &q.entry_id, q.depth.clamp(1, 12)))
            .unwrap_or_default())
    }
}
