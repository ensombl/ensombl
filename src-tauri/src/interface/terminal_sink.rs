//! [`TerminalSink`] that streams over a Tauri channel: raw bytes for output
//! (an `ArrayBuffer` in JS), a small JSON object when the process exits.

use tauri::ipc::{Channel, InvokeResponseBody};

use crate::application::ports::TerminalSink;

pub struct ChannelSink(pub Channel<InvokeResponseBody>);

impl TerminalSink for ChannelSink {
    fn output(&self, bytes: &[u8]) {
        if !bytes.is_empty() {
            let _ = self.0.send(InvokeResponseBody::Raw(bytes.to_vec()));
        }
    }

    fn exited(&self, code: Option<i32>) {
        let json = serde_json::json!({ "exit": code }).to_string();
        let _ = self.0.send(InvokeResponseBody::Json(json));
    }
}
