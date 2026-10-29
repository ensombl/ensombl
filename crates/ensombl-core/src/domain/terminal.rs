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
    /// Extra args that start it as the lead, when this CLI can lead.
    pub lead_args: Option<Vec<String>>,
}

/// What the lead agent is told: plan, delegate through Ensombl, review, merge.
pub const LEAD_INSTRUCTIONS: &str = "You are the lead engineer in Ensombl. Other agents do the implementation: use the ensombl tools to split the user's request into well-scoped tasks (create_task, with depends_on when one needs another merged first), wait for them, read their summaries, activity and diffs, ask for changes with send_follow_up, and approve_task to merge work you're happy with. Answer tasks waiting on a permission with answer_task when the request is safe, otherwise ask the user. Keep the user informed, and only edit code yourself for trivial fixes.";

/// How a lead reaches Ensombl's MCP bridge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeadMcp {
    /// A Claude-style MCP config file naming the bridge.
    pub config: String,
    /// The bridge itself, for CLIs configured by flags.
    pub command: String,
    pub args: Vec<String>,
}

/// A JSON string or array, which is also valid TOML.
fn toml(value: impl Into<serde_json::Value>) -> String {
    value.into().to_string()
}

impl AgentProfile {
    /// What makes this CLI a lead: Ensombl's MCP bridge and the lead's
    /// instructions. `None` for CLIs that can't lead.
    pub fn lead_args_for(kind: AgentKind, mcp: &LeadMcp) -> Option<Vec<String>> {
        match kind {
            AgentKind::Claude => Some(vec![
                "--mcp-config".into(),
                mcp.config.clone(),
                "--allowedTools".into(),
                "mcp__ensombl".into(),
                "--append-system-prompt".into(),
                LEAD_INSTRUCTIONS.into(),
            ]),
            AgentKind::Codex => Some(vec![
                "-c".into(),
                format!("mcp_servers.ensombl.command={}", toml(mcp.command.as_str())),
                "-c".into(),
                format!("mcp_servers.ensombl.args={}", toml(mcp.args.clone())),
                // Ensombl's tools run without asking, as for the Claude lead.
                "-c".into(),
                r#"mcp_servers.ensombl.default_tools_approval_mode="approve""#.into(),
                "-c".into(),
                format!("developer_instructions={}", toml(LEAD_INSTRUCTIONS)),
            ]),
            AgentKind::Shell | AgentKind::Custom => None,
        }
    }

    /// The built-in profiles; with the MCP bridge, those that can lead say how.
    pub fn builtin_with(lead: Option<&LeadMcp>) -> Vec<AgentProfile> {
        let mut profiles = Self::builtin();
        if let Some(mcp) = lead {
            for p in &mut profiles {
                p.lead_args = Self::lead_args_for(p.kind, mcp);
            }
        }
        profiles
    }

    pub fn builtin() -> Vec<AgentProfile> {
        let profile = |id: &str, label: &str, kind, command: &str| AgentProfile {
            id: id.into(),
            label: label.into(),
            kind,
            command: command.into(),
            args: vec![],
            lead_args: None,
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
    /// Started as the lead.
    pub lead: bool,
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
    fn claude_and_codex_can_lead_with_ensombls_tools() {
        let mcp = LeadMcp {
            config: "/data/mcp.json".into(),
            command: "/app/Ensombl".into(),
            args: vec!["--mcp".into(), "--api-file".into(), "/data/api.json".into()],
        };
        let profiles = AgentProfile::builtin_with(Some(&mcp));
        let lead = |id: &str| {
            profiles
                .iter()
                .find(|p| p.id == id)
                .unwrap()
                .lead_args
                .clone()
        };
        assert_eq!(
            &lead("claude").unwrap()[..4],
            [
                "--mcp-config",
                "/data/mcp.json",
                "--allowedTools",
                "mcp__ensombl"
            ]
        );
        assert_eq!(
            &lead("codex").unwrap()[..4],
            [
                "-c",
                r#"mcp_servers.ensombl.command="/app/Ensombl""#,
                "-c",
                r#"mcp_servers.ensombl.args=["--mcp","--api-file","/data/api.json"]"#
            ]
        );
        assert_eq!(lead("shell"), None);
        assert!(
            AgentProfile::builtin_with(None)
                .iter()
                .all(|p| p.lead_args.is_none())
        );
    }

    #[test]
    fn builtin_profiles_include_claude_and_codex() {
        let ids: Vec<_> = AgentProfile::builtin().into_iter().map(|p| p.id).collect();
        assert_eq!(ids, ["claude", "codex", "shell"]);
    }
}
