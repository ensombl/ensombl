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
    fn add(&self, repo: &Path, path: &Path, branch: &str, base: Option<&str>, create_branch: bool) -> Result<()>;
    fn remove(&self, repo: &Path, path: &Path, force: bool) -> Result<()>;
}

/// Persistence for the tab session. `update` must be atomic.
pub trait SessionRepository: Send + Sync {
    fn get(&self) -> Result<Session>;
    fn update(&self, f: &mut dyn FnMut(&mut Session) -> Result<()>) -> Result<()>;
}
