//! Read models for agents and running terminals.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::application::Result;
use crate::application::commands::terminals::TerminalRegistry;
use crate::application::cqrs::{Query, QueryHandler};
use crate::application::ports::ShellEnvironment;
use crate::domain::terminal::{AgentProfile, TerminalInfo};
use crate::domain::workspace::WorktreeId;

/// Agent profiles and whether their CLI is installed.
pub struct ListAgents;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AgentAvailability {
    pub profile: AgentProfile,
    pub available: bool,
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ListTerminals {
    pub worktree_id: WorktreeId,
}

impl Query for ListAgents {
    type Output = Vec<AgentAvailability>;
}
impl Query for ListTerminals {
    type Output = Vec<TerminalInfo>;
}

pub struct TerminalQueries {
    shell: Arc<dyn ShellEnvironment>,
    registry: Arc<TerminalRegistry>,
    profiles: Vec<AgentProfile>,
}

impl TerminalQueries {
    pub fn new(
        shell: Arc<dyn ShellEnvironment>,
        registry: Arc<TerminalRegistry>,
        profiles: Vec<AgentProfile>,
    ) -> Self {
        Self {
            shell,
            registry,
            profiles,
        }
    }
}

impl QueryHandler<ListAgents> for TerminalQueries {
    fn handle(&self, _: ListAgents) -> Result<Vec<AgentAvailability>> {
        Ok(self
            .profiles
            .iter()
            .map(|profile| {
                let path = if profile.command == "$SHELL" {
                    Some(self.shell.shell())
                } else {
                    self.shell.find(&profile.command)
                };
                AgentAvailability {
                    profile: profile.clone(),
                    available: path.is_some(),
                    path: path.map(|p| p.to_string_lossy().into_owned()),
                }
            })
            .collect())
    }
}

impl QueryHandler<ListTerminals> for TerminalQueries {
    fn handle(&self, q: ListTerminals) -> Result<Vec<TerminalInfo>> {
        Ok(self.registry.list(&q.worktree_id))
    }
}
