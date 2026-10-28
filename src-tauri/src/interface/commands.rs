//! Tauri entry points. Each one only takes the message from the frontend and
//! dispatches it to its handler; git-touching work runs off the main thread.

use std::sync::Arc;

use tauri::State;
use tauri::ipc::{Channel, InvokeResponseBody};

use crate::application::commands::architecture::{
    MergeComponents, SetRuleEnabled, SplitComponent, UpdateComponent,
};
use crate::application::commands::chats::{
    AttachChat, ChatSnapshot, CloseChat, InterruptChat, RespondPermission, SendChatMessage,
    SetChatModel, SetChatPermissionMode, StartChat, WithChatSink,
};
use crate::application::commands::indexing::{DropIndex, IndexSummary, IndexWorktree};
use crate::application::commands::patterns::SetPatternDecision;
use crate::application::commands::review::{
    AddComment, DeleteComment, EditorLaunch, OpenInEditor, ResolveComment, SaveSource,
    UpdateSettings,
};
use crate::application::commands::terminals::{
    AttachTerminal, CloseTerminal, ResizeTerminal, StartTerminal, WithSink, WriteTerminal,
};
use crate::application::commands::workspaces::{
    ActivateTab, AddProject, CloseTab, CreateWorktree, OpenWorktree, RemoveProject, RemoveWorktree,
    ToggleGroup,
};
use crate::application::cqrs::{CommandHandler, QueryHandler};
use crate::application::ports::Settings;
use crate::application::queries::architecture::{GetFlow, GetSystem, SystemView};
use crate::application::queries::chats::ListChats;
use crate::application::queries::files::ListFiles;
use crate::application::queries::get_app_info::{AppInfo, GetAppInfo};
use crate::application::queries::model::GetModel;
use crate::application::queries::patterns::{GetPatterns, PatternView};
use crate::application::queries::review::{GetSettings, ListComments, ReadSource, SourceText};
use crate::application::queries::terminals::{AgentAvailability, ListAgents, ListTerminals};
use crate::application::queries::workspaces::{
    GetSession, ListBranches, ListWorktrees, SessionView, WorktreeListing,
};
use crate::application::{Error, Result};
use crate::domain::chat::{ChatEvent, ChatInfo};
use crate::domain::model::ArchModel;
use crate::domain::review::{AnchoredComment, ReviewComment};
use crate::domain::system::flow::FlowStep;
use crate::domain::terminal::TerminalInfo;
use crate::domain::workspace::WorktreeId;

use super::chat_sink::ChatChannelSink;
use super::state::Handlers;
use super::terminal_sink::ChannelSink;

async fn blocking<H, T>(
    handler: &Arc<H>,
    f: impl FnOnce(&H) -> Result<T> + Send + 'static,
) -> Result<T>
where
    H: Send + Sync + 'static,
    T: Send + 'static,
{
    let handler = handler.clone();
    tauri::async_runtime::spawn_blocking(move || f(&handler))
        .await
        .map_err(|e| Error::Invalid(e.to_string()))?
}

// Queries

#[tauri::command]
pub fn get_app_info(handlers: State<'_, Handlers>) -> Result<AppInfo> {
    handlers.get_app_info.handle(GetAppInfo)
}

#[tauri::command]
pub fn get_session(handlers: State<'_, Handlers>) -> Result<SessionView> {
    handlers.workspace_queries.handle(GetSession)
}

#[tauri::command]
pub async fn list_worktrees(
    handlers: State<'_, Handlers>,
    input: ListWorktrees,
) -> Result<Vec<WorktreeListing>> {
    blocking(&handlers.workspace_queries, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn list_branches(
    handlers: State<'_, Handlers>,
    input: ListBranches,
) -> Result<Vec<String>> {
    blocking(&handlers.workspace_queries, move |h| h.handle(input)).await
}

#[tauri::command]
pub fn get_model(handlers: State<'_, Handlers>, input: GetModel) -> Result<Option<ArchModel>> {
    handlers.model_queries.handle(input)
}

#[tauri::command]
pub async fn list_agents(handlers: State<'_, Handlers>) -> Result<Vec<AgentAvailability>> {
    blocking(&handlers.terminal_queries, |h| h.handle(ListAgents)).await
}

#[tauri::command]
pub fn list_terminals(
    handlers: State<'_, Handlers>,
    input: ListTerminals,
) -> Result<Vec<TerminalInfo>> {
    handlers.terminal_queries.handle(input)
}

#[tauri::command]
pub async fn get_patterns(
    handlers: State<'_, Handlers>,
    input: GetPatterns,
) -> Result<Vec<PatternView>> {
    blocking(&handlers.pattern_queries, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn read_source(handlers: State<'_, Handlers>, input: ReadSource) -> Result<SourceText> {
    blocking(&handlers.review_queries, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn list_comments(
    handlers: State<'_, Handlers>,
    input: ListComments,
) -> Result<Vec<AnchoredComment>> {
    blocking(&handlers.review_queries, move |h| h.handle(input)).await
}

#[tauri::command]
pub fn get_settings(handlers: State<'_, Handlers>) -> Result<Settings> {
    handlers.review_queries.handle(GetSettings)
}

// Commands

#[tauri::command]
pub async fn add_comment(
    handlers: State<'_, Handlers>,
    input: AddComment,
) -> Result<ReviewComment> {
    blocking(&handlers.review_commands, move |h| h.handle(input)).await
}

#[tauri::command]
pub fn resolve_comment(handlers: State<'_, Handlers>, input: ResolveComment) -> Result<()> {
    handlers.review_commands.handle(input)
}

#[tauri::command]
pub fn delete_comment(handlers: State<'_, Handlers>, input: DeleteComment) -> Result<()> {
    handlers.review_commands.handle(input)
}

#[tauri::command]
pub fn update_settings(handlers: State<'_, Handlers>, input: UpdateSettings) -> Result<()> {
    handlers.review_commands.handle(input)
}

#[tauri::command]
pub async fn open_in_editor(
    handlers: State<'_, Handlers>,
    input: OpenInEditor,
) -> Result<EditorLaunch> {
    blocking(&handlers.review_commands, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn save_source(handlers: State<'_, Handlers>, input: SaveSource) -> Result<()> {
    blocking(&handlers.review_commands, move |h| h.handle(input)).await
}

#[tauri::command]
pub fn set_pattern_decision(
    handlers: State<'_, Handlers>,
    input: SetPatternDecision,
) -> Result<()> {
    handlers.pattern_commands.handle(input)
}

#[tauri::command]
pub async fn start_terminal(
    handlers: State<'_, Handlers>,
    input: StartTerminal,
    output: Channel<InvokeResponseBody>,
) -> Result<TerminalInfo> {
    let sink = Arc::new(ChannelSink(output));
    blocking(&handlers.terminal_commands, move |h| {
        h.handle(WithSink {
            command: input,
            sink,
        })
    })
    .await
}

#[tauri::command]
pub fn attach_terminal(
    handlers: State<'_, Handlers>,
    input: AttachTerminal,
    output: Channel<InvokeResponseBody>,
) -> Result<TerminalInfo> {
    let sink = Arc::new(ChannelSink(output));
    handlers.terminal_commands.handle(WithSink {
        command: input,
        sink,
    })
}

#[tauri::command]
pub fn write_terminal(handlers: State<'_, Handlers>, input: WriteTerminal) -> Result<()> {
    handlers.terminal_commands.handle(input)
}

#[tauri::command]
pub fn resize_terminal(handlers: State<'_, Handlers>, input: ResizeTerminal) -> Result<()> {
    handlers.terminal_commands.handle(input)
}

#[tauri::command]
pub fn close_terminal(handlers: State<'_, Handlers>, input: CloseTerminal) -> Result<()> {
    handlers.terminal_commands.handle(input)
}

#[tauri::command]
pub async fn index_worktree(
    handlers: State<'_, Handlers>,
    input: IndexWorktree,
) -> Result<IndexSummary> {
    blocking(&handlers.indexing_commands, move |h| h.handle(input)).await
}

#[tauri::command]
pub fn drop_index(handlers: State<'_, Handlers>, input: DropIndex) -> Result<()> {
    handlers.indexing_commands.handle(input)
}

#[tauri::command]
pub async fn add_project(handlers: State<'_, Handlers>, input: AddProject) -> Result<WorktreeId> {
    blocking(&handlers.workspace_commands, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn create_worktree(
    handlers: State<'_, Handlers>,
    input: CreateWorktree,
) -> Result<WorktreeId> {
    blocking(&handlers.workspace_commands, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn open_worktree(
    handlers: State<'_, Handlers>,
    input: OpenWorktree,
) -> Result<WorktreeId> {
    blocking(&handlers.workspace_commands, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn remove_worktree(handlers: State<'_, Handlers>, input: RemoveWorktree) -> Result<()> {
    blocking(&handlers.workspace_commands, move |h| h.handle(input)).await
}

#[tauri::command]
pub fn close_tab(handlers: State<'_, Handlers>, input: CloseTab) -> Result<()> {
    handlers.workspace_commands.handle(input)
}

#[tauri::command]
pub fn activate_tab(handlers: State<'_, Handlers>, input: ActivateTab) -> Result<()> {
    handlers.workspace_commands.handle(input)
}

#[tauri::command]
pub fn toggle_group(handlers: State<'_, Handlers>, input: ToggleGroup) -> Result<()> {
    handlers.workspace_commands.handle(input)
}

#[tauri::command]
pub fn remove_project(handlers: State<'_, Handlers>, input: RemoveProject) -> Result<()> {
    handlers.workspace_commands.handle(input)
}

// Chats

#[tauri::command]
pub fn list_chats(handlers: State<'_, Handlers>, input: ListChats) -> Result<Vec<ChatInfo>> {
    handlers.chat_queries.handle(input)
}

#[tauri::command]
pub async fn start_chat(
    handlers: State<'_, Handlers>,
    input: StartChat,
    output: Channel<ChatEvent>,
) -> Result<ChatInfo> {
    let sink = Arc::new(ChatChannelSink(output));
    blocking(&handlers.chat_commands, move |h| {
        h.handle(WithChatSink {
            command: input,
            sink,
        })
    })
    .await
}

#[tauri::command]
pub fn attach_chat(
    handlers: State<'_, Handlers>,
    input: AttachChat,
    output: Channel<ChatEvent>,
) -> Result<ChatSnapshot> {
    let sink = Arc::new(ChatChannelSink(output));
    handlers.chat_commands.handle(WithChatSink {
        command: input,
        sink,
    })
}

#[tauri::command]
pub fn send_chat_message(handlers: State<'_, Handlers>, input: SendChatMessage) -> Result<()> {
    handlers.chat_commands.handle(input)
}

#[tauri::command]
pub fn respond_permission(handlers: State<'_, Handlers>, input: RespondPermission) -> Result<()> {
    handlers.chat_commands.handle(input)
}

#[tauri::command]
pub fn interrupt_chat(handlers: State<'_, Handlers>, input: InterruptChat) -> Result<()> {
    handlers.chat_commands.handle(input)
}

#[tauri::command]
pub fn set_chat_permission_mode(
    handlers: State<'_, Handlers>,
    input: SetChatPermissionMode,
) -> Result<()> {
    handlers.chat_commands.handle(input)
}

#[tauri::command]
pub fn set_chat_model(handlers: State<'_, Handlers>, input: SetChatModel) -> Result<()> {
    handlers.chat_commands.handle(input)
}

#[tauri::command]
pub fn close_chat(handlers: State<'_, Handlers>, input: CloseChat) -> Result<()> {
    handlers.chat_commands.handle(input)
}

// Architecture

#[tauri::command]
pub async fn get_system(
    handlers: State<'_, Handlers>,
    input: GetSystem,
) -> Result<Option<SystemView>> {
    blocking(&handlers.architecture_queries, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn get_flow(handlers: State<'_, Handlers>, input: GetFlow) -> Result<Vec<FlowStep>> {
    blocking(&handlers.architecture_queries, move |h| h.handle(input)).await
}

#[tauri::command]
pub fn update_component(handlers: State<'_, Handlers>, input: UpdateComponent) -> Result<()> {
    handlers.architecture_commands.handle(input)
}

#[tauri::command]
pub fn merge_components(handlers: State<'_, Handlers>, input: MergeComponents) -> Result<String> {
    handlers.architecture_commands.handle(input)
}

#[tauri::command]
pub fn split_component(handlers: State<'_, Handlers>, input: SplitComponent) -> Result<()> {
    handlers.architecture_commands.handle(input)
}

#[tauri::command]
pub fn set_rule_enabled(handlers: State<'_, Handlers>, input: SetRuleEnabled) -> Result<()> {
    handlers.architecture_commands.handle(input)
}

#[tauri::command]
pub async fn list_files(handlers: State<'_, Handlers>, input: ListFiles) -> Result<Vec<String>> {
    blocking(&handlers.file_queries, move |h| h.handle(input)).await
}
