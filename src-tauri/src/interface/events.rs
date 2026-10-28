//! Core events delivered to the webview as Tauri events.

use std::sync::Arc;

use ensombl_core::application::ports::EventBus;
use tauri::{AppHandle, Emitter};

pub struct TauriBus(pub AppHandle);

impl EventBus for TauriBus {
    fn publish(&self, topic: &str, payload: serde_json::Value) {
        let _ = self.0.emit(topic, payload);
    }
}

/// Publishes to the webview and to the local API's WebSocket clients.
pub struct FanoutBus(pub Vec<Arc<dyn EventBus>>);

impl EventBus for FanoutBus {
    fn publish(&self, topic: &str, payload: serde_json::Value) {
        for bus in &self.0 {
            bus.publish(topic, payload.clone());
        }
    }
}
