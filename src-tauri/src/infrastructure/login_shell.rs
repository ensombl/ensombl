//! [`ShellEnvironment`] from the user's login shell. macOS GUI apps start
//! with a minimal PATH, so we ask the login shell once for the real one
//! (picking up nvm, Homebrew, ~/.local/bin and so on).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use crate::application::ports::ShellEnvironment;

const MARKER: &str = "__CRM_PATH__";

/// Set by a parent Claude Code session (e.g. when CRM itself was launched from
/// one); inherited, they make agents think they're nested and change behavior.
fn is_agent_session_marker(key: &str) -> bool {
    key == "CLAUDECODE"
        || key == "CLAUDE_PID"
        || key == "CLAUDE_EFFORT"
        || key.starts_with("CLAUDE_CODE_")
}

pub struct LoginShell {
    shell: PathBuf,
    path: String,
}

impl LoginShell {
    pub fn detect() -> Self {
        let shell = std::env::var("SHELL")
            .map(PathBuf::from)
            .unwrap_or_else(|_| "/bin/zsh".into());
        let fallback = std::env::var("PATH").unwrap_or_default();
        let path = Self::login_path(&shell).unwrap_or(fallback);
        Self { shell, path }
    }

    fn login_path(shell: &Path) -> Option<String> {
        let shell = shell.to_path_buf();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let out = Command::new(&shell)
                .args(["-ilc", &format!("printf '{MARKER}%s{MARKER}' \"$PATH\"")])
                .stdin(Stdio::null())
                .stderr(Stdio::null())
                .output();
            let _ = tx.send(out);
        });
        // A slow or interactive rc file must not hang startup.
        let out = rx.recv_timeout(Duration::from_secs(5)).ok()?.ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        let start = text.find(MARKER)? + MARKER.len();
        let end = start + text[start..].find(MARKER)?;
        Some(text[start..end].to_string()).filter(|p| !p.is_empty())
    }

    #[cfg(test)]
    pub fn with_path(shell: PathBuf, path: String) -> Self {
        Self { shell, path }
    }
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

impl ShellEnvironment for LoginShell {
    fn shell(&self) -> PathBuf {
        self.shell.clone()
    }

    fn find(&self, program: &str) -> Option<PathBuf> {
        let candidate = Path::new(program);
        if candidate.is_absolute() {
            return is_executable(candidate).then(|| candidate.to_path_buf());
        }
        std::env::split_paths(&self.path)
            .map(|dir| dir.join(program))
            .find(|p| is_executable(p))
    }

    fn env(&self) -> Vec<(String, String)> {
        let mut env: Vec<(String, String)> = std::env::vars()
            .filter(|(k, _)| {
                !matches!(k.as_str(), "PATH" | "TERM" | "COLORTERM" | "TERM_PROGRAM")
                    && !is_agent_session_marker(k)
            })
            .collect();
        env.push(("PATH".into(), self.path.clone()));
        env.push(("TERM".into(), "xterm-256color".into()));
        env.push(("COLORTERM".into(), "truecolor".into()));
        env.push(("TERM_PROGRAM".into(), "CRM".into()));
        if !env.iter().any(|(k, _)| k == "LANG") {
            env.push(("LANG".into(), "en_US.UTF-8".into()));
        }
        env
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn finds_programs_on_the_login_path() {
        let shell = LoginShell::with_path("/bin/sh".into(), "/nonexistent:/bin".into());
        assert_eq!(shell.find("sh"), Some(PathBuf::from("/bin/sh")));
        assert_eq!(shell.find("definitely-not-a-cli"), None);
        assert_eq!(shell.find("/bin/sh"), Some(PathBuf::from("/bin/sh")));
    }

    #[test]
    fn env_sets_terminal_variables() {
        let env = LoginShell::with_path("/bin/sh".into(), "/x".into()).env();
        let get = |k: &str| {
            env.iter()
                .find(|(key, _)| key == k)
                .map(|(_, v)| v.as_str())
        };
        assert_eq!(get("PATH"), Some("/x"));
        assert_eq!(get("TERM"), Some("xterm-256color"));
    }

    #[test]
    fn parent_agent_markers_are_not_inherited() {
        assert!(is_agent_session_marker("CLAUDE_CODE_CHILD_SESSION"));
        assert!(is_agent_session_marker("CLAUDECODE"));
        assert!(!is_agent_session_marker("CLAUDE_CONFIG_DIR"));
        assert!(!is_agent_session_marker("HOME"));
    }
}
