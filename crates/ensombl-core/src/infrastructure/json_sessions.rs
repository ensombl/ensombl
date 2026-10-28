//! [`SessionRepository`] persisted as JSON in the app data directory.

use std::path::PathBuf;
use std::sync::Mutex;

use crate::application::ports::SessionRepository;
use crate::application::{Error, Result};
use crate::domain::workspace::Session;

pub struct JsonSessions {
    path: PathBuf,
    cache: Mutex<Session>,
}

impl JsonSessions {
    /// Loads the session from `path`, starting empty if it's missing or unreadable.
    pub fn open(path: PathBuf) -> Self {
        let session = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Self {
            path,
            cache: Mutex::new(session),
        }
    }

    fn persist(&self, session: &Session) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_vec_pretty(session).map_err(|e| Error::Storage(e.to_string()))?;
        // Write then rename so a crash never leaves a truncated file.
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(tmp, &self.path)?;
        Ok(())
    }
}

impl SessionRepository for JsonSessions {
    fn get(&self) -> Result<Session> {
        Ok(self
            .cache
            .lock()
            .map_err(|e| Error::Storage(e.to_string()))?
            .clone())
    }

    fn update(&self, f: &mut dyn FnMut(&mut Session) -> Result<()>) -> Result<()> {
        let mut guard = self
            .cache
            .lock()
            .map_err(|e| Error::Storage(e.to_string()))?;
        let mut next = guard.clone();
        f(&mut next)?;
        self.persist(&next)?;
        *guard = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_across_instances() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        JsonSessions::open(path.clone())
            .update(&mut |s| {
                s.add_project("p".into(), "proj".into(), "/p".into());
                Ok(())
            })
            .unwrap();
        assert_eq!(
            JsonSessions::open(path).get().unwrap().projects[0].name,
            "proj"
        );
    }

    #[test]
    fn failed_updates_leave_state_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let repo = JsonSessions::open(dir.path().join("session.json"));
        let result = repo.update(&mut |s| {
            s.add_project("p".into(), "proj".into(), "/p".into());
            Err(Error::Invalid("nope".into()))
        });
        assert!(result.is_err());
        assert!(repo.get().unwrap().projects.is_empty());
    }
}
