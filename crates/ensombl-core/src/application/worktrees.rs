//! Helpers for resolving open worktrees and paths inside them.

use std::path::{Component, Path, PathBuf};

use std::sync::Arc;

use super::ports::{SessionRepository, TaskStore, WorktreeResolver};
use super::{Error, Result};
use crate::domain::orchestration::task_of_worktree;

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

/// Resolves open tabs, then (when tasks are wired in) task worktrees.
pub struct Worktrees {
    sessions: Arc<dyn SessionRepository>,
    tasks: Option<Arc<dyn TaskStore>>,
}

impl Worktrees {
    pub fn new(sessions: Arc<dyn SessionRepository>, tasks: Option<Arc<dyn TaskStore>>) -> Self {
        Self { sessions, tasks }
    }
}

impl WorktreeResolver for Worktrees {
    fn root(&self, worktree_id: &str) -> Result<PathBuf> {
        if let (Some(task_id), Some(tasks)) = (task_of_worktree(worktree_id), &self.tasks) {
            return tasks
                .get(task_id)?
                .and_then(|t| t.worktree)
                .map(PathBuf::from)
                .ok_or_else(|| Error::Invalid(format!("task {task_id} has no worktree yet")));
        }
        worktree_root(self.sessions.as_ref(), worktree_id)
    }
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
    use crate::domain::agent_run::PermissionMode;
    use crate::domain::orchestration::{Task, TaskStatus, task_worktree_id};
    use crate::domain::terminal::AgentKind;
    use crate::domain::workspace::{Session, Worktree};
    use crate::infrastructure::sqlite_tasks::SqliteTasks;

    struct Sessions(Session);
    impl SessionRepository for Sessions {
        fn get(&self) -> Result<Session> {
            Ok(self.0.clone())
        }
        fn update(&self, f: &mut dyn FnMut(&mut Session) -> Result<()>) -> Result<()> {
            f(&mut self.0.clone())
        }
    }

    #[test]
    fn resolves_tabs_and_task_worktrees() {
        let mut session = Session::default();
        session.add_project("p".into(), "proj".into(), "/repo".into());
        session
            .open_tab(Worktree {
                id: "wt".into(),
                project_id: "p".into(),
                path: "/repo".into(),
                branch: None,
                is_main: true,
            })
            .unwrap();
        let tasks = Arc::new(SqliteTasks::in_memory().unwrap());
        let mut task = Task {
            id: "k1".into(),
            project_id: "p".into(),
            title: "t".into(),
            prompt: "p".into(),
            agent: AgentKind::Claude,
            base_branch: "main".into(),
            branch: "ensombl/t-k1".into(),
            worktree: None,
            status: TaskStatus::Queued,
            depends_on: vec![],
            permission: PermissionMode::AcceptEdits,
            session_id: None,
            next_prompt: None,
            summary: None,
            error: None,
            created_at: 0,
            updated_at: 0,
        };
        tasks.put(&task).unwrap();
        let worktrees = Worktrees::new(Arc::new(Sessions(session)), Some(tasks.clone()));
        assert_eq!(worktrees.root("wt").unwrap(), PathBuf::from("/repo"));
        assert!(worktrees.root(&task_worktree_id("k1")).is_err());
        task.worktree = Some("/w/k1".into());
        tasks.put(&task).unwrap();
        assert_eq!(
            worktrees.root(&task_worktree_id("k1")).unwrap(),
            PathBuf::from("/w/k1")
        );
        assert!(worktrees.root("nope").is_err());
    }

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
