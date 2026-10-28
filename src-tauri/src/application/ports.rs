//! Ports: traits owned by the application layer and implemented by
//! infrastructure adapters.

use std::path::{Path, PathBuf};

use crate::domain::workspace::Session;

use super::Result;

/// Static facts about the running build.
pub trait BuildInfo: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
}

/// One entry of `git worktree list`. Paths are canonical.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitWorktreeInfo {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub is_main: bool,
}

/// Git worktree operations.
pub trait GitWorktrees: Send + Sync {
    /// Top level of the worktree containing `path`.
    fn toplevel(&self, path: &Path) -> Result<PathBuf>;
    /// All worktrees of the repository containing `repo`; the main one first.
    fn list(&self, repo: &Path) -> Result<Vec<GitWorktreeInfo>>;
    /// Local branch names.
    fn branches(&self, repo: &Path) -> Result<Vec<String>>;
    /// Adds a worktree at `path`. Creates `branch` from `base` when `create_branch` is set.
    fn add(
        &self,
        repo: &Path,
        path: &Path,
        branch: &str,
        base: Option<&str>,
        create_branch: bool,
    ) -> Result<()>;
    fn remove(&self, repo: &Path, path: &Path, force: bool) -> Result<()>;
}

/// Persistence for the tab session. `update` must be atomic.
pub trait SessionRepository: Send + Sync {
    fn get(&self) -> Result<Session>;
    fn update(&self, f: &mut dyn FnMut(&mut Session) -> Result<()>) -> Result<()>;
}

// Indexing

use crate::domain::parsed::ParsedFile;

/// A source file found in a worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    /// Repo-relative path with `/` separators.
    pub path: String,
    /// Workspace package the file belongs to.
    pub package: String,
}

/// Lists indexable source files (respecting .gitignore) and reads them.
pub trait SourceScanner: Send + Sync {
    fn scan(&self, root: &Path) -> Result<Vec<SourceFile>>;
    /// Package for a single path, used when files appear after the initial scan.
    fn package_of(&self, root: &Path, path: &str) -> String;
    fn is_source(&self, path: &str) -> bool;
    fn read(&self, root: &Path, path: &str) -> Result<String>;
}

/// Turns one source file into symbols, imports and type references.
pub trait SourceParser: Send + Sync {
    /// Bumped whenever extraction changes so caches are invalidated.
    fn version(&self) -> u32;
    fn parse(&self, path: &str, package: &str, source: &str) -> Result<ParsedFile>;
}

/// Resolves import specifiers to repo-relative files within a worktree.
pub trait ModuleResolver: Send + Sync {
    fn resolve(&self, from: &str, specifier: &str) -> Option<String>;
}

/// Builds a resolver for a worktree (reads tsconfig paths, workspace packages, ...).
pub trait ModuleResolverFactory: Send + Sync {
    fn for_root(&self, root: &Path) -> Box<dyn ModuleResolver>;
}

/// Content-addressed cache of parse results, so reopening a worktree is fast.
pub trait ParseCache: Send + Sync {
    fn get(&self, worktree: &str, path: &str, hash: &str) -> Option<ParsedFile>;
    fn put_many(&self, worktree: &str, entries: &[(String, ParsedFile)]);
    fn forget(&self, worktree: &str, paths: &[String]);
}

/// Watches worktrees for source changes; changes are reported out-of-band.
pub trait FileWatcher: Send + Sync {
    fn watch(&self, worktree: &str, root: &Path) -> Result<()>;
    fn unwatch(&self, worktree: &str);
}

/// Progress and change notifications for the UI.
pub trait IndexEvents: Send + Sync {
    fn progress(&self, worktree: &str, done: usize, total: usize);
    /// The model changed; `changed` lists symbol ids that were added or modified.
    fn updated(&self, worktree: &str, changed: &[String], removed: &[String]);
}

// Terminals

/// What to run in a PTY.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnSpec {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
    pub cols: u16,
    pub rows: u16,
}

/// Control side of a running PTY process.
pub trait PtyControl: Send {
    fn write(&mut self, data: &[u8]) -> Result<()>;
    fn resize(&mut self, cols: u16, rows: u16) -> Result<()>;
    fn kill(&mut self);
    /// Blocks until the process exits.
    fn wait(&mut self) -> Option<i32>;
}

pub struct SpawnedPty {
    pub output: Box<dyn std::io::Read + Send>,
    pub control: Box<dyn PtyControl>,
}

pub trait PtySpawner: Send + Sync {
    fn spawn(&self, spec: SpawnSpec) -> Result<SpawnedPty>;
}

/// The user's login-shell environment (GUI apps on macOS don't inherit it).
pub trait ShellEnvironment: Send + Sync {
    fn shell(&self) -> PathBuf;
    /// Resolves a program on the login PATH.
    fn find(&self, program: &str) -> Option<PathBuf>;
    /// Environment for spawned agents.
    fn env(&self) -> Vec<(String, String)>;
}

/// Where a terminal's output goes; swapped when a view re-attaches.
pub trait TerminalSink: Send + Sync {
    fn output(&self, bytes: &[u8]);
    fn exited(&self, code: Option<i32>);
}

// Patterns

/// The user's confirm/dismiss verdicts on detected patterns, per project.
pub trait PatternDecisions: Send + Sync {
    fn load(
        &self,
        project: &str,
    ) -> std::collections::HashMap<String, crate::domain::patterns::PatternDecision>;
    fn set(
        &self,
        project: &str,
        pattern_id: &str,
        decision: Option<crate::domain::patterns::PatternDecision>,
    ) -> Result<()>;
}

// Review comments and settings

/// Review comments, stored per worktree.
pub trait CommentStore: Send + Sync {
    fn load(&self, worktree: &str) -> Vec<crate::domain::review::ReviewComment>;
    fn save(&self, worktree: &str, comments: &[crate::domain::review::ReviewComment])
    -> Result<()>;
}

/// App-wide user settings.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct Settings {
    /// Command to open a file at a line. `{file}` and `{line}` are substituted.
    pub editor_command: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            editor_command: "nvim +{line} {file}".into(),
        }
    }
}

pub trait SettingsRepository: Send + Sync {
    fn get(&self) -> Settings;
    fn set(&self, settings: Settings) -> Result<()>;
}

/// Starts a GUI program detached from CRM.
pub trait DetachedLauncher: Send + Sync {
    fn launch(&self, command_line: &str, cwd: &Path, env: &[(String, String)]) -> Result<()>;
}
