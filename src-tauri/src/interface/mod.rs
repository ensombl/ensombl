//! Interface layer: thin Tauri commands that dispatch to application
//! handlers, and adapters that push application events to the frontend.

pub mod chat_sink;
pub mod commands;
pub mod events;
pub mod state;
pub mod terminal_sink;
