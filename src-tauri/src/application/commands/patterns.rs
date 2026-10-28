//! Confirming or dismissing detected patterns.

use std::sync::Arc;

use serde::Deserialize;
use ts_rs::TS;

use crate::application::Result;
use crate::application::cqrs::{Command, CommandHandler};
use crate::application::ports::{PatternDecisions, SessionRepository};
use crate::application::queries::patterns::project_of;
use crate::domain::patterns::PatternDecision;
use crate::domain::workspace::WorktreeId;

/// Records (or with `decision: null`, clears) the user's verdict on a pattern.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SetPatternDecision {
    pub worktree_id: WorktreeId,
    pub pattern_id: String,
    pub decision: Option<PatternDecision>,
}

impl Command for SetPatternDecision {
    type Output = ();
}

pub struct PatternCommands {
    sessions: Arc<dyn SessionRepository>,
    decisions: Arc<dyn PatternDecisions>,
}

impl PatternCommands {
    pub fn new(sessions: Arc<dyn SessionRepository>, decisions: Arc<dyn PatternDecisions>) -> Self {
        Self {
            sessions,
            decisions,
        }
    }
}

impl CommandHandler<SetPatternDecision> for PatternCommands {
    fn handle(&self, cmd: SetPatternDecision) -> Result<()> {
        let project = project_of(self.sessions.as_ref(), &cmd.worktree_id)?;
        self.decisions.set(&project, &cmd.pattern_id, cmd.decision)
    }
}
