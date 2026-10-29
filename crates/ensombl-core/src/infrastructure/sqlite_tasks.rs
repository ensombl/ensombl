//! [`TaskStore`] in SQLite. Rows hold the JSON of each task or run, so the
//! model can grow without migrations.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{Connection, OptionalExtension, params};

use crate::application::ports::TaskStore;
use crate::application::{Error, Result};
use crate::domain::orchestration::{Run, Task};

pub struct SqliteTasks {
    conn: Mutex<Connection>,
}

fn storage(e: impl std::fmt::Display) -> Error {
    Error::Storage(e.to_string())
}

impl SqliteTasks {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        Self::init(Connection::open(path).map_err(storage)?)
    }

    pub fn in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory().map_err(storage)?)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS tasks (
               id TEXT PRIMARY KEY,
               created_at INTEGER NOT NULL,
               data TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS runs (
               id TEXT PRIMARY KEY,
               task_id TEXT NOT NULL,
               started_at INTEGER NOT NULL,
               data TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS runs_by_task ON runs (task_id, started_at);",
        )
        .map_err(storage)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn rows<T: serde::de::DeserializeOwned>(
        &self,
        sql: &str,
        args: impl rusqlite::Params,
    ) -> Result<Vec<T>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(sql).map_err(storage)?;
        let rows = stmt
            .query_map(args, |row| row.get::<_, String>(0))
            .map_err(storage)?;
        rows.map(|json| serde_json::from_str(&json.map_err(storage)?).map_err(storage))
            .collect()
    }
}

impl TaskStore for SqliteTasks {
    fn list(&self) -> Result<Vec<Task>> {
        self.rows("SELECT data FROM tasks ORDER BY created_at, id", [])
    }

    fn get(&self, id: &str) -> Result<Option<Task>> {
        let json: Option<String> = self
            .conn
            .lock()
            .unwrap()
            .query_row("SELECT data FROM tasks WHERE id = ?1", [id], |r| r.get(0))
            .optional()
            .map_err(storage)?;
        json.map(|j| serde_json::from_str(&j).map_err(storage))
            .transpose()
    }

    fn put(&self, task: &Task) -> Result<()> {
        let json = serde_json::to_string(task).map_err(storage)?;
        self.conn
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO tasks (id, created_at, data) VALUES (?1, ?2, ?3)
                 ON CONFLICT(id) DO UPDATE SET data = excluded.data",
                params![task.id, task.created_at as i64, json],
            )
            .map_err(storage)?;
        Ok(())
    }

    fn delete(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM runs WHERE task_id = ?1", [id])
            .map_err(storage)?;
        conn.execute("DELETE FROM tasks WHERE id = ?1", [id])
            .map_err(storage)?;
        Ok(())
    }

    fn runs(&self, task_id: &str) -> Result<Vec<Run>> {
        self.rows(
            "SELECT data FROM runs WHERE task_id = ?1 ORDER BY started_at, id",
            [task_id],
        )
    }

    fn put_run(&self, run: &Run) -> Result<()> {
        let json = serde_json::to_string(run).map_err(storage)?;
        self.conn
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO runs (id, task_id, started_at, data) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET data = excluded.data",
                params![run.id, run.task_id, run.started_at as i64, json],
            )
            .map_err(storage)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agent_run::PermissionMode;
    use crate::domain::orchestration::{RunOutcome, TaskStatus};
    use crate::domain::terminal::AgentKind;

    fn task(id: &str, created_at: u64) -> Task {
        Task {
            id: id.into(),
            project_id: "p".into(),
            title: "t".into(),
            prompt: "do it".into(),
            agent: AgentKind::Claude,
            base_branch: "main".into(),
            branch: format!("ensombl/t-{id}"),
            worktree: None,
            status: TaskStatus::Backlog,
            depends_on: vec![],
            permission: PermissionMode::AcceptEdits,
            session_id: None,
            next_prompt: None,
            auto_approve: false,
            workflow: None,
            summary: None,
            error: None,
            created_at,
            updated_at: created_at,
        }
    }

    fn run(id: &str, task_id: &str, started_at: u64) -> Run {
        Run {
            id: id.into(),
            task_id: task_id.into(),
            prompt: "do it".into(),
            started_at,
            ended_at: None,
            outcome: None,
            cost_usd: None,
            input_tokens: None,
            output_tokens: None,
        }
    }

    #[test]
    fn tasks_round_trip_in_creation_order() {
        let store = SqliteTasks::in_memory().unwrap();
        store.put(&task("b", 2)).unwrap();
        store.put(&task("a", 1)).unwrap();
        let mut queued = task("b", 2);
        queued.status = TaskStatus::Queued;
        store.put(&queued).unwrap();
        let ids: Vec<_> = store.list().unwrap().into_iter().map(|t| t.id).collect();
        assert_eq!(ids, ["a", "b"]);
        assert_eq!(store.get("b").unwrap().unwrap().status, TaskStatus::Queued);
        assert_eq!(store.get("missing").unwrap(), None);
    }

    #[test]
    fn runs_belong_to_their_task_and_go_with_it() {
        let store = SqliteTasks::in_memory().unwrap();
        store.put(&task("a", 1)).unwrap();
        store.put_run(&run("r2", "a", 20)).unwrap();
        store.put_run(&run("r1", "a", 10)).unwrap();
        store.put_run(&run("x", "other", 5)).unwrap();
        let mut done = run("r1", "a", 10);
        done.outcome = Some(RunOutcome::Completed);
        store.put_run(&done).unwrap();
        let runs = store.runs("a").unwrap();
        assert_eq!(
            runs.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            ["r1", "r2"]
        );
        assert_eq!(runs[0].outcome, Some(RunOutcome::Completed));
        store.delete("a").unwrap();
        assert!(store.runs("a").unwrap().is_empty());
        assert_eq!(store.get("a").unwrap(), None);
    }

    #[test]
    fn persists_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("o.db");
        SqliteTasks::open(&path)
            .unwrap()
            .put(&task("a", 1))
            .unwrap();
        assert_eq!(SqliteTasks::open(&path).unwrap().list().unwrap().len(), 1);
    }
}
