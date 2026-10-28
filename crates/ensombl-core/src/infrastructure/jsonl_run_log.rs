//! [`RunLog`] as one JSON-lines file per run. Appends survive crashes line by
//! line, and a damaged line is skipped instead of losing the whole log.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use crate::application::ports::RunLog;
use crate::application::{Error, Result};
use crate::domain::agent_run::RunEvent;

pub struct JsonlRunLog {
    dir: PathBuf,
}

impl JsonlRunLog {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn path(&self, run_id: &str) -> Result<PathBuf> {
        if run_id.is_empty()
            || !run_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(Error::Invalid(format!("bad run id {run_id:?}")));
        }
        Ok(self.dir.join(format!("{run_id}.jsonl")))
    }
}

impl RunLog for JsonlRunLog {
    fn append(&self, run_id: &str, event: &RunEvent) -> Result<()> {
        let path = self.path(run_id)?;
        std::fs::create_dir_all(&self.dir)?;
        let mut line = serde_json::to_string(event).map_err(|e| Error::Storage(e.to_string()))?;
        line.push('\n');
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?
            .write_all(line.as_bytes())?;
        Ok(())
    }

    fn read(&self, run_id: &str) -> Result<Vec<RunEvent>> {
        let text = match std::fs::read_to_string(self.path(run_id)?) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(e.into()),
        };
        Ok(text
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replays_appended_events_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let log = JsonlRunLog::new(dir.path().join("runs"));
        let events = [
            RunEvent::Session {
                session_id: "s".into(),
                model: None,
                permission_mode: None,
            },
            RunEvent::Exited { code: Some(0) },
        ];
        for e in &events {
            log.append("r1", e).unwrap();
        }
        assert_eq!(log.read("r1").unwrap(), events);
        assert!(log.read("r2").unwrap().is_empty());
    }

    #[test]
    fn skips_damaged_lines() {
        let dir = tempfile::tempdir().unwrap();
        let log = JsonlRunLog::new(dir.path().to_path_buf());
        log.append("r1", &RunEvent::Exited { code: None }).unwrap();
        std::fs::OpenOptions::new()
            .append(true)
            .open(dir.path().join("r1.jsonl"))
            .unwrap()
            .write_all(b"{not json\n")
            .unwrap();
        assert_eq!(log.read("r1").unwrap().len(), 1);
    }

    #[test]
    fn refuses_ids_that_escape_the_directory() {
        let log = JsonlRunLog::new(PathBuf::from("/tmp/x"));
        assert!(log.read("../etc/passwd").is_err());
    }
}
