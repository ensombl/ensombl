//! Writing review comments, updating settings and opening files in an editor.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::application::cqrs::{Command, CommandHandler};
use crate::application::ports::{
    CommentStore, DetachedLauncher, SessionRepository, Settings, SettingsRepository,
    ShellEnvironment, TerminalSink,
};
use crate::application::worktrees::{safe_join, worktree_root};
use crate::application::{Error, Result};
use crate::domain::review::ReviewComment;
use crate::domain::terminal::TerminalInfo;
use crate::domain::workspace::WorktreeId;

use super::terminals::{StartCommand, TerminalCommands, WithSink};

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AddComment {
    pub worktree_id: WorktreeId,
    pub file: String,
    pub line: u32,
    pub body: String,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ResolveComment {
    pub worktree_id: WorktreeId,
    pub comment_id: String,
    pub resolved: bool,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeleteComment {
    pub worktree_id: WorktreeId,
    pub comment_id: String,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateSettings {
    pub settings: Settings,
}

/// Opens a file at a line with the configured editor.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OpenInEditor {
    pub worktree_id: WorktreeId,
    pub path: String,
    pub line: u32,
}

/// Saves a file edited in the Code tab.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SaveSource {
    pub worktree_id: WorktreeId,
    pub path: String,
    pub content: String,
    /// What the editor loaded; the save is refused if the file has changed
    /// since (an agent edited it). `None` overwrites.
    #[ts(optional)]
    pub base: Option<String>,
}

/// Terminal editors open in an agent tab; GUI editors are launched detached.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "type")]
#[ts(export)]
pub enum EditorLaunch {
    Terminal { terminal: TerminalInfo },
    Detached,
}

impl Command for AddComment {
    type Output = ReviewComment;
}
impl Command for ResolveComment {
    type Output = ();
}
impl Command for DeleteComment {
    type Output = ();
}
impl Command for UpdateSettings {
    type Output = ();
}
impl Command for OpenInEditor {
    type Output = EditorLaunch;
}
impl Command for SaveSource {
    type Output = ();
}

/// Refuses to overwrite a file that changed under the editor.
fn check_base(path: &str, current: &str, base: Option<&str>) -> Result<()> {
    match base {
        Some(base) if base != current => Err(Error::Invalid(format!(
            "{path} changed on disk since you opened it"
        ))),
        _ => Ok(()),
    }
}

const GUI_EDITORS: &[&str] = &[
    "code", "cursor", "zed", "subl", "idea", "webstorm", "windsurf", "open", "fleet",
];

/// Quotes a value for a POSIX shell command line.
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

pub fn editor_command_line(template: &str, file: &str, line: u32) -> String {
    template
        .replace("{file}", &shell_quote(file))
        .replace("{line}", &line.to_string())
}

fn is_gui_editor(command_line: &str) -> bool {
    let program = command_line.split_whitespace().next().unwrap_or("");
    let name = program.rsplit('/').next().unwrap_or(program);
    GUI_EDITORS.contains(&name)
}

struct NullSink;
impl TerminalSink for NullSink {
    fn output(&self, _: &[u8]) {}
    fn exited(&self, _: Option<i32>) {}
}

pub struct ReviewCommands {
    sessions: Arc<dyn SessionRepository>,
    comments: Arc<dyn CommentStore>,
    settings: Arc<dyn SettingsRepository>,
    shell: Arc<dyn ShellEnvironment>,
    launcher: Arc<dyn DetachedLauncher>,
    terminals: Arc<TerminalCommands>,
}

impl ReviewCommands {
    pub fn new(
        sessions: Arc<dyn SessionRepository>,
        comments: Arc<dyn CommentStore>,
        settings: Arc<dyn SettingsRepository>,
        shell: Arc<dyn ShellEnvironment>,
        launcher: Arc<dyn DetachedLauncher>,
        terminals: Arc<TerminalCommands>,
    ) -> Self {
        Self {
            sessions,
            comments,
            settings,
            shell,
            launcher,
            terminals,
        }
    }

    fn edit(
        &self,
        worktree: &str,
        f: impl FnOnce(&mut Vec<ReviewComment>) -> Result<()>,
    ) -> Result<()> {
        let mut comments = self.comments.load(worktree);
        f(&mut comments)?;
        self.comments.save(worktree, &comments)
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl CommandHandler<AddComment> for ReviewCommands {
    fn handle(&self, cmd: AddComment) -> Result<ReviewComment> {
        let body = cmd.body.trim();
        if body.is_empty() {
            return Err(Error::Invalid("comment is empty".into()));
        }
        let root = worktree_root(self.sessions.as_ref(), &cmd.worktree_id)?;
        let source = std::fs::read_to_string(safe_join(&root, &cmd.file)?)?;
        let anchor = source
            .lines()
            .nth(cmd.line.saturating_sub(1) as usize)
            .ok_or_else(|| Error::Invalid(format!("{} has no line {}", cmd.file, cmd.line)))?
            .trim()
            .to_string();
        let created_at = now_millis();
        let comment = ReviewComment {
            id: format!("c{created_at:x}{:x}", cmd.line),
            file: cmd.file,
            line: cmd.line,
            body: body.into(),
            anchor,
            resolved: false,
            created_at,
        };
        let saved = comment.clone();
        self.edit(&cmd.worktree_id, |all| {
            all.push(saved);
            Ok(())
        })?;
        Ok(comment)
    }
}

impl CommandHandler<ResolveComment> for ReviewCommands {
    fn handle(&self, cmd: ResolveComment) -> Result<()> {
        self.edit(&cmd.worktree_id, |all| {
            let c = all
                .iter_mut()
                .find(|c| c.id == cmd.comment_id)
                .ok_or_else(|| Error::Invalid("no such comment".into()))?;
            c.resolved = cmd.resolved;
            Ok(())
        })
    }
}

impl CommandHandler<DeleteComment> for ReviewCommands {
    fn handle(&self, cmd: DeleteComment) -> Result<()> {
        self.edit(&cmd.worktree_id, |all| {
            all.retain(|c| c.id != cmd.comment_id);
            Ok(())
        })
    }
}

impl CommandHandler<UpdateSettings> for ReviewCommands {
    fn handle(&self, cmd: UpdateSettings) -> Result<()> {
        if !cmd.settings.editor_command.contains("{file}") {
            return Err(Error::Invalid(
                "the editor command must contain {file}".into(),
            ));
        }
        self.settings.set(cmd.settings)
    }
}

impl CommandHandler<OpenInEditor> for ReviewCommands {
    fn handle(&self, cmd: OpenInEditor) -> Result<EditorLaunch> {
        let root = worktree_root(self.sessions.as_ref(), &cmd.worktree_id)?;
        let file = safe_join(&root, &cmd.path)?;
        let command_line = editor_command_line(
            &self.settings.get().editor_command,
            &file.to_string_lossy(),
            cmd.line,
        );
        if is_gui_editor(&command_line) {
            self.launcher
                .launch(&command_line, &root, &self.shell.env())?;
            return Ok(EditorLaunch::Detached);
        }
        let title = cmd.path.rsplit('/').next().unwrap_or(&cmd.path).to_string();
        let terminal = self.terminals.handle(WithSink {
            command: StartCommand {
                worktree_id: cmd.worktree_id,
                command_line,
                title,
                cols: 100,
                rows: 30,
            },
            sink: Arc::new(NullSink),
        })?;
        Ok(EditorLaunch::Terminal { terminal })
    }
}

impl CommandHandler<SaveSource> for ReviewCommands {
    fn handle(&self, cmd: SaveSource) -> Result<()> {
        let root = worktree_root(self.sessions.as_ref(), &cmd.worktree_id)?;
        let file = safe_join(&root, &cmd.path)?;
        let current = std::fs::read_to_string(&file)?;
        check_base(&cmd.path, &current, cmd.base.as_deref())?;
        std::fs::write(file, cmd.content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_saves_over_changes_made_on_disk() {
        assert!(check_base("a.ts", "x", Some("x")).is_ok());
        assert!(check_base("a.ts", "y", None).is_ok());
        assert!(check_base("a.ts", "y", Some("x")).is_err());
    }

    #[test]
    fn builds_editor_command_lines() {
        assert_eq!(
            editor_command_line("nvim +{line} {file}", "/r/a b.ts", 12),
            "nvim +12 '/r/a b.ts'"
        );
        assert_eq!(
            editor_command_line("code -g {file}:{line}", "/r/it's.ts", 3),
            r"code -g '/r/it'\''s.ts':3"
        );
    }

    #[test]
    fn recognises_gui_editors() {
        assert!(is_gui_editor("code -g '/a':1"));
        assert!(is_gui_editor("/usr/local/bin/cursor -g x"));
        assert!(!is_gui_editor("nvim +1 x"));
        assert!(!is_gui_editor("hx x:1"));
    }
}
