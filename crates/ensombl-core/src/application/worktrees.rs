//! Helpers for resolving open worktrees and paths inside them.

use std::path::{Component, Path, PathBuf};

use super::ports::SessionRepository;
use super::{Error, Result};

/// Root directory of an open worktree tab.
pub fn worktree_root(sessions: &dyn SessionRepository, worktree_id: &str) -> Result<PathBuf> {
    sessions
        .get()?
        .tabs
        .iter()
        .find(|t| t.id == worktree_id)
        .map(|t| PathBuf::from(&t.path))
        .ok_or_else(|| Error::Invalid(format!("worktree {worktree_id} isn't open")))
}

/// Joins a repo-relative path onto `root`, refusing anything that escapes it.
pub fn safe_join(root: &Path, rel: &str) -> Result<PathBuf> {
    let rel = Path::new(rel);
    if rel.is_absolute() || rel.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(Error::Invalid(format!(
            "{} is outside the worktree",
            rel.display()
        )));
    }
    Ok(root.join(rel))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_paths_outside_the_worktree() {
        let root = Path::new("/repo");
        assert_eq!(
            safe_join(root, "src/a.ts").unwrap(),
            PathBuf::from("/repo/src/a.ts")
        );
        assert!(safe_join(root, "../etc/passwd").is_err());
        assert!(safe_join(root, "/etc/passwd").is_err());
    }
}
