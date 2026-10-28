//! [`ParseCache`] stored in SQLite, keyed by worktree, path and content hash.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{Connection, params};

use crate::application::ports::ParseCache;
use crate::domain::parsed::ParsedFile;

pub struct SqliteCache {
    conn: Mutex<Connection>,
}

impl SqliteCache {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        Self::init(Connection::open(path)?)
    }

    pub fn in_memory() -> rusqlite::Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> rusqlite::Result<Self> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS parsed (
               worktree TEXT NOT NULL,
               path TEXT NOT NULL,
               hash TEXT NOT NULL,
               data BLOB NOT NULL,
               PRIMARY KEY (worktree, path)
             );",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

impl ParseCache for SqliteCache {
    fn get(&self, worktree: &str, path: &str, hash: &str) -> Option<ParsedFile> {
        let conn = self.conn.lock().ok()?;
        let data: Vec<u8> = conn
            .query_row(
                "SELECT data FROM parsed WHERE worktree = ?1 AND path = ?2 AND hash = ?3",
                params![worktree, path, hash],
                |row| row.get(0),
            )
            .ok()?;
        serde_json::from_slice(&data).ok()
    }

    fn put_many(&self, worktree: &str, entries: &[(String, ParsedFile)]) {
        let Ok(mut conn) = self.conn.lock() else {
            return;
        };
        let Ok(tx) = conn.transaction() else { return };
        {
            let Ok(mut stmt) = tx.prepare(
                "INSERT OR REPLACE INTO parsed (worktree, path, hash, data) VALUES (?1, ?2, ?3, ?4)",
            ) else {
                return;
            };
            for (hash, file) in entries {
                if let Ok(data) = serde_json::to_vec(file) {
                    let _ = stmt.execute(params![worktree, file.path, hash, data]);
                }
            }
        }
        let _ = tx.commit();
    }

    fn forget(&self, worktree: &str, paths: &[String]) {
        let Ok(conn) = self.conn.lock() else { return };
        for path in paths {
            let _ = conn.execute(
                "DELETE FROM parsed WHERE worktree = ?1 AND path = ?2",
                params![worktree, path],
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_by_hash() {
        let cache = SqliteCache::in_memory().unwrap();
        let file = ParsedFile {
            path: "a.ts".into(),
            default_export: Some("A".into()),
            ..Default::default()
        };
        cache.put_many("wt", &[("h1".into(), file.clone())]);
        assert_eq!(cache.get("wt", "a.ts", "h1"), Some(file));
        assert_eq!(cache.get("wt", "a.ts", "h2"), None, "stale hash misses");
        assert_eq!(cache.get("other", "a.ts", "h1"), None);
        cache.forget("wt", &["a.ts".into()]);
        assert_eq!(cache.get("wt", "a.ts", "h1"), None);
    }
}
