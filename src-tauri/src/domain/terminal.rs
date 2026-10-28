//! Agent terminals: a CLI (claude, codex, a shell, ...) running in a PTY
//! inside one worktree.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::workspace::WorktreeId;

pub type TerminalId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum AgentKind {
    Claude,
    Codex,
    Shell,
    Custom,
}

/// Whether an agent runs as a native chat or in a terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum AgentMode {
    Chat,
    Terminal,
}

/// How to launch an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AgentProfile {
    pub id: String,
    pub label: String,
    pub kind: AgentKind,
    pub mode: AgentMode,
    /// Program name looked up on the login shell's PATH (or an absolute path).
    pub command: String,
    pub args: Vec<String>,
}

impl AgentProfile {
    pub fn builtin() -> Vec<AgentProfile> {
        let profile = |id: &str, label: &str, kind, mode, command: &str| AgentProfile {
            id: id.into(),
            label: label.into(),
            kind,
            mode,
            command: command.into(),
            args: vec![],
        };
        use AgentMode::{Chat, Terminal};
        vec![
            profile("claude", "Claude Code", AgentKind::Claude, Chat, "claude"),
            profile("codex", "Codex", AgentKind::Codex, Terminal, "codex"),
            profile(
                "claude-tui",
                "Claude Code (terminal)",
                AgentKind::Claude,
                Terminal,
                "claude",
            ),
            profile("shell", "Shell", AgentKind::Shell, Terminal, "$SHELL"),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TerminalInfo {
    pub id: TerminalId,
    pub worktree_id: WorktreeId,
    pub profile_id: String,
    pub title: String,
    pub running: bool,
    pub exit_code: Option<i32>,
}

/// Keeps the most recent output so a re-attached view can replay it.
#[derive(Debug, Default)]
pub struct Scrollback {
    buf: std::collections::VecDeque<u8>,
}

impl Scrollback {
    pub const CAPACITY: usize = 1024 * 1024;

    pub fn push(&mut self, bytes: &[u8]) {
        self.buf.extend(bytes);
        let excess = self.buf.len().saturating_sub(Self::CAPACITY);
        if excess > 0 {
            self.buf.drain(..excess);
        }
    }

    pub fn snapshot(&self) -> Vec<u8> {
        self.buf.iter().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrollback_keeps_the_most_recent_bytes() {
        let mut s = Scrollback::default();
        s.push(&vec![b'a'; Scrollback::CAPACITY]);
        s.push(b"xyz");
        let snap = s.snapshot();
        assert_eq!(snap.len(), Scrollback::CAPACITY);
        assert!(snap.ends_with(b"xyz"));
    }

    #[test]
    fn builtin_profiles_include_claude_and_codex() {
        let ids: Vec<_> = AgentProfile::builtin().into_iter().map(|p| p.id).collect();
        assert_eq!(ids, ["claude", "codex", "claude-tui", "shell"]);
    }
}
