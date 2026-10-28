//! [`IndexEvents`] delivered to the frontend as Tauri events.

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use ts_rs::TS;

use crate::application::ports::IndexEvents;

pub const INDEX_PROGRESS: &str = "index://progress";
pub const MODEL_UPDATED: &str = "index://updated";

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct IndexProgress {
    pub worktree_id: String,
    pub done: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModelUpdated {
    pub worktree_id: String,
    /// Symbol ids added or modified; empty after a full (re)index.
    pub changed: Vec<String>,
    pub removed: Vec<String>,
}

pub struct TauriEvents(pub AppHandle);

impl IndexEvents for TauriEvents {
    fn progress(&self, worktree: &str, done: usize, total: usize) {
        let _ = self.0.emit(
            INDEX_PROGRESS,
            IndexProgress {
                worktree_id: worktree.into(),
                done: done as u32,
                total: total as u32,
            },
        );
    }

    fn updated(&self, worktree: &str, changed: &[String], removed: &[String]) {
        let _ = self.0.emit(
            MODEL_UPDATED,
            ModelUpdated {
                worktree_id: worktree.into(),
                changed: changed.to_vec(),
                removed: removed.to_vec(),
            },
        );
    }
}
