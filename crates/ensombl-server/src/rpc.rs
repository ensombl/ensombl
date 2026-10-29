//! The core's commands and queries by name, taking and returning JSON. The
//! same names as the desktop app's IPC, minus the ones that stream terminal
//! output or open windows on the host.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use ensombl_core::app::Handlers;
use ensombl_core::application::commands::architecture::{
    MergeComponents, SetRuleEnabled, SplitComponent, UpdateComponent,
};
use ensombl_core::application::commands::indexing::{DropIndex, IndexWorktree};
use ensombl_core::application::commands::patterns::SetPatternDecision;
use ensombl_core::application::commands::review::{
    AddComment, DeleteComment, ResolveComment, SaveSource, UpdateSettings,
};
use ensombl_core::application::commands::tasks::{
    ApproveTask, CancelTask, CreateTask, DeleteTask, QueueTask, RespondToTask, RetryTask,
    SendFollowUp, StartWorkflow, StopAllTasks, UpdateTaskBranch,
};
use ensombl_core::application::commands::terminals::{
    CloseTerminal, ResizeTerminal, WriteTerminal,
};
use ensombl_core::application::commands::workspaces::{
    ActivateTab, AddProject, CloseTab, CreateWorktree, OpenWorktree, RemoveProject, RemoveWorktree,
    ToggleGroup,
};
use ensombl_core::application::cqrs::{Command, Query};
use ensombl_core::application::cqrs::{CommandHandler, QueryHandler};
use ensombl_core::application::queries::architecture::{GetFlow, GetSystem};
use ensombl_core::application::queries::files::ListFiles;
use ensombl_core::application::queries::get_app_info::GetAppInfo;
use ensombl_core::application::queries::model::GetModel;
use ensombl_core::application::queries::patterns::GetPatterns;
use ensombl_core::application::queries::review::{GetSettings, ListComments, ReadSource};
use ensombl_core::application::queries::tasks::{
    GetCoordination, GetRunLog, GetTask, GetTaskDiff, GetUsage, ListTasks, ListWorkflows,
};
use ensombl_core::application::queries::terminals::{ListAgents, ListTerminals};
use ensombl_core::application::queries::workspaces::{GetSession, ListBranches, ListWorktrees};
use ensombl_core::application::{Error, Result};

fn to_value<T: Serialize>(result: Result<T>) -> Result<Value> {
    result.and_then(|v| serde_json::to_value(v).map_err(|e| Error::Invalid(e.to_string())))
}

fn parse<T: DeserializeOwned>(input: Value) -> Result<T> {
    serde_json::from_value(input).map_err(|e| Error::Invalid(format!("bad input: {e}")))
}

fn command<C, H>(handler: &H, input: Value) -> Result<Value>
where
    C: Command + DeserializeOwned,
    C::Output: Serialize,
    H: CommandHandler<C> + ?Sized,
{
    to_value(handler.handle(parse(input)?))
}

fn query<Q, H>(handler: &H, input: Value) -> Result<Value>
where
    Q: Query + DeserializeOwned,
    Q::Output: Serialize,
    H: QueryHandler<Q> + ?Sized,
{
    to_value(handler.handle(parse(input)?))
}

/// Every name [`call`] answers to.
pub const NAMES: &[&str] = &[
    "get_app_info",
    "get_session",
    "list_worktrees",
    "list_branches",
    "get_model",
    "list_agents",
    "list_terminals",
    "get_patterns",
    "read_source",
    "list_comments",
    "get_settings",
    "add_comment",
    "resolve_comment",
    "delete_comment",
    "update_settings",
    "save_source",
    "set_pattern_decision",
    "write_terminal",
    "resize_terminal",
    "close_terminal",
    "index_worktree",
    "drop_index",
    "add_project",
    "create_worktree",
    "open_worktree",
    "remove_worktree",
    "close_tab",
    "activate_tab",
    "toggle_group",
    "remove_project",
    "get_system",
    "get_flow",
    "update_component",
    "merge_components",
    "split_component",
    "set_rule_enabled",
    "list_files",
    "list_tasks",
    "get_task",
    "get_run_log",
    "get_task_diff",
    "create_task",
    "queue_task",
    "cancel_task",
    "retry_task",
    "respond_to_task",
    "send_follow_up",
    "approve_task",
    "delete_task",
    "get_coordination",
    "update_task_branch",
    "get_usage",
    "stop_all_tasks",
    "list_workflows",
    "start_workflow",
];

/// Runs the named command or query. Blocking: call it off the async runtime.
pub fn call(handlers: &Handlers, name: &str, input: Value) -> Result<Value> {
    match name {
        "get_app_info" => to_value(handlers.get_app_info.handle(GetAppInfo)),
        "get_session" => to_value(handlers.workspace_queries.handle(GetSession)),
        "list_worktrees" => query::<ListWorktrees, _>(&*handlers.workspace_queries, input),
        "list_branches" => query::<ListBranches, _>(&*handlers.workspace_queries, input),
        "get_model" => query::<GetModel, _>(&*handlers.model_queries, input),
        "list_agents" => to_value(handlers.terminal_queries.handle(ListAgents)),
        "list_terminals" => query::<ListTerminals, _>(&*handlers.terminal_queries, input),
        "get_patterns" => query::<GetPatterns, _>(&*handlers.pattern_queries, input),
        "read_source" => query::<ReadSource, _>(&*handlers.review_queries, input),
        "list_comments" => query::<ListComments, _>(&*handlers.review_queries, input),
        "get_settings" => to_value(handlers.review_queries.handle(GetSettings)),
        "add_comment" => command::<AddComment, _>(&*handlers.review_commands, input),
        "resolve_comment" => command::<ResolveComment, _>(&*handlers.review_commands, input),
        "delete_comment" => command::<DeleteComment, _>(&*handlers.review_commands, input),
        "update_settings" => command::<UpdateSettings, _>(&*handlers.review_commands, input),
        "save_source" => command::<SaveSource, _>(&*handlers.review_commands, input),
        "set_pattern_decision" => {
            command::<SetPatternDecision, _>(&*handlers.pattern_commands, input)
        }
        "write_terminal" => command::<WriteTerminal, _>(&*handlers.terminal_commands, input),
        "resize_terminal" => command::<ResizeTerminal, _>(&*handlers.terminal_commands, input),
        "close_terminal" => command::<CloseTerminal, _>(&*handlers.terminal_commands, input),
        "index_worktree" => command::<IndexWorktree, _>(&*handlers.indexing_commands, input),
        "drop_index" => command::<DropIndex, _>(&*handlers.indexing_commands, input),
        "add_project" => command::<AddProject, _>(&*handlers.workspace_commands, input),
        "create_worktree" => command::<CreateWorktree, _>(&*handlers.workspace_commands, input),
        "open_worktree" => command::<OpenWorktree, _>(&*handlers.workspace_commands, input),
        "remove_worktree" => command::<RemoveWorktree, _>(&*handlers.workspace_commands, input),
        "close_tab" => command::<CloseTab, _>(&*handlers.workspace_commands, input),
        "activate_tab" => command::<ActivateTab, _>(&*handlers.workspace_commands, input),
        "toggle_group" => command::<ToggleGroup, _>(&*handlers.workspace_commands, input),
        "remove_project" => command::<RemoveProject, _>(&*handlers.workspace_commands, input),
        "get_system" => query::<GetSystem, _>(&*handlers.architecture_queries, input),
        "get_flow" => query::<GetFlow, _>(&*handlers.architecture_queries, input),
        "update_component" => {
            command::<UpdateComponent, _>(&*handlers.architecture_commands, input)
        }
        "merge_components" => {
            command::<MergeComponents, _>(&*handlers.architecture_commands, input)
        }
        "split_component" => command::<SplitComponent, _>(&*handlers.architecture_commands, input),
        "set_rule_enabled" => command::<SetRuleEnabled, _>(&*handlers.architecture_commands, input),
        "list_files" => query::<ListFiles, _>(&*handlers.file_queries, input),
        "list_tasks" => query::<ListTasks, _>(&*handlers.task_queries, input),
        "get_task" => query::<GetTask, _>(&*handlers.task_queries, input),
        "get_run_log" => query::<GetRunLog, _>(&*handlers.task_queries, input),
        "get_task_diff" => query::<GetTaskDiff, _>(&*handlers.task_queries, input),
        "create_task" => command::<CreateTask, _>(&*handlers.task_commands, input),
        "queue_task" => command::<QueueTask, _>(&*handlers.task_commands, input),
        "cancel_task" => command::<CancelTask, _>(&*handlers.task_commands, input),
        "retry_task" => command::<RetryTask, _>(&*handlers.task_commands, input),
        "respond_to_task" => command::<RespondToTask, _>(&*handlers.task_commands, input),
        "send_follow_up" => command::<SendFollowUp, _>(&*handlers.task_commands, input),
        "approve_task" => command::<ApproveTask, _>(&*handlers.task_commands, input),
        "delete_task" => command::<DeleteTask, _>(&*handlers.task_commands, input),
        "get_coordination" => query::<GetCoordination, _>(&*handlers.task_queries, input),
        "update_task_branch" => command::<UpdateTaskBranch, _>(&*handlers.task_commands, input),
        "get_usage" => query::<GetUsage, _>(&*handlers.task_queries, input),
        "stop_all_tasks" => command::<StopAllTasks, _>(&*handlers.task_commands, input),
        "list_workflows" => query::<ListWorkflows, _>(&*handlers.task_queries, input),
        "start_workflow" => command::<StartWorkflow, _>(&*handlers.task_commands, input),
        _ => Err(Error::Invalid(format!("unknown command {name}"))),
    }
}
