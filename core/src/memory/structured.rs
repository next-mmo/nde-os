//! SQLite structured store for key-value pairs (per-agent).

use crate::memory::types::AgentId;
use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Structured store backed by SQLite for per-agent key-value operations.
#[derive(Clone)]
pub struct StructuredStore {
    conn: Arc<Mutex<Connection>>,
}

impl StructuredStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Get a value from the key-value store.
    pub fn get(&self, agent_id: AgentId, key: &str) -> Result<Option<serde_json::Value>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let mut stmt = conn.prepare("SELECT value FROM kv_store WHERE agent_id = ?1 AND key = ?2")?;
        let result = stmt.query_row(rusqlite::params![agent_id.0.to_string(), key], |row| {
            let blob: Vec<u8> = row.get(0)?;
            Ok(blob)
        });
        match result {
            Ok(blob) => {
                let value: serde_json::Value =
                    serde_json::from_slice(&blob).context("deserialize KV value")?;
                Ok(Some(value))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Set a value in the key-value store.
    pub fn set(&self, agent_id: AgentId, key: &str, value: serde_json::Value) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let blob = serde_json::to_vec(&value)?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO kv_store (agent_id, key, value, version, updated_at) VALUES (?1, ?2, ?3, 1, ?4)
             ON CONFLICT(agent_id, key) DO UPDATE SET value = ?3, version = version + 1, updated_at = ?4",
            rusqlite::params![agent_id.0.to_string(), key, blob, now],
        )?;
        Ok(())
    }

    /// Delete a value from the key-value store.
    pub fn delete(&self, agent_id: AgentId, key: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        conn.execute(
            "DELETE FROM kv_store WHERE agent_id = ?1 AND key = ?2",
            rusqlite::params![agent_id.0.to_string(), key],
        )?;
        Ok(())
    }

    /// List all key-value pairs for an agent.
    pub fn list_kv(&self, agent_id: AgentId) -> Result<Vec<(String, serde_json::Value)>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let mut stmt =
            conn.prepare("SELECT key, value FROM kv_store WHERE agent_id = ?1 ORDER BY key")?;
        let rows = stmt.query_map(rusqlite::params![agent_id.0.to_string()], |row| {
            let key: String = row.get(0)?;
            let blob: Vec<u8> = row.get(1)?;
            Ok((key, blob))
        })?;

        let mut pairs = Vec::new();
        for row in rows {
            let (key, blob) = row?;
            let value: serde_json::Value = serde_json::from_slice(&blob).unwrap_or_else(|_| {
                String::from_utf8(blob)
                    .map(serde_json::Value::String)
                    .unwrap_or(serde_json::Value::Null)
            });
            pairs.push((key, value));
        }
        Ok(pairs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::migration::run_migrations;

    fn setup() -> StructuredStore {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        StructuredStore::new(Arc::new(Mutex::new(conn)))
    }

    #[test]
    fn test_kv_set_get() {
        let store = setup();
        let agent_id = AgentId::new();
        store
            .set(agent_id, "test_key", serde_json::json!("test_value"))
            .unwrap();
        let value = store.get(agent_id, "test_key").unwrap();
        assert_eq!(value, Some(serde_json::json!("test_value")));
    }

    #[test]
    fn test_kv_get_missing() {
        let store = setup();
        let agent_id = AgentId::new();
        let value = store.get(agent_id, "nonexistent").unwrap();
        assert!(value.is_none());
    }

    #[test]
    fn test_kv_delete() {
        let store = setup();
        let agent_id = AgentId::new();
        store
            .set(agent_id, "to_delete", serde_json::json!(42))
            .unwrap();
        store.delete(agent_id, "to_delete").unwrap();
        let value = store.get(agent_id, "to_delete").unwrap();
        assert!(value.is_none());
    }

    #[test]
    fn test_kv_update() {
        let store = setup();
        let agent_id = AgentId::new();
        store.set(agent_id, "key", serde_json::json!("v1")).unwrap();
        store.set(agent_id, "key", serde_json::json!("v2")).unwrap();
        let value = store.get(agent_id, "key").unwrap();
        assert_eq!(value, Some(serde_json::json!("v2")));
    }
}
