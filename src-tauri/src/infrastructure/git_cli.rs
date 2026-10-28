//! [`GitWorktrees`] backed by the `git` command line.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::application::ports::{GitWorktreeInfo, GitWorktrees};
use crate::application::{Error, Result};

pub struct GitCli;

fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|e| Error::Git(format!("failed to run git: {e}")))?;
    if !out.status.success() {
        return Err(Error::Git(String::from_utf8_lossy(&out.stderr).trim().to_string()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn canonical(path: &str) -> PathBuf {
    let p = PathBuf::from(path);
    p.canonicalize().unwrap_or(p)
}

/// Parses `git worktree list --porcelain`. The first entry is the main worktree.
fn parse_porcelain(out: &str) -> Vec<GitWorktreeInfo> {
    out.split("\n\n")
        .filter_map(|block| {
            let mut path = None;
            let mut branch = None;
            for line in block.lines() {
                if let Some(p) = line.strip_prefix("worktree ") {
                    path = Some(canonical(p));
                } else if let Some(b) = line.strip_prefix("branch ") {
                    branch = Some(b.trim_start_matches("refs/heads/").to_string());
                } else if line == "bare" {
                    return None;
                }
            }
            path.map(|path| GitWorktreeInfo { path, branch, is_main: false })
        })
        .enumerate()
        .map(|(i, mut w)| {
            w.is_main = i == 0;
            w
        })
        .collect()
}

impl GitWorktrees for GitCli {
    fn toplevel(&self, path: &Path) -> Result<PathBuf> {
        let out = git(path, &["rev-parse", "--show-toplevel"])
            .map_err(|_| Error::Invalid(format!("{} is not inside a git repository", path.display())))?;
        Ok(canonical(out.trim()))
    }

    fn list(&self, repo: &Path) -> Result<Vec<GitWorktreeInfo>> {
        Ok(parse_porcelain(&git(repo, &["worktree", "list", "--porcelain"])?))
    }

    fn branches(&self, repo: &Path) -> Result<Vec<String>> {
        let out = git(repo, &["for-each-ref", "--format=%(refname:short)", "refs/heads"])?;
        Ok(out.lines().map(str::to_string).collect())
    }

    fn add(&self, repo: &Path, path: &Path, branch: &str, base: Option<&str>, create_branch: bool) -> Result<()> {
        let path = path.to_string_lossy();
        let mut args = vec!["worktree", "add"];
        if create_branch {
            args.extend(["-b", branch, &path]);
            args.extend(base);
        } else {
            args.extend([&*path, branch]);
        }
        git(repo, &args).map(drop)
    }

    fn remove(&self, repo: &Path, path: &Path, force: bool) -> Result<()> {
        let path = path.to_string_lossy();
        let mut args = vec!["worktree", "remove"];
        if force {
            args.push("--force");
        }
        args.push(&path);
        git(repo, &args).map(drop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_porcelain_output() {
        let out = "worktree /a\nHEAD 1\nbranch refs/heads/main\n\nworktree /b\nHEAD 2\ndetached\n\n";
        let list = parse_porcelain(out);
        assert_eq!(list.len(), 2);
        assert!(list[0].is_main);
        assert_eq!(list[0].branch.as_deref(), Some("main"));
        assert_eq!(list[1].branch, None);
        assert!(!list[1].is_main);
    }

    fn init_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        git(p, &["init", "-q", "-b", "main"]).unwrap();
        std::fs::write(p.join("a.txt"), "a").unwrap();
        git(p, &["add", "."]).unwrap();
        git(p, &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "init"]).unwrap();
        dir
    }

    #[test]
    fn adds_lists_and_removes_worktrees() {
        let repo = init_repo();
        let wt_parent = tempfile::tempdir().unwrap();
        let wt = wt_parent.path().join("feature");
        let git = GitCli;

        git.add(repo.path(), &wt, "feature", None, true).unwrap();
        let list = git.list(repo.path()).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].path, repo.path().canonicalize().unwrap());
        assert_eq!(list[1].branch.as_deref(), Some("feature"));
        assert_eq!(git.toplevel(&wt).unwrap(), wt.canonicalize().unwrap());
        assert!(git.branches(repo.path()).unwrap().contains(&"feature".to_string()));

        git.remove(repo.path(), &wt, false).unwrap();
        assert_eq!(git.list(repo.path()).unwrap().len(), 1);
    }

    #[test]
    fn toplevel_outside_a_repo_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(GitCli.toplevel(dir.path()).is_err());
    }
}
