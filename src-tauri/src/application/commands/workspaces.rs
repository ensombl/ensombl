//! Commands that change projects, worktrees and open tabs.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Deserialize;
use ts_rs::TS;

use crate::application::cqrs::{Command, CommandHandler};
use crate::application::ids::{folder_name, id_for_path, slug};
use crate::application::ports::{GitWorktreeInfo, GitWorktrees, SessionRepository};
use crate::application::{Error, Result};
use crate::domain::workspace::{ProjectId, Session, Worktree, WorktreeId};

/// Adds the git repository containing `path` as a project and opens a tab for that worktree.
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct AddProject {
    pub path: String,
}

/// Creates a git worktree (and a branch, if it doesn't exist yet) and opens it in a tab.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateWorktree {
    pub project_id: ProjectId,
    pub branch: String,
    /// Base ref for a new branch; defaults to the main worktree's HEAD.
    #[ts(optional)]
    pub base: Option<String>,
}

/// Opens an existing worktree of a project in a tab.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OpenWorktree {
    pub project_id: ProjectId,
    pub path: String,
}

/// Deletes a (non-main) worktree from disk with `git worktree remove`.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RemoveWorktree {
    pub project_id: ProjectId,
    pub path: String,
    pub force: bool,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CloseTab {
    pub worktree_id: WorktreeId,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActivateTab {
    pub worktree_id: WorktreeId,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ToggleGroup {
    pub project_id: ProjectId,
}

/// Removes a project from CRM. Nothing on disk is touched.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RemoveProject {
    pub project_id: ProjectId,
}

impl Command for AddProject {
    type Output = WorktreeId;
}
impl Command for CreateWorktree {
    type Output = WorktreeId;
}
impl Command for OpenWorktree {
    type Output = WorktreeId;
}
impl Command for RemoveWorktree {
    type Output = ();
}
impl Command for CloseTab {
    type Output = ();
}
impl Command for ActivateTab {
    type Output = ();
}
impl Command for ToggleGroup {
    type Output = ();
}
impl Command for RemoveProject {
    type Output = ();
}

/// Handles every workspace command; shares the git and session ports.
pub struct WorkspaceCommands {
    git: Arc<dyn GitWorktrees>,
    sessions: Arc<dyn SessionRepository>,
    /// New worktrees go to `<worktrees_dir>/<project>/<branch-slug>`.
    worktrees_dir: PathBuf,
}

impl WorkspaceCommands {
    pub fn new(git: Arc<dyn GitWorktrees>, sessions: Arc<dyn SessionRepository>, worktrees_dir: PathBuf) -> Self {
        Self { git, sessions, worktrees_dir }
    }

    fn project_root(&self, project_id: &str) -> Result<PathBuf> {
        let session = self.sessions.get()?;
        let project = session
            .project(project_id)
            .ok_or_else(|| Error::Invalid(format!("unknown project {project_id}")))?;
        Ok(PathBuf::from(&project.root))
    }

    fn find_worktree(&self, root: &Path, path: &Path) -> Result<GitWorktreeInfo> {
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.git
            .list(root)?
            .into_iter()
            .find(|w| w.path == path)
            .ok_or_else(|| Error::Invalid(format!("{} is not a worktree of this project", path.display())))
    }

    fn open(&self, project_id: &str, info: GitWorktreeInfo) -> Result<WorktreeId> {
        let worktree = to_worktree(project_id, info);
        let id = worktree.id.clone();
        self.sessions.update(&mut |s: &mut Session| Ok(s.open_tab(worktree.clone())?))?;
        Ok(id)
    }

    fn free_path(&self, project_name: &str, branch: &str) -> PathBuf {
        let base = self.worktrees_dir.join(slug(project_name)).join(slug(branch));
        let mut candidate = base.clone();
        let mut n = 2;
        while candidate.exists() {
            candidate = PathBuf::from(format!("{}-{n}", base.display()));
            n += 1;
        }
        candidate
    }
}

pub fn to_worktree(project_id: &str, info: GitWorktreeInfo) -> Worktree {
    Worktree {
        id: id_for_path(&info.path),
        project_id: project_id.into(),
        path: info.path.to_string_lossy().into_owned(),
        branch: info.branch,
        is_main: info.is_main,
    }
}

impl CommandHandler<AddProject> for WorkspaceCommands {
    fn handle(&self, cmd: AddProject) -> Result<WorktreeId> {
        let toplevel = self.git.toplevel(Path::new(&cmd.path))?;
        let worktrees = self.git.list(&toplevel)?;
        let main = worktrees
            .iter()
            .find(|w| w.is_main)
            .ok_or_else(|| Error::Git("repository has no main worktree".into()))?;

        let project_id = id_for_path(&main.path);
        let name = folder_name(&main.path);
        let root = main.path.to_string_lossy().into_owned();
        self.sessions.update(&mut |s: &mut Session| {
            s.add_project(project_id.clone(), name.clone(), root.clone());
            Ok(())
        })?;

        let opened = worktrees
            .iter()
            .find(|w| w.path == toplevel)
            .unwrap_or(main)
            .clone();
        self.open(&project_id, opened)
    }
}

impl CommandHandler<CreateWorktree> for WorkspaceCommands {
    fn handle(&self, cmd: CreateWorktree) -> Result<WorktreeId> {
        let branch = cmd.branch.trim();
        if branch.is_empty() || branch.contains(char::is_whitespace) {
            return Err(Error::Invalid("branch name must be non-empty and contain no spaces".into()));
        }
        let root = self.project_root(&cmd.project_id)?;
        let exists = self.git.branches(&root)?.iter().any(|b| b == branch);
        let path = self.free_path(&folder_name(&root), branch);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        self.git.add(&root, &path, branch, cmd.base.as_deref(), !exists)?;
        let info = self.find_worktree(&root, &path)?;
        self.open(&cmd.project_id, info)
    }
}

impl CommandHandler<OpenWorktree> for WorkspaceCommands {
    fn handle(&self, cmd: OpenWorktree) -> Result<WorktreeId> {
        let root = self.project_root(&cmd.project_id)?;
        let info = self.find_worktree(&root, Path::new(&cmd.path))?;
        self.open(&cmd.project_id, info)
    }
}

impl CommandHandler<RemoveWorktree> for WorkspaceCommands {
    fn handle(&self, cmd: RemoveWorktree) -> Result<()> {
        let root = self.project_root(&cmd.project_id)?;
        let info = self.find_worktree(&root, Path::new(&cmd.path))?;
        if info.is_main {
            return Err(Error::Invalid("the main worktree can't be removed".into()));
        }
        let id = id_for_path(&info.path);
        self.sessions.update(&mut |s: &mut Session| {
            if s.tabs.iter().any(|t| t.id == id) {
                s.close_tab(&id)?;
            }
            Ok(())
        })?;
        self.git.remove(&root, &info.path, cmd.force)
    }
}

impl CommandHandler<CloseTab> for WorkspaceCommands {
    fn handle(&self, cmd: CloseTab) -> Result<()> {
        self.sessions.update(&mut |s: &mut Session| Ok(s.close_tab(&cmd.worktree_id)?))
    }
}

impl CommandHandler<ActivateTab> for WorkspaceCommands {
    fn handle(&self, cmd: ActivateTab) -> Result<()> {
        self.sessions.update(&mut |s: &mut Session| Ok(s.activate(&cmd.worktree_id)?))
    }
}

impl CommandHandler<ToggleGroup> for WorkspaceCommands {
    fn handle(&self, cmd: ToggleGroup) -> Result<()> {
        self.sessions.update(&mut |s: &mut Session| Ok(s.toggle_group(&cmd.project_id)?))
    }
}

impl CommandHandler<RemoveProject> for WorkspaceCommands {
    fn handle(&self, cmd: RemoveProject) -> Result<()> {
        self.sessions.update(&mut |s: &mut Session| Ok(s.remove_project(&cmd.project_id)?))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct MemorySessions(Mutex<Session>);
    impl SessionRepository for MemorySessions {
        fn get(&self) -> Result<Session> {
            Ok(self.0.lock().unwrap().clone())
        }
        fn update(&self, f: &mut dyn FnMut(&mut Session) -> Result<()>) -> Result<()> {
            f(&mut self.0.lock().unwrap())
        }
    }

    /// Fake repo at /repo with a main worktree and one linked worktree at /wt/feature.
    struct FakeGit {
        worktrees: Mutex<Vec<GitWorktreeInfo>>,
        added: Mutex<Vec<(PathBuf, String, bool)>>,
    }
    impl FakeGit {
        fn new() -> Self {
            Self {
                worktrees: Mutex::new(vec![
                    GitWorktreeInfo { path: "/repo".into(), branch: Some("main".into()), is_main: true },
                    GitWorktreeInfo { path: "/wt/feature".into(), branch: Some("feature".into()), is_main: false },
                ]),
                added: Mutex::default(),
            }
        }
    }
    impl GitWorktrees for FakeGit {
        fn toplevel(&self, path: &Path) -> Result<PathBuf> {
            Ok(if path.starts_with("/wt/feature") { "/wt/feature".into() } else { "/repo".into() })
        }
        fn list(&self, _: &Path) -> Result<Vec<GitWorktreeInfo>> {
            Ok(self.worktrees.lock().unwrap().clone())
        }
        fn branches(&self, _: &Path) -> Result<Vec<String>> {
            Ok(vec!["main".into(), "feature".into()])
        }
        fn add(&self, _: &Path, path: &Path, branch: &str, _: Option<&str>, create: bool) -> Result<()> {
            self.added.lock().unwrap().push((path.into(), branch.into(), create));
            self.worktrees.lock().unwrap().push(GitWorktreeInfo {
                path: path.into(),
                branch: Some(branch.into()),
                is_main: false,
            });
            Ok(())
        }
        fn remove(&self, _: &Path, path: &Path, _: bool) -> Result<()> {
            self.worktrees.lock().unwrap().retain(|w| w.path != path);
            Ok(())
        }
    }

    fn setup() -> (WorkspaceCommands, Arc<FakeGit>, Arc<MemorySessions>, tempfile::TempDir) {
        let git = Arc::new(FakeGit::new());
        let sessions = Arc::new(MemorySessions::default());
        let dir = tempfile::tempdir().unwrap();
        let cmds = WorkspaceCommands::new(git.clone(), sessions.clone(), dir.path().into());
        (cmds, git, sessions, dir)
    }

    #[test]
    fn adding_a_linked_worktree_registers_the_main_repo_as_project() {
        let (cmds, _, sessions, _d) = setup();
        let id = cmds.handle(AddProject { path: "/wt/feature/src".into() }).unwrap();
        let s = sessions.get().unwrap();
        assert_eq!(s.projects[0].root, "/repo");
        assert_eq!(s.projects[0].name, "repo");
        assert_eq!(s.tabs[0].path, "/wt/feature");
        assert_eq!(s.active, Some(id));
    }

    #[test]
    fn create_worktree_makes_a_new_branch_under_the_worktrees_dir() {
        let (cmds, git, sessions, dir) = setup();
        cmds.handle(AddProject { path: "/repo".into() }).unwrap();
        let project_id = sessions.get().unwrap().projects[0].id.clone();

        cmds.handle(CreateWorktree { project_id, branch: "feat/x".into(), base: None }).unwrap();

        let (path, branch, created) = git.added.lock().unwrap()[0].clone();
        assert_eq!(path, dir.path().join("repo").join("feat-x"));
        assert_eq!(branch, "feat/x");
        assert!(created);
        assert_eq!(sessions.get().unwrap().tabs.len(), 2);
    }

    #[test]
    fn create_worktree_reuses_an_existing_branch() {
        let (cmds, git, sessions, _d) = setup();
        cmds.handle(AddProject { path: "/repo".into() }).unwrap();
        let project_id = sessions.get().unwrap().projects[0].id.clone();
        cmds.handle(CreateWorktree { project_id, branch: "main".into(), base: None }).unwrap();
        assert!(!git.added.lock().unwrap()[0].2);
    }

    #[test]
    fn create_worktree_rejects_bad_branch_names() {
        let (cmds, _, sessions, _d) = setup();
        cmds.handle(AddProject { path: "/repo".into() }).unwrap();
        let project_id = sessions.get().unwrap().projects[0].id.clone();
        let err = cmds.handle(CreateWorktree { project_id, branch: "a b".into(), base: None });
        assert!(matches!(err, Err(Error::Invalid(_))));
    }

    #[test]
    fn the_main_worktree_cannot_be_removed() {
        let (cmds, _, sessions, _d) = setup();
        cmds.handle(AddProject { path: "/repo".into() }).unwrap();
        let project_id = sessions.get().unwrap().projects[0].id.clone();
        let err = cmds.handle(RemoveWorktree { project_id, path: "/repo".into(), force: false });
        assert!(matches!(err, Err(Error::Invalid(_))));
    }

    #[test]
    fn removing_a_worktree_closes_its_tab() {
        let (cmds, git, sessions, _d) = setup();
        cmds.handle(AddProject { path: "/wt/feature".into() }).unwrap();
        let project_id = sessions.get().unwrap().projects[0].id.clone();
        cmds.handle(RemoveWorktree { project_id, path: "/wt/feature".into(), force: false }).unwrap();
        assert!(sessions.get().unwrap().tabs.is_empty());
        assert_eq!(git.worktrees.lock().unwrap().len(), 1);
    }
}
