//! [`ChatSink`] that streams chat events over a Tauri channel as JSON.

use tauri::ipc::Channel;

use crate::application::ports::ChatSink;
use crate::domain::chat::ChatEvent;

pub struct ChatChannelSink(pub Channel<ChatEvent>);

impl ChatSink for ChatChannelSink {
    fn event(&self, event: &ChatEvent) {
        let _ = self.0.send(event.clone());
    }
}
