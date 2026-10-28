//! Projects, worktrees and the tab session.
//!
//! A *project* is a git repository. Each *worktree* of that repository can be
//! opened as a *tab*; tabs are grouped by project like browser tab groups. The
//! [`Session`] aggregate owns which tabs are open, their order and the active
//! one. It's pure state with no IO.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub type ProjectId = String;
pub type WorktreeId = String;

/// Group colors, matching the browser tab-group palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum GroupColor {
    Blue,
    Green,
    Purple,
    Orange,
    Pink,
    Cyan,
    Yellow,
    Red,
    Grey,
}

impl GroupColor {
    const PALETTE: [GroupColor; 9] = [
        GroupColor::Blue,
        GroupColor::Green,
        GroupColor::Purple,
        GroupColor::Orange,
        GroupColor::Pink,
        GroupColor::Cyan,
        GroupColor::Yellow,
        GroupColor::Red,
        GroupColor::Grey,
    ];

    /// Picks the first palette color not already used, cycling when all are taken.
    pub fn next_free(used: impl IntoIterator<Item = GroupColor>) -> GroupColor {
        let used: Vec<_> = used.into_iter().collect();
        Self::PALETTE
            .into_iter()
            .find(|c| !used.contains(c))
            .unwrap_or(Self::PALETTE[used.len() % Self::PALETTE.len()])
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    /// Path of the main worktree (the original checkout).
    pub root: String,
    pub color: GroupColor,
    pub collapsed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Worktree {
    pub id: WorktreeId,
    pub project_id: ProjectId,
    pub path: String,
    /// `None` for a detached HEAD.
    pub branch: Option<String>,
    pub is_main: bool,
}

impl Worktree {
    /// Human label for a tab: the branch, or the folder name when detached.
    pub fn label(&self) -> String {
        self.branch.clone().unwrap_or_else(|| {
            self.path
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(&self.path)
                .to_string()
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub projects: Vec<Project>,
    /// Open tabs in display order. A worktree is open in at most one tab.
    pub tabs: Vec<Worktree>,
    pub active: Option<WorktreeId>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SessionError {
    #[error("unknown project {0}")]
    UnknownProject(ProjectId),
    #[error("worktree {0} isn't open")]
    TabNotOpen(WorktreeId),
}

impl Session {
    pub fn project(&self, id: &str) -> Option<&Project> {
        self.projects.iter().find(|p| p.id == id)
    }

    /// Adds the project if it's new; returns its id either way.
    pub fn add_project(&mut self, id: ProjectId, name: String, root: String) -> ProjectId {
        if self.project(&id).is_none() {
            let color = GroupColor::next_free(self.projects.iter().map(|p| p.color));
            self.projects.push(Project { id: id.clone(), name, root, color, collapsed: false });
        }
        id
    }

    /// Opens (or focuses) a tab. New tabs go at the end of their project's group.
    pub fn open_tab(&mut self, worktree: Worktree) -> Result<(), SessionError> {
        let project = self
            .project(&worktree.project_id)
            .ok_or_else(|| SessionError::UnknownProject(worktree.project_id.clone()))?;
        let project_id = project.id.clone();

        if let Some(existing) = self.tabs.iter_mut().find(|t| t.id == worktree.id) {
            *existing = worktree.clone();
        } else {
            let insert_at = self
                .tabs
                .iter()
                .rposition(|t| t.project_id == project_id)
                .map(|i| i + 1)
                .unwrap_or(self.tabs.len());
            self.tabs.insert(insert_at, worktree.clone());
        }
        self.set_collapsed(&project_id, false);
        self.active = Some(worktree.id);
        Ok(())
    }

    /// Closes a tab and focuses its neighbour, preferring one in the same group.
    pub fn close_tab(&mut self, id: &str) -> Result<(), SessionError> {
        let index = self
            .tabs
            .iter()
            .position(|t| t.id == id)
            .ok_or_else(|| SessionError::TabNotOpen(id.into()))?;
        let closed = self.tabs.remove(index);

        if self.active.as_deref() == Some(id) {
            let same_group = |t: &&Worktree| t.project_id == closed.project_id;
            self.active = self
                .tabs
                .get(index)
                .filter(same_group)
                .or_else(|| index.checked_sub(1).and_then(|i| self.tabs.get(i)).filter(same_group))
                .or_else(|| self.tabs.get(index.min(self.tabs.len().saturating_sub(1))))
                .map(|t| t.id.clone());
        }
        Ok(())
    }

    pub fn activate(&mut self, id: &str) -> Result<(), SessionError> {
        let tab = self
            .tabs
            .iter()
            .find(|t| t.id == id)
            .ok_or_else(|| SessionError::TabNotOpen(id.into()))?;
        let project_id = tab.project_id.clone();
        self.set_collapsed(&project_id, false);
        self.active = Some(id.into());
        Ok(())
    }

    pub fn toggle_group(&mut self, project_id: &str) -> Result<(), SessionError> {
        let project = self
            .projects
            .iter_mut()
            .find(|p| p.id == project_id)
            .ok_or_else(|| SessionError::UnknownProject(project_id.into()))?;
        project.collapsed = !project.collapsed;
        Ok(())
    }

    /// Removes a project and all of its tabs from the session (nothing on disk is touched).
    pub fn remove_project(&mut self, project_id: &str) -> Result<(), SessionError> {
        if self.project(project_id).is_none() {
            return Err(SessionError::UnknownProject(project_id.into()));
        }
        let ids: Vec<_> = self
            .tabs
            .iter()
            .filter(|t| t.project_id == project_id)
            .map(|t| t.id.clone())
            .collect();
        for id in ids {
            self.close_tab(&id)?;
        }
        self.projects.retain(|p| p.id != project_id);
        Ok(())
    }

    fn set_collapsed(&mut self, project_id: &str, collapsed: bool) {
        if let Some(p) = self.projects.iter_mut().find(|p| p.id == project_id) {
            p.collapsed = collapsed;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wt(id: &str, project: &str) -> Worktree {
        Worktree {
            id: id.into(),
            project_id: project.into(),
            path: format!("/tmp/{id}"),
            branch: Some(id.into()),
            is_main: false,
        }
    }

    fn session() -> Session {
        let mut s = Session::default();
        s.add_project("a".into(), "alpha".into(), "/a".into());
        s.add_project("b".into(), "beta".into(), "/b".into());
        s
    }

    fn order(s: &Session) -> Vec<&str> {
        s.tabs.iter().map(|t| t.id.as_str()).collect()
    }

    #[test]
    fn projects_get_distinct_colors() {
        let s = session();
        assert_ne!(s.projects[0].color, s.projects[1].color);
    }

    #[test]
    fn adding_a_project_twice_is_idempotent() {
        let mut s = session();
        s.add_project("a".into(), "alpha".into(), "/a".into());
        assert_eq!(s.projects.len(), 2);
    }

    #[test]
    fn tabs_are_grouped_by_project() {
        let mut s = session();
        s.open_tab(wt("a1", "a")).unwrap();
        s.open_tab(wt("b1", "b")).unwrap();
        s.open_tab(wt("a2", "a")).unwrap();
        assert_eq!(order(&s), ["a1", "a2", "b1"]);
        assert_eq!(s.active.as_deref(), Some("a2"));
    }

    #[test]
    fn reopening_a_tab_focuses_it_without_duplicating() {
        let mut s = session();
        s.open_tab(wt("a1", "a")).unwrap();
        s.open_tab(wt("a2", "a")).unwrap();
        s.open_tab(wt("a1", "a")).unwrap();
        assert_eq!(order(&s), ["a1", "a2"]);
        assert_eq!(s.active.as_deref(), Some("a1"));
    }

    #[test]
    fn opening_a_tab_for_an_unknown_project_fails() {
        let mut s = session();
        assert_eq!(s.open_tab(wt("x", "nope")), Err(SessionError::UnknownProject("nope".into())));
    }

    #[test]
    fn closing_the_active_tab_prefers_a_sibling_in_the_same_group() {
        let mut s = session();
        s.open_tab(wt("a1", "a")).unwrap();
        s.open_tab(wt("a2", "a")).unwrap();
        s.open_tab(wt("b1", "b")).unwrap();
        s.activate("a2").unwrap();
        s.close_tab("a2").unwrap();
        assert_eq!(s.active.as_deref(), Some("a1"));
    }

    #[test]
    fn closing_the_last_tab_clears_active() {
        let mut s = session();
        s.open_tab(wt("a1", "a")).unwrap();
        s.close_tab("a1").unwrap();
        assert_eq!(s.active, None);
    }

    #[test]
    fn activating_a_tab_expands_its_group() {
        let mut s = session();
        s.open_tab(wt("a1", "a")).unwrap();
        s.toggle_group("a").unwrap();
        assert!(s.projects[0].collapsed);
        s.activate("a1").unwrap();
        assert!(!s.projects[0].collapsed);
    }

    #[test]
    fn removing_a_project_closes_its_tabs() {
        let mut s = session();
        s.open_tab(wt("a1", "a")).unwrap();
        s.open_tab(wt("b1", "b")).unwrap();
        s.remove_project("b").unwrap();
        assert_eq!(order(&s), ["a1"]);
        assert_eq!(s.active.as_deref(), Some("a1"));
        assert!(s.project("b").is_none());
    }

    #[test]
    fn detached_worktree_label_falls_back_to_folder() {
        let mut w = wt("x", "a");
        w.branch = None;
        w.path = "/repos/crm/graysby".into();
        assert_eq!(w.label(), "graysby");
    }
}
