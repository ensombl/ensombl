//! [`CommentStore`] and [`SettingsRepository`] as JSON files in the app data dir.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::application::ports::{CommentStore, Settings, SettingsRepository};
use crate::application::{Error, Result};
use crate::domain::review::ReviewComment;

/// A JSON document cached in memory and written atomically.
struct JsonFile<T> {
    path: PathBuf,
    value: Mutex<T>,
}

impl<T: Serialize + DeserializeOwned + Default + Clone> JsonFile<T> {
    fn open(path: PathBuf) -> Self {
        let value = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Self {
            path,
            value: Mutex::new(value),
        }
    }

    fn get(&self) -> T {
        self.value.lock().unwrap().clone()
    }

    fn update(&self, f: impl FnOnce(&mut T)) -> Result<()> {
        let mut guard = self.value.lock().unwrap();
        let mut next = guard.clone();
        f(&mut next);
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_vec_pretty(&next).map_err(|e| Error::Storage(e.to_string()))?;
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(tmp, &self.path)?;
        *guard = next;
        Ok(())
    }
}

pub struct JsonComments(JsonFile<HashMap<String, Vec<ReviewComment>>>);

impl JsonComments {
    pub fn open(path: PathBuf) -> Self {
        Self(JsonFile::open(path))
    }
}

impl CommentStore for JsonComments {
    fn load(&self, worktree: &str) -> Vec<ReviewComment> {
        self.0.get().remove(worktree).unwrap_or_default()
    }

    fn save(&self, worktree: &str, comments: &[ReviewComment]) -> Result<()> {
        self.0.update(|all| {
            all.insert(worktree.into(), comments.to_vec());
        })
    }
}

pub struct JsonSettings(JsonFile<Settings>);

impl JsonSettings {
    pub fn open(path: PathBuf) -> Self {
        Self(JsonFile::open(path))
    }
}

impl SettingsRepository for JsonSettings {
    fn get(&self) -> Settings {
        self.0.get()
    }

    fn set(&self, settings: Settings) -> Result<()> {
        self.0.update(|s| *s = settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_persist_per_worktree() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("comments.json");
        let c = ReviewComment {
            id: "c1".into(),
            file: "a.ts".into(),
            line: 1,
            end_line: Some(3),
            body: "x".into(),
            anchor: "a".into(),
            resolved: false,
            created_at: 1,
        };
        JsonComments::open(path.clone())
            .save("wt", std::slice::from_ref(&c))
            .unwrap();
        let reopened = JsonComments::open(path);
        assert_eq!(reopened.load("wt"), vec![c]);
        assert!(reopened.load("other").is_empty());
    }

    #[test]
    fn settings_default_to_nvim() {
        let dir = tempfile::tempdir().unwrap();
        let settings = JsonSettings::open(dir.path().join("settings.json"));
        assert_eq!(settings.get().editor_command, "nvim +{line} {file}");
        settings
            .set(Settings {
                editor_command: "code -g {file}:{line}".into(),
            })
            .unwrap();
        assert_eq!(
            JsonSettings::open(dir.path().join("settings.json"))
                .get()
                .editor_command,
            "code -g {file}:{line}"
        );
    }
}
