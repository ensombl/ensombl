//! [`FileWatcher`] built on `notify` with debouncing. Changed source paths
//! are sent to a channel; the composition root turns them into commands.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};

use crate::application::ports::FileWatcher;
use crate::application::{Error, Result};

use super::fs_scanner::to_rel;

/// `(worktree id, repo-relative changed paths)`.
pub type ChangeBatch = (String, Vec<String>);

type Watcher = Debouncer<notify::RecommendedWatcher, RecommendedCache>;

pub struct NotifyWatcher {
    tx: Sender<ChangeBatch>,
    is_source: Arc<dyn Fn(&str) -> bool + Send + Sync>,
    watchers: Mutex<HashMap<String, Watcher>>,
}

impl NotifyWatcher {
    pub fn new(
        tx: Sender<ChangeBatch>,
        is_source: Arc<dyn Fn(&str) -> bool + Send + Sync>,
    ) -> Self {
        Self {
            tx,
            is_source,
            watchers: Mutex::default(),
        }
    }
}

impl FileWatcher for NotifyWatcher {
    fn watch(&self, worktree: &str, root: &Path) -> Result<()> {
        let mut watchers = self.watchers.lock().unwrap();
        if watchers.contains_key(worktree) {
            return Ok(());
        }
        let root: PathBuf = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let (tx, id, is_source, base) = (
            self.tx.clone(),
            worktree.to_string(),
            self.is_source.clone(),
            root.clone(),
        );

        let mut debouncer = new_debouncer(
            Duration::from_millis(150),
            None,
            move |result: DebounceEventResult| {
                let Ok(events) = result else { return };
                let paths: HashSet<String> = events
                    .iter()
                    .flat_map(|e| e.paths.iter())
                    .filter_map(|p| to_rel(&base, p))
                    .filter(|p| is_source(p))
                    .collect();
                if !paths.is_empty() {
                    let mut paths: Vec<_> = paths.into_iter().collect();
                    paths.sort();
                    let _ = tx.send((id.clone(), paths));
                }
            },
        )
        .map_err(|e| Error::Invalid(format!("watcher: {e}")))?;
        debouncer
            .watch(&root, RecursiveMode::Recursive)
            .map_err(|e| Error::Invalid(format!("watcher: {e}")))?;
        watchers.insert(worktree.into(), debouncer);
        Ok(())
    }

    fn unwatch(&self, worktree: &str) {
        self.watchers.lock().unwrap().remove(worktree);
    }
}
