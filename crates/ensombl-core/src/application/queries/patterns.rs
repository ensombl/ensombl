//! Detected design patterns for a worktree, merged with the user's decisions.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::application::cqrs::{Query, QueryHandler};
use crate::application::index_registry::IndexRegistry;
use crate::application::ports::{PatternDecisions, SessionRepository};
use crate::application::{Error, Result};
pub use crate::domain::patterns::PatternDecision;
use crate::domain::patterns::{PatternInstance, detect};
use crate::domain::workspace::WorktreeId;

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GetPatterns {
    pub worktree_id: WorktreeId,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PatternView {
    pub pattern: PatternInstance,
    pub decision: Option<PatternDecision>,
}

impl Query for GetPatterns {
    type Output = Vec<PatternView>;
}

pub struct PatternQueries {
    sessions: Arc<dyn SessionRepository>,
    registry: Arc<IndexRegistry>,
    decisions: Arc<dyn PatternDecisions>,
}

impl PatternQueries {
    pub fn new(
        sessions: Arc<dyn SessionRepository>,
        registry: Arc<IndexRegistry>,
        decisions: Arc<dyn PatternDecisions>,
    ) -> Self {
        Self {
            sessions,
            registry,
            decisions,
        }
    }
}

/// Decisions are shared by every worktree of a project.
pub fn project_of(sessions: &dyn SessionRepository, worktree_id: &str) -> Result<String> {
    sessions
        .get()?
        .tabs
        .iter()
        .find(|t| t.id == worktree_id)
        .map(|t| t.project_id.clone())
        .ok_or_else(|| Error::Invalid(format!("worktree {worktree_id} isn't open")))
}

impl QueryHandler<GetPatterns> for PatternQueries {
    fn handle(&self, q: GetPatterns) -> Result<Vec<PatternView>> {
        let Some(model) = self.registry.model(&q.worktree_id) else {
            return Ok(vec![]);
        };
        let project = project_of(self.sessions.as_ref(), &q.worktree_id)?;
        let decisions = self.decisions.load(&project);
        let mut views: Vec<_> = detect(&model)
            .into_iter()
            .map(|pattern| PatternView {
                decision: decisions.get(&pattern.id).copied(),
                pattern,
            })
            .collect();
        // Confirmed first, dismissed last; confidence order within.
        views.sort_by_key(|v| match v.decision {
            Some(PatternDecision::Confirmed) => 0,
            None => 1,
            Some(PatternDecision::Dismissed) => 2,
        });
        Ok(views)
    }
}
