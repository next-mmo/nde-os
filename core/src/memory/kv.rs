use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

/// SQLite-backed key-value store for agent state, preferences, and config.
pub struct KvStore {
    conn: Mutex<Connection>,
}

impl KvStore {
    pub fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(db_path.as_ref())
            .context("Failed to open KV database")?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS kv (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );"
        )?;

        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn get(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM kv WHERE key = ?1")?;
        let mut rows = stmt.query(rusqlite::params![key])?;

        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn set(&self, key: &str, value: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO kv (key, value, updated_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![key, value, now],
        )?;
        Ok(())
    }

    pub fn delete(&self, key: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let changed = conn.execute("DELETE FROM kv WHERE key = ?1", rusqlite::params![key])?;
        Ok(changed > 0)
    }

    pub fn list_keys(&self, prefix: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT key FROM kv WHERE key LIKE ?1 ORDER BY key")?;
        let pattern = format!("{}%", prefix);
        let rows = stmt.query_map(rusqlite::params![pattern], |row| row.get(0))?;

        let mut keys = Vec::new();
        for row in rows {
            keys.push(row?);
        }
        Ok(keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kv_crud() {
        let dir = tempfile::tempdir().unwrap();
        let kv = KvStore::new(dir.path().join("test.db")).unwrap();

        kv.set("key1", "value1").unwrap();
        assert_eq!(kv.get("key1").unwrap(), Some("value1".into()));
        assert_eq!(kv.get("missing").unwrap(), None);

        kv.set("key1", "updated").unwrap();
        assert_eq!(kv.get("key1").unwrap(), Some("updated".into()));

        assert!(kv.delete("key1").unwrap());
        assert_eq!(kv.get("key1").unwrap(), None);
    }
}
