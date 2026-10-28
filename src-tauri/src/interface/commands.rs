//! Tauri entry points. Each one only takes the message from the frontend and
//! dispatches it to its handler; git-touching work runs off the main thread.

use std::sync::Arc;

use tauri::State;

use crate::application::commands::indexing::{DropIndex, IndexSummary, IndexWorktree};
use crate::application::commands::workspaces::{
    ActivateTab, AddProject, CloseTab, CreateWorktree, OpenWorktree, RemoveProject, RemoveWorktree,
    ToggleGroup,
};
use crate::application::cqrs::{CommandHandler, QueryHandler};
use crate::application::queries::get_app_info::{AppInfo, GetAppInfo};
use crate::application::queries::model::GetModel;
use crate::application::queries::workspaces::{
    GetSession, ListBranches, ListWorktrees, SessionView, WorktreeListing,
};
use crate::application::{Error, Result};
use crate::domain::model::ArchModel;
use crate::domain::workspace::WorktreeId;

use super::state::Handlers;

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

// Commands

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
