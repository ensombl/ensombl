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

/// How to launch an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AgentProfile {
    pub id: String,
    pub label: String,
    pub kind: AgentKind,
    /// Program name looked up on the login shell's PATH (or an absolute path).
    pub command: String,
    pub args: Vec<String>,
}

/// What the lead agent is told: plan, delegate through Ensombl, review, merge.
pub const LEAD_INSTRUCTIONS: &str = "You are the lead engineer in Ensombl. Other agents do the implementation: use the ensombl tools to split the user's request into well-scoped tasks (create_task, with depends_on when one needs another merged first), wait for them, read their summaries, activity and diffs, ask for changes with send_follow_up, and approve_task to merge work you're happy with. Answer tasks waiting on a permission with answer_task when the request is safe, otherwise ask the user. Keep the user informed, and only edit code yourself for trivial fixes.";

impl AgentProfile {
    /// An interactive Claude that orchestrates through Ensombl's MCP bridge,
    /// described by the config file at `mcp_config`.
    pub fn lead(mcp_config: &str) -> AgentProfile {
        AgentProfile {
            id: "lead".into(),
            label: "Lead (orchestrates tasks)".into(),
            kind: AgentKind::Claude,
            command: "claude".into(),
            args: vec![
                "--mcp-config".into(),
                mcp_config.into(),
                "--allowedTools".into(),
                "mcp__ensombl".into(),
                "--append-system-prompt".into(),
                LEAD_INSTRUCTIONS.into(),
            ],
        }
    }

    pub fn builtin() -> Vec<AgentProfile> {
        let profile = |id: &str, label: &str, kind, command: &str| AgentProfile {
            id: id.into(),
            label: label.into(),
            kind,
            command: command.into(),
            args: vec![],
        };
        vec![
            profile("claude", "Claude Code", AgentKind::Claude, "claude"),
            profile("codex", "Codex", AgentKind::Codex, "codex"),
            profile("shell", "Shell", AgentKind::Shell, "$SHELL"),
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
    fn the_lead_uses_ensombls_tools() {
        let lead = AgentProfile::lead("/data/mcp.json");
        assert_eq!(
            &lead.args[..4],
            [
                "--mcp-config",
                "/data/mcp.json",
                "--allowedTools",
                "mcp__ensombl"
            ]
        );
    }

    #[test]
    fn builtin_profiles_include_claude_and_codex() {
        let ids: Vec<_> = AgentProfile::builtin().into_iter().map(|p| p.id).collect();
        assert_eq!(ids, ["claude", "codex", "shell"]);
    }
}
