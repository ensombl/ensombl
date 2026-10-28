//! [`GitWorktrees`] and [`TaskGit`] backed by the `git` command line.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::application::ports::{GitWorktreeInfo, GitWorktrees, TaskGit};
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
        return Err(Error::Git(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Runs git and returns its exit code and stdout, for commands whose
/// non-zero exit is an answer rather than a failure.
fn git_code(dir: &Path, args: &[&str]) -> Result<(i32, String)> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|e| Error::Git(format!("failed to run git: {e}")))?;
    Ok((
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    ))
}

/// Commits need an identity; a server or fresh machine may have none, so
/// fall back to one naming the app.
fn identity(dir: &Path) -> Vec<&'static str> {
    match git_code(dir, &["config", "user.email"]) {
        Ok((0, email)) if !email.trim().is_empty() => vec![],
        _ => vec![
            "-c",
            "user.name=Ensombl",
            "-c",
            "user.email=ensombl@localhost",
        ],
    }
}

fn with_identity<'a>(dir: &Path, args: &[&'a str]) -> Vec<&'a str> {
    let mut all = identity(dir);
    all.extend_from_slice(args);
    all
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
            path.map(|path| GitWorktreeInfo {
                path,
                branch,
                is_main: false,
            })
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
        let out = git(path, &["rev-parse", "--show-toplevel"]).map_err(|_| {
            Error::Invalid(format!("{} is not inside a git repository", path.display()))
        })?;
        Ok(canonical(out.trim()))
    }

    fn list(&self, repo: &Path) -> Result<Vec<GitWorktreeInfo>> {
        Ok(parse_porcelain(&git(
            repo,
            &["worktree", "list", "--porcelain"],
        )?))
    }

    fn branches(&self, repo: &Path) -> Result<Vec<String>> {
        let out = git(
            repo,
            &["for-each-ref", "--format=%(refname:short)", "refs/heads"],
        )?;
        Ok(out.lines().map(str::to_string).collect())
    }

    fn add(
        &self,
        repo: &Path,
        path: &Path,
        branch: &str,
        base: Option<&str>,
        create_branch: bool,
    ) -> Result<()> {
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

impl TaskGit for GitCli {
    fn commit_all(&self, worktree: &Path, message: &str) -> Result<bool> {
        git(worktree, &["add", "-A"])?;
        let (staged, _) = git_code(worktree, &["diff", "--cached", "--quiet"])?;
        if staged == 0 {
            return Ok(false);
        }
        git(
            worktree,
            &with_identity(worktree, &["commit", "-qm", message]),
        )?;
        Ok(true)
    }

    fn diff(&self, repo: &Path, base: &str, branch: &str) -> Result<String> {
        git(repo, &["diff", &format!("{base}...{branch}")])
    }

    fn changed_files(&self, repo: &Path, base: &str, branch: &str) -> Result<Vec<String>> {
        let out = git(
            repo,
            &["diff", "--name-only", &format!("{base}...{branch}")],
        )?;
        Ok(out.lines().map(str::to_string).collect())
    }

    fn conflicts(&self, repo: &Path, base: &str, branch: &str) -> Result<Vec<String>> {
        let (code, out) = git_code(
            repo,
            &[
                "merge-tree",
                "--write-tree",
                "--name-only",
                "--no-messages",
                base,
                branch,
            ],
        )?;
        match code {
            0 => Ok(vec![]),
            // The first line is the (conflicted) tree; the rest are paths.
            1 => Ok(out
                .lines()
                .skip(1)
                .filter(|l| !l.is_empty())
                .map(str::to_string)
                .collect()),
            _ => Err(Error::Git(format!("can't compare {branch} with {base}"))),
        }
    }

    fn merge(&self, repo: &Path, base: &str, branch: &str, message: &str) -> Result<()> {
        let conflicts = self.conflicts(repo, base, branch)?;
        if !conflicts.is_empty() {
            return Err(Error::Git(format!(
                "{branch} conflicts with {base} in {}",
                conflicts.join(", ")
            )));
        }
        let checked_out = self
            .list(repo)?
            .into_iter()
            .find(|w| w.branch.as_deref() == Some(base));
        match checked_out {
            // Merge where base is checked out so its files stay in step.
            Some(w) => {
                if !git(&w.path, &["status", "--porcelain"])?.trim().is_empty() {
                    return Err(Error::Git(format!(
                        "{} has uncommitted changes; commit or stash them before merging into {base}",
                        w.path.display()
                    )));
                }
                git(
                    &w.path,
                    &with_identity(&w.path, &["merge", "--no-ff", "-qm", message, branch]),
                )?;
            }
            // Nobody has base checked out: build the merge commit directly.
            None => {
                let tree = git(repo, &["merge-tree", "--write-tree", base, branch])?;
                let tree = tree.lines().next().unwrap_or_default().to_string();
                let commit = git(
                    repo,
                    &with_identity(
                        repo,
                        &[
                            "commit-tree",
                            &tree,
                            "-p",
                            base,
                            "-p",
                            branch,
                            "-m",
                            message,
                        ],
                    ),
                )?;
                git(
                    repo,
                    &["update-ref", &format!("refs/heads/{base}"), commit.trim()],
                )?;
            }
        }
        Ok(())
    }

    fn delete_branch(&self, repo: &Path, branch: &str) -> Result<()> {
        git(repo, &["branch", "-D", branch])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_porcelain_output() {
        let out =
            "worktree /a\nHEAD 1\nbranch refs/heads/main\n\nworktree /b\nHEAD 2\ndetached\n\n";
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
        git(
            p,
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "commit",
                "-qm",
                "init",
            ],
        )
        .unwrap();
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
        assert!(
            git.branches(repo.path())
                .unwrap()
                .contains(&"feature".to_string())
        );

        git.remove(repo.path(), &wt, false).unwrap();
        assert_eq!(git.list(repo.path()).unwrap().len(), 1);
    }

    fn commit(dir: &Path, file: &str, text: &str) {
        std::fs::write(dir.join(file), text).unwrap();
        assert!(GitCli.commit_all(dir, &format!("edit {file}")).unwrap());
    }

    #[test]
    fn commits_diffs_and_merges_a_task_branch_into_a_checked_out_base() {
        let repo = init_repo();
        let wt_parent = tempfile::tempdir().unwrap();
        let wt = wt_parent.path().join("task");
        GitCli
            .add(repo.path(), &wt, "task", Some("main"), true)
            .unwrap();

        assert!(!GitCli.commit_all(&wt, "nothing").unwrap());
        commit(&wt, "b.txt", "b");
        assert_eq!(
            GitCli.changed_files(repo.path(), "main", "task").unwrap(),
            ["b.txt"]
        );
        assert!(
            GitCli
                .diff(repo.path(), "main", "task")
                .unwrap()
                .contains("+b")
        );
        assert!(
            GitCli
                .conflicts(repo.path(), "main", "task")
                .unwrap()
                .is_empty()
        );

        GitCli
            .merge(repo.path(), "main", "task", "Merge task")
            .unwrap();
        // main is checked out in the repo, so its files follow the merge.
        assert_eq!(
            std::fs::read_to_string(repo.path().join("b.txt")).unwrap(),
            "b"
        );
        GitCli.remove(repo.path(), &wt, false).unwrap();
        GitCli.delete_branch(repo.path(), "task").unwrap();
        assert!(
            !GitCli
                .branches(repo.path())
                .unwrap()
                .contains(&"task".into())
        );
    }

    #[test]
    fn merges_into_a_base_nobody_has_checked_out() {
        let repo = init_repo();
        git(repo.path(), &["branch", "develop"]).unwrap();
        let wt_parent = tempfile::tempdir().unwrap();
        let wt = wt_parent.path().join("task");
        GitCli
            .add(repo.path(), &wt, "task", Some("develop"), true)
            .unwrap();
        commit(&wt, "c.txt", "c");
        GitCli
            .merge(repo.path(), "develop", "task", "Merge task")
            .unwrap();
        let files = git(repo.path(), &["ls-tree", "--name-only", "develop"]).unwrap();
        assert!(files.lines().any(|f| f == "c.txt"));
    }

    #[test]
    fn reports_conflicts_and_refuses_to_merge_them() {
        let repo = init_repo();
        let wt_parent = tempfile::tempdir().unwrap();
        let wt = wt_parent.path().join("task");
        GitCli
            .add(repo.path(), &wt, "task", Some("main"), true)
            .unwrap();
        commit(&wt, "a.txt", "theirs");
        commit(repo.path(), "a.txt", "ours");
        assert_eq!(
            GitCli.conflicts(repo.path(), "main", "task").unwrap(),
            ["a.txt"]
        );
        assert!(GitCli.merge(repo.path(), "main", "task", "m").is_err());
    }

    #[test]
    fn refuses_to_merge_into_a_dirty_checkout() {
        let repo = init_repo();
        let wt_parent = tempfile::tempdir().unwrap();
        let wt = wt_parent.path().join("task");
        GitCli
            .add(repo.path(), &wt, "task", Some("main"), true)
            .unwrap();
        commit(&wt, "b.txt", "b");
        std::fs::write(repo.path().join("a.txt"), "wip").unwrap();
        let err = GitCli.merge(repo.path(), "main", "task", "m").unwrap_err();
        assert!(err.to_string().contains("uncommitted changes"));
    }

    #[test]
    fn toplevel_outside_a_repo_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(GitCli.toplevel(dir.path()).is_err());
    }
}
