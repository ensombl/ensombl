pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod interface;

use std::sync::Arc;

use application::queries::get_app_info::GetAppInfoHandler;
use infrastructure::build_info::CargoBuildInfo;
use interface::state::Handlers;

/// Composition root: wire adapters into handlers and expose them to Tauri.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let handlers = Handlers {
        get_app_info: GetAppInfoHandler::new(Arc::new(CargoBuildInfo)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(handlers)
        .invoke_handler(tauri::generate_handler![interface::commands::get_app_info])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
