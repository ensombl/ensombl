use crate::application::queries::get_app_info::GetAppInfoHandler;

/// Handlers wired by the composition root and managed by Tauri.
pub struct Handlers {
    pub get_app_info: GetAppInfoHandler,
}
