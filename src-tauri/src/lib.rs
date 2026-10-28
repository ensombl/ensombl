pub mod interface;

use std::sync::Arc;

use ensombl_core::app::{AppPaths, Core};
use tauri::Manager;

use interface::commands as cmd;
use interface::events::TauriBus;

/// Wires the core up inside Tauri: paths from the OS, events to the webview.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let paths = app.path();
            let core = Core::build(
                AppPaths {
                    data_dir: paths.app_data_dir()?,
                    worktrees_dir: paths.home_dir()?.join(".crm").join("worktrees"),
                },
                Arc::new(TauriBus(app.handle().clone())),
            )?;
            app.manage(core.handlers.clone());
            app.manage(Arc::new(core));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            cmd::get_app_info,
            cmd::get_session,
            cmd::list_worktrees,
            cmd::list_branches,
            cmd::get_model,
            cmd::add_project,
            cmd::create_worktree,
            cmd::open_worktree,
            cmd::remove_worktree,
            cmd::close_tab,
            cmd::activate_tab,
            cmd::toggle_group,
            cmd::remove_project,
            cmd::index_worktree,
            cmd::drop_index,
            cmd::get_patterns,
            cmd::set_pattern_decision,
            cmd::read_source,
            cmd::save_source,
            cmd::list_comments,
            cmd::get_settings,
            cmd::add_comment,
            cmd::resolve_comment,
            cmd::delete_comment,
            cmd::update_settings,
            cmd::open_in_editor,
            cmd::list_agents,
            cmd::list_terminals,
            cmd::start_terminal,
            cmd::attach_terminal,
            cmd::write_terminal,
            cmd::resize_terminal,
            cmd::close_terminal,
            cmd::get_system,
            cmd::get_flow,
            cmd::update_component,
            cmd::merge_components,
            cmd::split_component,
            cmd::set_rule_enabled,
            cmd::list_files,
            cmd::list_tasks,
            cmd::get_task,
            cmd::get_run_log,
            cmd::get_task_diff,
            cmd::create_task,
            cmd::queue_task,
            cmd::cancel_task,
            cmd::retry_task,
            cmd::respond_to_task,
            cmd::send_follow_up,
            cmd::approve_task,
            cmd::delete_task,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // Don't leave agent processes behind.
            if let tauri::RunEvent::Exit = event {
                app.state::<Arc<Core>>().shutdown();
            }
        });
}
