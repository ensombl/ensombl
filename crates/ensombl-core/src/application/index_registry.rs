//! In-memory index state per worktree tab. Owned by the application layer;
//! commands write to it and queries read from it.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

use crate::domain::model::ArchModel;
use crate::domain::parsed::ParsedFile;
use crate::domain::system::model::SystemModel;

use super::ports::ModuleResolver;

pub struct WorktreeIndex {
    pub root: PathBuf,
    pub files: HashMap<String, ParsedFile>,
    pub resolver: Box<dyn ModuleResolver>,
    pub model: ArchModel,
    pub system: SystemModel,
}

#[derive(Default)]
pub struct IndexRegistry {
    indexes: RwLock<HashMap<String, WorktreeIndex>>,
}

impl IndexRegistry {
    pub fn insert(&self, worktree: &str, index: WorktreeIndex) {
        self.indexes.write().unwrap().insert(worktree.into(), index);
    }

    pub fn remove(&self, worktree: &str) {
        self.indexes.write().unwrap().remove(worktree);
    }

    pub fn contains(&self, worktree: &str) -> bool {
        self.indexes.read().unwrap().contains_key(worktree)
    }

    pub fn root(&self, worktree: &str) -> Option<PathBuf> {
        self.indexes
            .read()
            .unwrap()
            .get(worktree)
            .map(|i| i.root.clone())
    }

    pub fn model(&self, worktree: &str) -> Option<ArchModel> {
        self.indexes
            .read()
            .unwrap()
            .get(worktree)
            .map(|i| i.model.clone())
    }

    pub fn system(&self, worktree: &str) -> Option<SystemModel> {
        self.indexes
            .read()
            .unwrap()
            .get(worktree)
            .map(|i| i.system.clone())
    }

    pub fn update<T>(&self, worktree: &str, f: impl FnOnce(&mut WorktreeIndex) -> T) -> Option<T> {
        self.indexes.write().unwrap().get_mut(worktree).map(f)
    }
}
