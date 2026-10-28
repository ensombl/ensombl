//! Interface layer: thin Tauri commands that dispatch to application
//! handlers, and adapters that push application events to the frontend.

pub mod commands;
pub mod events;
pub mod terminal_sink;
