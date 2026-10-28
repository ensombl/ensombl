//! Tauri entry points. Each one only takes the message from the frontend and
//! dispatches it to its handler; git-touching work runs off the main thread.

use std::sync::Arc;

use tauri::State;
use tauri::ipc::{Channel, InvokeResponseBody};

use ensombl_core::application::commands::architecture::{
    MergeComponents, SetRuleEnabled, SplitComponent, UpdateComponent,
};
use ensombl_core::application::commands::indexing::{DropIndex, IndexSummary, IndexWorktree};
use ensombl_core::application::commands::patterns::SetPatternDecision;
use ensombl_core::application::commands::review::{
    AddComment, DeleteComment, EditorLaunch, OpenInEditor, ResolveComment, SaveSource,
    UpdateSettings,
};
use ensombl_core::application::commands::tasks::{
    ApproveTask, CancelTask, CreateTask, DeleteTask, QueueTask, RespondToTask, RetryTask,
    SendFollowUp,
};
use ensombl_core::application::commands::terminals::{
    AttachTerminal, CloseTerminal, ResizeTerminal, StartTerminal, WithSink, WriteTerminal,
};
use ensombl_core::application::commands::workspaces::{
    ActivateTab, AddProject, CloseTab, CreateWorktree, OpenWorktree, RemoveProject, RemoveWorktree,
    ToggleGroup,
};
use ensombl_core::application::cqrs::{CommandHandler, QueryHandler};
use ensombl_core::application::ports::Settings;
use ensombl_core::application::queries::architecture::{GetFlow, GetSystem, SystemView};
use ensombl_core::application::queries::files::ListFiles;
use ensombl_core::application::queries::get_app_info::{AppInfo, GetAppInfo};
use ensombl_core::application::queries::model::GetModel;
use ensombl_core::application::queries::patterns::{GetPatterns, PatternView};
use ensombl_core::application::queries::review::{
    GetSettings, ListComments, ReadSource, SourceText,
};
use ensombl_core::application::queries::tasks::{
    GetRunLog, GetTask, GetTaskDiff, ListTasks, TaskDetail,
};
use ensombl_core::application::queries::terminals::{AgentAvailability, ListAgents, ListTerminals};
use ensombl_core::application::queries::workspaces::{
    GetSession, ListBranches, ListWorktrees, SessionView, WorktreeListing,
};
use ensombl_core::application::{Error, Result};
use ensombl_core::domain::agent_run::RunEvent;
use ensombl_core::domain::model::ArchModel;
use ensombl_core::domain::orchestration::Task;
use ensombl_core::domain::review::{AnchoredComment, ReviewComment};
use ensombl_core::domain::system::flow::FlowStep;
use ensombl_core::domain::terminal::TerminalInfo;
use ensombl_core::domain::workspace::WorktreeId;

use super::terminal_sink::ChannelSink;
use ensombl_core::app::Handlers;

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
pub fn get_app_info(handlers: State<'_, Arc<Handlers>>) -> Result<AppInfo> {
    handlers.get_app_info.handle(GetAppInfo)
}

#[tauri::command]
pub fn get_session(handlers: State<'_, Arc<Handlers>>) -> Result<SessionView> {
    handlers.workspace_queries.handle(GetSession)
}

#[tauri::command]
pub async fn list_worktrees(
    handlers: State<'_, Arc<Handlers>>,
    input: ListWorktrees,
) -> Result<Vec<WorktreeListing>> {
    blocking(&handlers.workspace_queries, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn list_branches(
    handlers: State<'_, Arc<Handlers>>,
    input: ListBranches,
) -> Result<Vec<String>> {
    blocking(&handlers.workspace_queries, move |h| h.handle(input)).await
}

#[tauri::command]
pub fn get_model(handlers: State<'_, Arc<Handlers>>, input: GetModel) -> Result<Option<ArchModel>> {
    handlers.model_queries.handle(input)
}

#[tauri::command]
pub async fn list_agents(handlers: State<'_, Arc<Handlers>>) -> Result<Vec<AgentAvailability>> {
    blocking(&handlers.terminal_queries, |h| h.handle(ListAgents)).await
}

#[tauri::command]
pub fn list_terminals(
    handlers: State<'_, Arc<Handlers>>,
    input: ListTerminals,
) -> Result<Vec<TerminalInfo>> {
    handlers.terminal_queries.handle(input)
}

#[tauri::command]
pub async fn get_patterns(
    handlers: State<'_, Arc<Handlers>>,
    input: GetPatterns,
) -> Result<Vec<PatternView>> {
    blocking(&handlers.pattern_queries, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn read_source(
    handlers: State<'_, Arc<Handlers>>,
    input: ReadSource,
) -> Result<SourceText> {
    blocking(&handlers.review_queries, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn list_comments(
    handlers: State<'_, Arc<Handlers>>,
    input: ListComments,
) -> Result<Vec<AnchoredComment>> {
    blocking(&handlers.review_queries, move |h| h.handle(input)).await
}

#[tauri::command]
pub fn get_settings(handlers: State<'_, Arc<Handlers>>) -> Result<Settings> {
    handlers.review_queries.handle(GetSettings)
}

// Commands

#[tauri::command]
pub async fn add_comment(
    handlers: State<'_, Arc<Handlers>>,
    input: AddComment,
) -> Result<ReviewComment> {
    blocking(&handlers.review_commands, move |h| h.handle(input)).await
}

#[tauri::command]
pub fn resolve_comment(handlers: State<'_, Arc<Handlers>>, input: ResolveComment) -> Result<()> {
    handlers.review_commands.handle(input)
}

#[tauri::command]
pub fn delete_comment(handlers: State<'_, Arc<Handlers>>, input: DeleteComment) -> Result<()> {
    handlers.review_commands.handle(input)
}

#[tauri::command]
pub fn update_settings(handlers: State<'_, Arc<Handlers>>, input: UpdateSettings) -> Result<()> {
    handlers.review_commands.handle(input)
}

#[tauri::command]
pub async fn open_in_editor(
    handlers: State<'_, Arc<Handlers>>,
    input: OpenInEditor,
) -> Result<EditorLaunch> {
    blocking(&handlers.review_commands, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn save_source(handlers: State<'_, Arc<Handlers>>, input: SaveSource) -> Result<()> {
    blocking(&handlers.review_commands, move |h| h.handle(input)).await
}

#[tauri::command]
pub fn set_pattern_decision(
    handlers: State<'_, Arc<Handlers>>,
    input: SetPatternDecision,
) -> Result<()> {
    handlers.pattern_commands.handle(input)
}

#[tauri::command]
pub async fn start_terminal(
    handlers: State<'_, Arc<Handlers>>,
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
    handlers: State<'_, Arc<Handlers>>,
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
pub fn write_terminal(handlers: State<'_, Arc<Handlers>>, input: WriteTerminal) -> Result<()> {
    handlers.terminal_commands.handle(input)
}

#[tauri::command]
pub fn resize_terminal(handlers: State<'_, Arc<Handlers>>, input: ResizeTerminal) -> Result<()> {
    handlers.terminal_commands.handle(input)
}

#[tauri::command]
pub fn close_terminal(handlers: State<'_, Arc<Handlers>>, input: CloseTerminal) -> Result<()> {
    handlers.terminal_commands.handle(input)
}

#[tauri::command]
pub async fn index_worktree(
    handlers: State<'_, Arc<Handlers>>,
    input: IndexWorktree,
) -> Result<IndexSummary> {
    blocking(&handlers.indexing_commands, move |h| h.handle(input)).await
}

#[tauri::command]
pub fn drop_index(handlers: State<'_, Arc<Handlers>>, input: DropIndex) -> Result<()> {
    handlers.indexing_commands.handle(input)
}

#[tauri::command]
pub async fn add_project(
    handlers: State<'_, Arc<Handlers>>,
    input: AddProject,
) -> Result<WorktreeId> {
    blocking(&handlers.workspace_commands, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn create_worktree(
    handlers: State<'_, Arc<Handlers>>,
    input: CreateWorktree,
) -> Result<WorktreeId> {
    blocking(&handlers.workspace_commands, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn open_worktree(
    handlers: State<'_, Arc<Handlers>>,
    input: OpenWorktree,
) -> Result<WorktreeId> {
    blocking(&handlers.workspace_commands, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn remove_worktree(
    handlers: State<'_, Arc<Handlers>>,
    input: RemoveWorktree,
) -> Result<()> {
    blocking(&handlers.workspace_commands, move |h| h.handle(input)).await
}

#[tauri::command]
pub fn close_tab(handlers: State<'_, Arc<Handlers>>, input: CloseTab) -> Result<()> {
    handlers.workspace_commands.handle(input)
}

#[tauri::command]
pub fn activate_tab(handlers: State<'_, Arc<Handlers>>, input: ActivateTab) -> Result<()> {
    handlers.workspace_commands.handle(input)
}

#[tauri::command]
pub fn toggle_group(handlers: State<'_, Arc<Handlers>>, input: ToggleGroup) -> Result<()> {
    handlers.workspace_commands.handle(input)
}

#[tauri::command]
pub fn remove_project(handlers: State<'_, Arc<Handlers>>, input: RemoveProject) -> Result<()> {
    handlers.workspace_commands.handle(input)
}

// Architecture

#[tauri::command]
pub async fn get_system(
    handlers: State<'_, Arc<Handlers>>,
    input: GetSystem,
) -> Result<Option<SystemView>> {
    blocking(&handlers.architecture_queries, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn get_flow(handlers: State<'_, Arc<Handlers>>, input: GetFlow) -> Result<Vec<FlowStep>> {
    blocking(&handlers.architecture_queries, move |h| h.handle(input)).await
}

#[tauri::command]
pub fn update_component(handlers: State<'_, Arc<Handlers>>, input: UpdateComponent) -> Result<()> {
    handlers.architecture_commands.handle(input)
}

#[tauri::command]
pub fn merge_components(
    handlers: State<'_, Arc<Handlers>>,
    input: MergeComponents,
) -> Result<String> {
    handlers.architecture_commands.handle(input)
}

#[tauri::command]
pub fn split_component(handlers: State<'_, Arc<Handlers>>, input: SplitComponent) -> Result<()> {
    handlers.architecture_commands.handle(input)
}

#[tauri::command]
pub fn set_rule_enabled(handlers: State<'_, Arc<Handlers>>, input: SetRuleEnabled) -> Result<()> {
    handlers.architecture_commands.handle(input)
}

#[tauri::command]
pub async fn list_files(
    handlers: State<'_, Arc<Handlers>>,
    input: ListFiles,
) -> Result<Vec<String>> {
    blocking(&handlers.file_queries, move |h| h.handle(input)).await
}

// Orchestrated tasks. Commands start agents and touch git, so they run off
// the main thread.

#[tauri::command]
pub async fn list_tasks(handlers: State<'_, Arc<Handlers>>, input: ListTasks) -> Result<Vec<Task>> {
    blocking(&handlers.task_queries, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn get_task(handlers: State<'_, Arc<Handlers>>, input: GetTask) -> Result<TaskDetail> {
    blocking(&handlers.task_queries, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn get_run_log(
    handlers: State<'_, Arc<Handlers>>,
    input: GetRunLog,
) -> Result<Vec<RunEvent>> {
    blocking(&handlers.task_queries, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn get_task_diff(
    handlers: State<'_, Arc<Handlers>>,
    input: GetTaskDiff,
) -> Result<String> {
    blocking(&handlers.task_queries, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn create_task(handlers: State<'_, Arc<Handlers>>, input: CreateTask) -> Result<Task> {
    blocking(&handlers.task_commands, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn queue_task(handlers: State<'_, Arc<Handlers>>, input: QueueTask) -> Result<Task> {
    blocking(&handlers.task_commands, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn cancel_task(handlers: State<'_, Arc<Handlers>>, input: CancelTask) -> Result<Task> {
    blocking(&handlers.task_commands, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn retry_task(handlers: State<'_, Arc<Handlers>>, input: RetryTask) -> Result<Task> {
    blocking(&handlers.task_commands, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn respond_to_task(
    handlers: State<'_, Arc<Handlers>>,
    input: RespondToTask,
) -> Result<()> {
    blocking(&handlers.task_commands, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn send_follow_up(
    handlers: State<'_, Arc<Handlers>>,
    input: SendFollowUp,
) -> Result<Task> {
    blocking(&handlers.task_commands, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn approve_task(handlers: State<'_, Arc<Handlers>>, input: ApproveTask) -> Result<Task> {
    blocking(&handlers.task_commands, move |h| h.handle(input)).await
}

#[tauri::command]
pub async fn delete_task(handlers: State<'_, Arc<Handlers>>, input: DeleteTask) -> Result<()> {
    blocking(&handlers.task_commands, move |h| h.handle(input)).await
}
