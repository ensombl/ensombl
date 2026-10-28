pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod interface;

use std::sync::Arc;

use tauri::Manager;

use application::commands::workspaces::WorkspaceCommands;
use application::queries::get_app_info::GetAppInfoHandler;
use application::queries::workspaces::WorkspaceQueries;
use infrastructure::build_info::CargoBuildInfo;
use infrastructure::git_cli::GitCli;
use infrastructure::json_sessions::JsonSessions;
use interface::commands as cmd;
use interface::state::Handlers;

/// Composition root: wire adapters into handlers and expose them to Tauri.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let paths = app.path();
            let sessions = Arc::new(JsonSessions::open(paths.app_data_dir()?.join("session.json")));
            let worktrees_dir = paths.home_dir()?.join(".crm").join("worktrees");
            let git = Arc::new(GitCli);

            app.manage(Handlers {
                get_app_info: GetAppInfoHandler::new(Arc::new(CargoBuildInfo)),
                workspace_commands: Arc::new(WorkspaceCommands::new(git.clone(), sessions.clone(), worktrees_dir)),
                workspace_queries: Arc::new(WorkspaceQueries::new(git, sessions)),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            cmd::get_app_info,
            cmd::get_session,
            cmd::list_worktrees,
            cmd::list_branches,
            cmd::add_project,
            cmd::create_worktree,
            cmd::open_worktree,
            cmd::remove_worktree,
            cmd::close_tab,
            cmd::activate_tab,
            cmd::toggle_group,
            cmd::remove_project,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
