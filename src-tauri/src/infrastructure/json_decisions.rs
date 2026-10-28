//! [`PatternDecisions`] stored as JSON in the app data dir (never in the user's repo).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::application::ports::PatternDecisions;
use crate::application::{Error, Result};
use crate::domain::patterns::PatternDecision;

type Decisions = HashMap<String, HashMap<String, PatternDecision>>;

pub struct JsonDecisions {
    path: PathBuf,
    cache: Mutex<Decisions>,
}

impl JsonDecisions {
    pub fn open(path: PathBuf) -> Self {
        let cache = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Self {
            path,
            cache: Mutex::new(cache),
        }
    }
}

impl PatternDecisions for JsonDecisions {
    fn load(&self, project: &str) -> HashMap<String, PatternDecision> {
        self.cache
            .lock()
            .unwrap()
            .get(project)
            .cloned()
            .unwrap_or_default()
    }

    fn set(
        &self,
        project: &str,
        pattern_id: &str,
        decision: Option<PatternDecision>,
    ) -> Result<()> {
        let mut cache = self.cache.lock().unwrap();
        let entry = cache.entry(project.into()).or_default();
        match decision {
            Some(d) => entry.insert(pattern_id.into(), d),
            None => entry.remove(pattern_id),
        };
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_vec_pretty(&*cache).map_err(|e| Error::Storage(e.to_string()))?;
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(tmp, &self.path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_and_clears_decisions_per_project() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("patterns.json");
        let store = JsonDecisions::open(path.clone());
        store
            .set("p1", "strategy:x", Some(PatternDecision::Confirmed))
            .unwrap();
        store
            .set("p2", "strategy:x", Some(PatternDecision::Dismissed))
            .unwrap();

        let reopened = JsonDecisions::open(path);
        assert_eq!(
            reopened.load("p1").get("strategy:x"),
            Some(&PatternDecision::Confirmed)
        );
        assert_eq!(
            reopened.load("p2").get("strategy:x"),
            Some(&PatternDecision::Dismissed)
        );

        reopened.set("p1", "strategy:x", None).unwrap();
        assert!(reopened.load("p1").is_empty());
    }
}
