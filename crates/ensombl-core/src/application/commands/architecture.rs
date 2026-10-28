//! Commands that correct the detected architecture. Changes are written to the
//! worktree's `.crm/architecture.json`.

use std::sync::Arc;

use serde::Deserialize;
use ts_rs::TS;

use crate::application::cqrs::{Command, CommandHandler};
use crate::application::index_registry::IndexRegistry;
use crate::application::ports::ArchitectureStore;
use crate::application::{Error, Result};
use crate::domain::system::overrides::{ArchitectureFile, ComponentOverride};
use crate::domain::workspace::WorktreeId;

/// Replaces the user's override for one component (empty clears it).
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateComponent {
    pub worktree_id: WorktreeId,
    pub component_id: String,
    pub value: ComponentOverride,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MergeComponents {
    pub worktree_id: WorktreeId,
    pub component_ids: Vec<String>,
    pub name: String,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SplitComponent {
    pub worktree_id: WorktreeId,
    pub component_id: String,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SetRuleEnabled {
    pub worktree_id: WorktreeId,
    pub rule: String,
    pub enabled: bool,
}

impl Command for UpdateComponent {
    type Output = ();
}
impl Command for MergeComponents {
    /// The merged component's id.
    type Output = String;
}
impl Command for SplitComponent {
    type Output = ();
}
impl Command for SetRuleEnabled {
    type Output = ();
}

pub struct ArchitectureCommands {
    registry: Arc<IndexRegistry>,
    store: Arc<dyn ArchitectureStore>,
}

impl ArchitectureCommands {
    pub fn new(registry: Arc<IndexRegistry>, store: Arc<dyn ArchitectureStore>) -> Self {
        Self { registry, store }
    }

    fn edit<T>(&self, worktree: &str, f: impl FnOnce(&mut ArchitectureFile) -> T) -> Result<T> {
        let root = self
            .registry
            .root(worktree)
            .ok_or_else(|| Error::Invalid(format!("worktree {worktree} isn't indexed")))?;
        let mut file = self.store.load(&root);
        let out = f(&mut file);
        self.store.save(&root, &file)?;
        Ok(out)
    }
}

impl CommandHandler<UpdateComponent> for ArchitectureCommands {
    fn handle(&self, cmd: UpdateComponent) -> Result<()> {
        self.edit(&cmd.worktree_id, |f| {
            f.set_override(&cmd.component_id, cmd.value)
        })
    }
}

impl CommandHandler<MergeComponents> for ArchitectureCommands {
    fn handle(&self, cmd: MergeComponents) -> Result<String> {
        if cmd.component_ids.len() < 2 || cmd.name.trim().is_empty() {
            return Err(Error::Invalid(
                "pick at least two components and a name".into(),
            ));
        }
        self.edit(&cmd.worktree_id, |f| {
            f.merge(&cmd.component_ids, cmd.name.trim())
        })
    }
}

impl CommandHandler<SplitComponent> for ArchitectureCommands {
    fn handle(&self, cmd: SplitComponent) -> Result<()> {
        self.edit(&cmd.worktree_id, |f| f.split(&cmd.component_id))
    }
}

impl CommandHandler<SetRuleEnabled> for ArchitectureCommands {
    fn handle(&self, cmd: SetRuleEnabled) -> Result<()> {
        self.edit(&cmd.worktree_id, |f| {
            f.disabled_rules.retain(|r| r != &cmd.rule);
            if !cmd.enabled {
                f.disabled_rules.push(cmd.rule);
            }
        })
    }
}
