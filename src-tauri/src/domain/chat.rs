//! Native agent chats: an agent CLI driven through its structured protocol
//! instead of a terminal. Every runtime is normalized into [`ChatEvent`]s so
//! the UI renders Claude, Codex, ... the same way.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use super::terminal::AgentKind;
use super::workspace::WorktreeId;

pub type ChatId = String;

/// How much the agent may do without asking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PermissionMode {
    /// Ask before anything that isn't read-only.
    #[default]
    #[serde(alias = "manual")]
    Default,
    AcceptEdits,
    Plan,
    BypassPermissions,
}

impl PermissionMode {
    pub fn as_cli(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::AcceptEdits => "acceptEdits",
            Self::Plan => "plan",
            Self::BypassPermissions => "bypassPermissions",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChatInfo {
    pub id: ChatId,
    pub worktree_id: WorktreeId,
    pub profile_id: String,
    pub agent: AgentKind,
    pub title: String,
    pub running: bool,
    /// A turn is in progress.
    pub busy: bool,
    pub session_id: Option<String>,
    pub model: Option<String>,
    pub permission_mode: PermissionMode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum ContentBlock {
    Text {
        text: String,
    },
    Thinking {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        #[ts(type = "unknown")]
        input: Value,
    },
}

/// The user's answer to a permission request.
#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PermissionDecision {
    pub allow: bool,
    /// Also allow similar calls for the rest of the session.
    #[serde(default)]
    pub remember: bool,
    /// Replaces the tool input (e.g. answers to `AskUserQuestion`).
    #[serde(default)]
    #[ts(type = "unknown", optional)]
    pub updated_input: Option<Value>,
    #[serde(default)]
    #[ts(optional)]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum NoticeLevel {
    Info,
    Error,
}

/// One thing that happened in a chat, in order.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum ChatEvent {
    /// The agent reported its session (sent again at the start of each turn).
    Session {
        session_id: String,
        model: Option<String>,
        permission_mode: Option<PermissionMode>,
    },
    User {
        id: String,
        text: String,
    },
    /// A streamed content block began. `parent` is set for subagent output.
    BlockStart {
        message_id: String,
        parent: Option<String>,
        index: u32,
        block: ContentBlock,
    },
    BlockDelta {
        message_id: String,
        index: u32,
        text: String,
    },
    /// A complete assistant message (or part of one); authoritative over deltas.
    Assistant {
        message_id: String,
        parent: Option<String>,
        blocks: Vec<ContentBlock>,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
    Permission {
        request_id: String,
        tool_name: String,
        tool_use_id: Option<String>,
        #[ts(type = "unknown")]
        input: Value,
        description: Option<String>,
    },
    PermissionResolved {
        request_id: String,
        allowed: bool,
    },
    TurnComplete {
        is_error: bool,
        result: Option<String>,
        duration_ms: Option<f64>,
        /// Cumulative cost of the session so far.
        cost_usd: Option<f64>,
        input_tokens: Option<f64>,
        output_tokens: Option<f64>,
    },
    ModeChanged {
        mode: PermissionMode,
    },
    ModelChanged {
        model: String,
    },
    Notice {
        level: NoticeLevel,
        text: String,
    },
    Exited {
        code: Option<i32>,
    },
}

/// Everything a chat has emitted, so a re-attached view can rebuild itself.
/// Consecutive deltas of one block are merged to keep it small.
#[derive(Debug, Default)]
pub struct Transcript {
    events: Vec<ChatEvent>,
}

impl Transcript {
    pub fn push(&mut self, event: ChatEvent) {
        if let ChatEvent::BlockDelta {
            message_id,
            index,
            text,
        } = &event
            && let Some(ChatEvent::BlockDelta {
                message_id: last_id,
                index: last_index,
                text: last_text,
            }) = self.events.last_mut()
            && last_id == message_id
            && last_index == index
        {
            last_text.push_str(text);
            return;
        }
        self.events.push(event);
    }

    pub fn events(&self) -> &[ChatEvent] {
        &self.events
    }
}

impl ChatInfo {
    /// Folds an event into the summary shown in tabs.
    pub fn apply(&mut self, event: &ChatEvent) {
        match event {
            ChatEvent::Session {
                session_id,
                model,
                permission_mode,
            } => {
                self.session_id = Some(session_id.clone());
                if model.is_some() {
                    self.model = model.clone();
                }
                if let Some(mode) = permission_mode {
                    self.permission_mode = *mode;
                }
            }
            ChatEvent::User { .. } => self.busy = true,
            ChatEvent::TurnComplete { .. } => self.busy = false,
            ChatEvent::ModeChanged { mode } => self.permission_mode = *mode,
            ChatEvent::ModelChanged { model } => self.model = Some(model.clone()),
            ChatEvent::Exited { .. } => {
                self.running = false;
                self.busy = false;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn delta(id: &str, index: u32, text: &str) -> ChatEvent {
        ChatEvent::BlockDelta {
            message_id: id.into(),
            index,
            text: text.into(),
        }
    }

    #[test]
    fn transcript_merges_consecutive_deltas_of_a_block() {
        let mut t = Transcript::default();
        t.push(delta("m", 0, "Hel"));
        t.push(delta("m", 0, "lo"));
        t.push(delta("m", 1, "x"));
        t.push(delta("m", 0, "!"));
        assert_eq!(
            t.events(),
            [
                delta("m", 0, "Hello"),
                delta("m", 1, "x"),
                delta("m", 0, "!")
            ]
        );
    }

    #[test]
    fn info_tracks_session_turns_and_exit() {
        let mut info = ChatInfo {
            id: "c1".into(),
            worktree_id: "wt".into(),
            profile_id: "claude".into(),
            agent: AgentKind::Claude,
            title: "Claude Code".into(),
            running: true,
            busy: false,
            session_id: None,
            model: None,
            permission_mode: PermissionMode::Default,
        };
        info.apply(&ChatEvent::Session {
            session_id: "s".into(),
            model: Some("opus".into()),
            permission_mode: Some(PermissionMode::Plan),
        });
        info.apply(&ChatEvent::User {
            id: "u".into(),
            text: "hi".into(),
        });
        assert!(info.busy);
        assert_eq!(info.session_id.as_deref(), Some("s"));
        assert_eq!(info.permission_mode, PermissionMode::Plan);
        info.apply(&ChatEvent::Exited { code: Some(0) });
        assert!(!info.running && !info.busy);
    }

    #[test]
    fn manual_mode_reads_as_default() {
        let mode: PermissionMode = serde_json::from_str("\"manual\"").unwrap();
        assert_eq!(mode, PermissionMode::Default);
    }
}
