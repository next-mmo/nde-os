//! Cron-based autonomous task scheduler.
//!
//! Schedules are stored in SQLite and checked every 60 seconds.
//! When a schedule fires, it spawns a task via the AgentManager.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use tokio_util::sync::CancellationToken;

/// A scheduled agent task definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub id: String,
    pub cron_expr: String,
    pub input: String,
    pub enabled: bool,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
    pub run_count: u32,
    pub created_at: DateTime<Utc>,
}

/// Persistent schedule store.
pub struct ScheduleStore {
    conn: StdMutex<Connection>,
}

impl ScheduleStore {
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;

             CREATE TABLE IF NOT EXISTS schedules (
                 id TEXT PRIMARY KEY,
                 cron_expr TEXT NOT NULL,
                 input TEXT NOT NULL,
                 enabled INTEGER NOT NULL DEFAULT 1,
                 last_run TEXT,
                 next_run TEXT,
                 run_count INTEGER NOT NULL DEFAULT 0,
                 created_at TEXT NOT NULL
             );"
        )?;
        Ok(Self { conn: StdMutex::new(conn) })
    }

    pub fn add(&self, cron_expr: &str, input: &str) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        let next = compute_next_run(cron_expr, &now)?;

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO schedules (id, cron_expr, input, enabled, next_run, created_at)
             VALUES (?1, ?2, ?3, 1, ?4, ?5)",
            params![id, cron_expr, input, next.map(|t| t.to_rfc3339()), now.to_rfc3339()],
        )?;
        Ok(id)
    }

    pub fn remove(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM schedules WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn set_enabled(&self, id: &str, enabled: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE schedules SET enabled = ?1 WHERE id = ?2",
            params![enabled as i32, id],
        )?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<Schedule>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, cron_expr, input, enabled, last_run, next_run, run_count, created_at
             FROM schedules ORDER BY created_at DESC"
        )?;

        let schedules = stmt.query_map([], |row| {
            let parse_dt = |s: Option<String>| -> Option<DateTime<Utc>> {
                s.and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc))
            };

            Ok(Schedule {
                id: row.get(0)?,
                cron_expr: row.get(1)?,
                input: row.get(2)?,
                enabled: row.get::<_, i32>(3)? != 0,
                last_run: parse_dt(row.get(4)?),
                next_run: parse_dt(row.get(5)?),
                run_count: row.get(6)?,
                created_at: {
                    let s: String = row.get(7)?;
                    DateTime::parse_from_rfc3339(&s)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now())
                },
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

        Ok(schedules)
    }

    pub fn get_due(&self, now: &DateTime<Utc>) -> Result<Vec<Schedule>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, cron_expr, input, enabled, last_run, next_run, run_count, created_at
             FROM schedules
             WHERE enabled = 1 AND next_run IS NOT NULL AND next_run <= ?1"
        )?;

        let schedules = stmt.query_map(params![now.to_rfc3339()], |row| {
            let parse_dt = |s: Option<String>| -> Option<DateTime<Utc>> {
                s.and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc))
            };

            Ok(Schedule {
                id: row.get(0)?,
                cron_expr: row.get(1)?,
                input: row.get(2)?,
                enabled: true,
                last_run: parse_dt(row.get(4)?),
                next_run: parse_dt(row.get(5)?),
                run_count: row.get(6)?,
                created_at: {
                    let s: String = row.get(7)?;
                    DateTime::parse_from_rfc3339(&s)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now())
                },
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

        Ok(schedules)
    }

    pub fn mark_run(&self, id: &str, cron_expr: &str) -> Result<()> {
        let now = Utc::now();
        let next = compute_next_run(cron_expr, &now)?;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE schedules SET last_run = ?1, next_run = ?2, run_count = run_count + 1
             WHERE id = ?3",
            params![
                now.to_rfc3339(),
                next.map(|t| t.to_rfc3339()),
                id
            ],
        )?;
        Ok(())
    }
}

/// Simple cron-like expression parser.
/// Supports: `@every <N>m`, `@every <N>h`, `@every <N>s`, `@daily`, `@hourly`.
fn compute_next_run(cron_expr: &str, from: &DateTime<Utc>) -> Result<Option<DateTime<Utc>>> {
    let expr = cron_expr.trim().to_lowercase();

    if expr.starts_with("@every ") {
        let rest = expr.trim_start_matches("@every ").trim();
        let (num_str, unit) = if rest.ends_with('m') {
            (&rest[..rest.len() - 1], "m")
        } else if rest.ends_with('h') {
            (&rest[..rest.len() - 1], "h")
        } else if rest.ends_with('s') {
            (&rest[..rest.len() - 1], "s")
        } else {
            return Err(anyhow!("Invalid interval: {}", rest));
        };

        let n: u64 = num_str
            .parse()
            .map_err(|_| anyhow!("Invalid number in cron: {}", num_str))?;

        let secs = match unit {
            "s" => n,
            "m" => n * 60,
            "h" => n * 3600,
            _ => return Err(anyhow!("Invalid interval unit: {}", unit)),
        };

        let next = *from + chrono::Duration::seconds(secs as i64);
        return Ok(Some(next));
    }

    match expr.as_str() {
        "@daily" => {
            let next = *from + chrono::Duration::hours(24);
            Ok(Some(next))
        }
        "@hourly" => {
            let next = *from + chrono::Duration::hours(1);
            Ok(Some(next))
        }
        _ => Err(anyhow!(
            "Unsupported cron expression: {}. Use @every <N>m/h/s, @daily, or @hourly",
            cron_expr
        )),
    }
}

/// Handle for stopping the scheduler background task.
pub struct SchedulerHandle {
    cancel: CancellationToken,
}

impl SchedulerHandle {
    pub fn stop(&self) {
        self.cancel.cancel();
    }
}

/// Callback type for spawning tasks from the scheduler.
pub type SpawnFn = Arc<dyn Fn(String) -> tokio::task::JoinHandle<()> + Send + Sync>;

/// Start the scheduler background loop.
pub fn start_scheduler(
    store: Arc<ScheduleStore>,
    spawn_fn: SpawnFn,
    tick_interval: Duration,
) -> SchedulerHandle {
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(tick_interval) => {},
                _ = cancel_clone.cancelled() => break,
            }

            let now = Utc::now();
            match store.get_due(&now) {
                Ok(due) => {
                    for schedule in due {
                        tracing::info!(
                            schedule_id = %schedule.id,
                            input = %schedule.input,
                            "Firing scheduled task"
                        );
                        (spawn_fn)(schedule.input.clone());
                        if let Err(e) = store.mark_run(&schedule.id, &schedule.cron_expr) {
                            tracing::error!(error = %e, "Failed to mark schedule run");
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to check due schedules");
                }
            }
        }
        tracing::info!("Scheduler stopped");
    });

    SchedulerHandle { cancel }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_every_minutes() {
        let now = Utc::now();
        let next = compute_next_run("@every 5m", &now).unwrap().unwrap();
        let diff = (next - now).num_seconds();
        assert_eq!(diff, 300);
    }

    #[test]
    fn test_cron_every_hours() {
        let now = Utc::now();
        let next = compute_next_run("@every 2h", &now).unwrap().unwrap();
        let diff = (next - now).num_seconds();
        assert_eq!(diff, 7200);
    }

    #[test]
    fn test_cron_daily() {
        let now = Utc::now();
        let next = compute_next_run("@daily", &now).unwrap().unwrap();
        let diff = (next - now).num_seconds();
        assert_eq!(diff, 86400);
    }

    #[test]
    fn test_cron_invalid() {
        let now = Utc::now();
        assert!(compute_next_run("invalid", &now).is_err());
    }

    #[test]
    fn test_schedule_store() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("schedules.db");
        let store = ScheduleStore::new(&db).unwrap();

        let id = store.add("@every 10m", "Check system status").unwrap();
        let list = store.list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].input, "Check system status");

        store.remove(&id).unwrap();
        let list = store.list().unwrap();
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn test_schedule_due() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("schedules.db");
        let store = ScheduleStore::new(&db).unwrap();

        // Add a schedule that's due in 1 second
        store.add("@every 1s", "Quick task").unwrap();

        // Not due yet (use exact now)
        let now = Utc::now();
        let due = store.get_due(&now).unwrap();
        assert_eq!(due.len(), 0);

        // Due in 2 seconds
        let future = now + chrono::Duration::seconds(2);
        let due = store.get_due(&future).unwrap();
        assert_eq!(due.len(), 1);
    }
}
