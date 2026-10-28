use tauri::State;

use crate::application::cqrs::QueryHandler;
use crate::application::queries::get_app_info::{AppInfo, GetAppInfo};
use crate::application::Result;

use super::state::Handlers;

#[tauri::command]
pub fn get_app_info(handlers: State<'_, Handlers>) -> Result<AppInfo> {
    handlers.get_app_info.handle(GetAppInfo)
}
