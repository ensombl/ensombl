use std::sync::Arc;

use crate::application::commands::architecture::ArchitectureCommands;
use crate::application::commands::chats::ChatCommands;
use crate::application::commands::indexing::IndexingCommands;
use crate::application::commands::patterns::PatternCommands;
use crate::application::commands::review::ReviewCommands;
use crate::application::commands::terminals::TerminalCommands;
use crate::application::commands::workspaces::WorkspaceCommands;
use crate::application::queries::architecture::ArchitectureQueries;
use crate::application::queries::chats::ChatQueries;
use crate::application::queries::get_app_info::GetAppInfoHandler;
use crate::application::queries::model::ModelQueries;
use crate::application::queries::patterns::PatternQueries;
use crate::application::queries::review::ReviewQueries;
use crate::application::queries::terminals::TerminalQueries;
use crate::application::queries::workspaces::WorkspaceQueries;

/// Handlers wired by the composition root and managed by Tauri.
/// `Arc`s so blocking work can move onto worker threads.
pub struct Handlers {
    pub get_app_info: GetAppInfoHandler,
    pub workspace_commands: Arc<WorkspaceCommands>,
    pub workspace_queries: Arc<WorkspaceQueries>,
    pub indexing_commands: Arc<IndexingCommands>,
    pub model_queries: Arc<ModelQueries>,
    pub terminal_commands: Arc<TerminalCommands>,
    pub terminal_queries: Arc<TerminalQueries>,
    pub architecture_commands: Arc<ArchitectureCommands>,
    pub architecture_queries: Arc<ArchitectureQueries>,
    pub chat_commands: Arc<ChatCommands>,
    pub chat_queries: Arc<ChatQueries>,
    pub pattern_commands: Arc<PatternCommands>,
    pub pattern_queries: Arc<PatternQueries>,
    pub review_commands: Arc<ReviewCommands>,
    pub review_queries: Arc<ReviewQueries>,
}
