pub mod interface;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use ensombl_core::app::{AppPaths, Core, CoreOptions};
use tauri::Manager;

use ensombl_server::BroadcastBus;
use interface::commands as cmd;
use interface::events::{FanoutBus, TauriBus};

/// The lead agent's MCP config: this executable in `--mcp` mode, pointed at
/// the local API.
fn write_mcp_config(data_dir: &Path, api_file: &Path) -> std::io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let config = serde_json::json!({
        "mcpServers": {
            "ensombl": {
                "command": exe,
                "args": ["--mcp", "--api-file", api_file],
            }
        }
    });
    std::fs::create_dir_all(data_dir)?;
    let path = data_dir.join("mcp.json");
    std::fs::write(&path, serde_json::to_string_pretty(&config)?)?;
    Ok(path)
}

/// Wires the core up inside Tauri: paths from the OS, events to the webview.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let paths = app.path();
            let data_dir = paths.app_data_dir()?;
            let api_bus = BroadcastBus::new();
            let api_file = data_dir.join("api.json");
            let lead_mcp_config = write_mcp_config(&data_dir, &api_file)
                .inspect_err(|e| eprintln!("lead agent unavailable: {e}"))
                .ok();
            let core = Core::build_with(
                AppPaths {
                    data_dir: data_dir.clone(),
                    worktrees_dir: paths.home_dir()?.join(".crm").join("worktrees"),
                },
                Arc::new(FanoutBus(vec![
                    Arc::new(TauriBus(app.handle().clone())),
                    Arc::new(api_bus.clone()),
                ])),
                CoreOptions { lead_mcp_config },
            )?;
            // The same API as ensombl-server, on localhost, for local tools
            // like the MCP bridge the lead agent uses.
            if let Err(e) = ensombl_server::spawn_local(
                core.handlers.clone(),
                &api_bus,
                &data_dir.join("api.json"),
            ) {
                eprintln!("local API unavailable: {e}");
            }
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
            cmd::get_coordination,
            cmd::update_task_branch,
            cmd::get_usage,
            cmd::stop_all_tasks,
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
