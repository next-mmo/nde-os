//! Persistent SQLite task store for crash recovery.
//!
//! Every task is persisted so the agent manager can restore incomplete tasks
//! after a server restart, providing true 24/7 operation.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

use super::models::{AgentTask, TaskFilter, TaskState};
use crate::llm::Message;

/// SQLite-backed task store.
pub struct TaskStore {
    conn: Mutex<Connection>,
}

impl TaskStore {
    /// Open or create the task store database.
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)
            .with_context(|| format!("Failed to open task store: {}", db_path.display()))?;

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                state TEXT NOT NULL DEFAULT 'pending',
                input TEXT NOT NULL,
                output TEXT,
                error TEXT,
                conversation_id TEXT,
                parent_task_id TEXT,
                retry_count INTEGER NOT NULL DEFAULT 0,
                max_retries INTEGER NOT NULL DEFAULT 3,
                timeout_secs INTEGER NOT NULL DEFAULT 300,
                iterations INTEGER NOT NULL DEFAULT 0,
                tokens_used INTEGER NOT NULL DEFAULT 0,
                tool_calls_made INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                started_at TEXT,
                completed_at TEXT
            );

            CREATE TABLE IF NOT EXISTS task_checkpoints (
                task_id TEXT PRIMARY KEY,
                messages_json TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_tasks_state ON tasks(state);
            CREATE INDEX IF NOT EXISTS idx_tasks_conversation ON tasks(conversation_id);
            CREATE INDEX IF NOT EXISTS idx_tasks_created ON tasks(created_at);",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// In-memory store for testing.
    #[cfg(test)]
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        // Re-run schema creation
        store.conn.lock().unwrap().execute_batch(
            "CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                state TEXT NOT NULL DEFAULT 'pending',
                input TEXT NOT NULL,
                output TEXT,
                error TEXT,
                conversation_id TEXT,
                parent_task_id TEXT,
                retry_count INTEGER NOT NULL DEFAULT 0,
                max_retries INTEGER NOT NULL DEFAULT 3,
                timeout_secs INTEGER NOT NULL DEFAULT 300,
                iterations INTEGER NOT NULL DEFAULT 0,
                tokens_used INTEGER NOT NULL DEFAULT 0,
                tool_calls_made INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                started_at TEXT,
                completed_at TEXT
            );
            CREATE TABLE IF NOT EXISTS task_checkpoints (
                task_id TEXT PRIMARY KEY,
                messages_json TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
            );",
        )?;
        Ok(store)
    }

    /// Save or update a task.
    pub fn save_task(&self, task: &AgentTask) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO tasks
             (id, state, input, output, error, conversation_id, parent_task_id,
              retry_count, max_retries, timeout_secs, iterations, tokens_used,
              tool_calls_made, created_at, started_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                task.id,
                task.state.to_string(),
                task.input,
                task.output,
                task.error,
                task.conversation_id,
                task.parent_task_id,
                task.retry_count,
                task.max_retries,
                task.timeout_secs,
                task.iterations,
                task.tokens_used,
                task.tool_calls_made,
                task.created_at.to_rfc3339(),
                task.started_at.map(|t| t.to_rfc3339()),
                task.completed_at.map(|t| t.to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    /// Load a task by ID.
    pub fn load_task(&self, id: &str) -> Result<Option<AgentTask>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, state, input, output, error, conversation_id, parent_task_id,
                    retry_count, max_retries, timeout_secs, iterations, tokens_used,
                    tool_calls_made, created_at, started_at, completed_at
             FROM tasks WHERE id = ?1",
        )?;

        let result = stmt
            .query_row(params![id], |row| Ok(Self::row_to_task(row)))
            .optional()?;

        match result {
            Some(task) => Ok(Some(task?)),
            None => Ok(None),
        }
    }

    /// List tasks matching a filter.
    pub fn list_tasks(&self, filter: &TaskFilter) -> Result<Vec<AgentTask>> {
        let conn = self.conn.lock().unwrap();
        let mut sql = String::from(
            "SELECT id, state, input, output, error, conversation_id, parent_task_id,
                    retry_count, max_retries, timeout_secs, iterations, tokens_used,
                    tool_calls_made, created_at, started_at, completed_at
             FROM tasks WHERE 1=1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(state) = &filter.state {
            sql.push_str(&format!(" AND state = ?{}", param_values.len() + 1));
            param_values.push(Box::new(state.to_string()));
        }
        if let Some(conv_id) = &filter.conversation_id {
            sql.push_str(&format!(
                " AND conversation_id = ?{}",
                param_values.len() + 1
            ));
            param_values.push(Box::new(conv_id.clone()));
        }

        sql.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = filter.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        let mut stmt = conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let tasks = stmt
            .query_map(params_refs.as_slice(), |row| Ok(Self::row_to_task(row)))?
            .filter_map(|r| r.ok())
            .filter_map(|r| r.ok())
            .collect();

        Ok(tasks)
    }

    /// Update only the state of a task.
    pub fn update_state(&self, id: &str, state: TaskState) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let completed_at = if state.is_terminal() {
            Some(now.clone())
        } else {
            None
        };

        conn.execute(
            "UPDATE tasks SET state = ?1, completed_at = COALESCE(?2, completed_at) WHERE id = ?3",
            params![state.to_string(), completed_at, id],
        )?;
        Ok(())
    }

    /// Save a mid-execution checkpoint (message history) for crash recovery.
    pub fn save_checkpoint(&self, task_id: &str, messages: &[Message]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let json = serde_json::to_string(messages)?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR REPLACE INTO task_checkpoints (task_id, messages_json, updated_at)
             VALUES (?1, ?2, ?3)",
            params![task_id, json, now],
        )?;
        Ok(())
    }

    /// Load a checkpoint for task resumption.
    pub fn load_checkpoint(&self, task_id: &str) -> Result<Option<Vec<Message>>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT messages_json FROM task_checkpoints WHERE task_id = ?1")?;

        let result: Option<String> = stmt
            .query_row(params![task_id], |row| row.get(0))
            .optional()?;

        match result {
            Some(json) => {
                let messages: Vec<Message> = serde_json::from_str(&json)?;
                Ok(Some(messages))
            }
            None => Ok(None),
        }
    }

    /// Delete old completed/failed tasks older than `days`.
    pub fn cleanup_old(&self, days: u32) -> Result<u32> {
        let conn = self.conn.lock().unwrap();
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days as i64);
        let deleted = conn.execute(
            "DELETE FROM tasks WHERE state IN ('completed', 'failed', 'cancelled', 'timed_out')
             AND completed_at < ?1",
            params![cutoff.to_rfc3339()],
        )?;
        Ok(deleted as u32)
    }

    /// Get incomplete tasks (for recovery on boot).
    pub fn get_incomplete_tasks(&self) -> Result<Vec<AgentTask>> {
        self.list_tasks(&TaskFilter {
            state: Some(TaskState::Running),
            ..Default::default()
        })
    }

    // ── Private helpers ─────────────────────────────────────────────────────

    fn row_to_task(row: &rusqlite::Row<'_>) -> Result<AgentTask> {
        let state_str: String = row.get(1)?;
        let state = match state_str.as_str() {
            "pending" => TaskState::Pending,
            "running" => TaskState::Running,
            "paused" => TaskState::Paused,
            "completed" => TaskState::Completed,
            "failed" => TaskState::Failed,
            "cancelled" => TaskState::Cancelled,
            "timed_out" => TaskState::TimedOut,
            _ => TaskState::Failed,
        };

        let parse_dt = |s: Option<String>| -> Option<chrono::DateTime<chrono::Utc>> {
            s.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc))
        };

        Ok(AgentTask {
            id: row.get(0)?,
            state,
            input: row.get(2)?,
            output: row.get(3)?,
            error: row.get(4)?,
            conversation_id: row.get(5)?,
            parent_task_id: row.get(6)?,
            retry_count: row.get(7)?,
            max_retries: row.get(8)?,
            timeout_secs: row.get::<_, i64>(9)? as u64,
            iterations: row.get(10)?,
            tokens_used: row.get::<_, i64>(11)? as u64,
            tool_calls_made: row.get(12)?,
            created_at: {
                let s: String = row.get(13)?;
                chrono::DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now())
            },
            started_at: parse_dt(row.get(14)?),
            completed_at: parse_dt(row.get(15)?),
        })
    }
}

// We need this for rusqlite row operations
use rusqlite::OptionalExtension;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_save_and_load() {
        let store = TaskStore::in_memory().unwrap();
        let mut task = AgentTask::new("Hello world");
        task.mark_running();

        store.save_task(&task).unwrap();

        let loaded = store.load_task(&task.id).unwrap().unwrap();
        assert_eq!(loaded.id, task.id);
        assert_eq!(loaded.state, TaskState::Running);
        assert_eq!(loaded.input, "Hello world");
    }

    #[test]
    fn test_store_update_state() {
        let store = TaskStore::in_memory().unwrap();
        let task = AgentTask::new("test");
        store.save_task(&task).unwrap();

        store.update_state(&task.id, TaskState::Running).unwrap();
        let loaded = store.load_task(&task.id).unwrap().unwrap();
        assert_eq!(loaded.state, TaskState::Running);

        store.update_state(&task.id, TaskState::Completed).unwrap();
        let loaded = store.load_task(&task.id).unwrap().unwrap();
        assert_eq!(loaded.state, TaskState::Completed);
    }

    #[test]
    fn test_store_checkpoint() {
        let store = TaskStore::in_memory().unwrap();
        let task = AgentTask::new("test");
        store.save_task(&task).unwrap();

        let messages = vec![
            Message::system("You are an assistant"),
            Message::user("Hello"),
            Message::assistant_text("Hi there!"),
        ];
        store.save_checkpoint(&task.id, &messages).unwrap();

        let loaded = store.load_checkpoint(&task.id).unwrap().unwrap();
        assert_eq!(loaded.len(), 3);
    }

    #[test]
    fn test_store_list_filter() {
        let store = TaskStore::in_memory().unwrap();

        let mut t1 = AgentTask::new("task 1");
        t1.mark_running();
        store.save_task(&t1).unwrap();

        let mut t2 = AgentTask::new("task 2");
        t2.mark_completed("done");
        store.save_task(&t2).unwrap();

        let running = store
            .list_tasks(&TaskFilter {
                state: Some(TaskState::Running),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(running.len(), 1);
        assert_eq!(running[0].id, t1.id);
    }

    #[test]
    fn test_store_incomplete_tasks() {
        let store = TaskStore::in_memory().unwrap();

        let mut t1 = AgentTask::new("running task");
        t1.mark_running();
        store.save_task(&t1).unwrap();

        let t2 = AgentTask::new("pending task");
        store.save_task(&t2).unwrap();

        let incomplete = store.get_incomplete_tasks().unwrap();
        assert_eq!(incomplete.len(), 1); // Only running, not pending
    }
}
