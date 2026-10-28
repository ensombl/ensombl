use std::sync::Arc;

use crate::application::commands::workspaces::WorkspaceCommands;
use crate::application::queries::get_app_info::GetAppInfoHandler;
use crate::application::queries::workspaces::WorkspaceQueries;

/// Handlers wired by the composition root and managed by Tauri.
/// `Arc`s so blocking work can move onto worker threads.
pub struct Handlers {
    pub get_app_info: GetAppInfoHandler,
    pub workspace_commands: Arc<WorkspaceCommands>,
    pub workspace_queries: Arc<WorkspaceQueries>,
}
