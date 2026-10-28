//! Core events delivered to the webview as Tauri events.

use ensombl_core::application::ports::EventBus;
use tauri::{AppHandle, Emitter};

pub struct TauriBus(pub AppHandle);

impl EventBus for TauriBus {
    fn publish(&self, topic: &str, payload: serde_json::Value) {
        let _ = self.0.emit(topic, payload);
    }
}
