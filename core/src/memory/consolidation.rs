//! Memory consolidation and decay logic.

use crate::memory::types::ConsolidationReport;
use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Memory consolidation engine — decays old unaccessed memories.
#[derive(Clone)]
pub struct ConsolidationEngine {
    conn: Arc<Mutex<Connection>>,
    decay_rate: f32,
}

impl ConsolidationEngine {
    pub fn new(conn: Arc<Mutex<Connection>>, decay_rate: f32) -> Self {
        Self { conn, decay_rate }
    }

    /// Run a consolidation cycle: decay confidence of old memories.
    pub fn consolidate(&self) -> Result<ConsolidationReport> {
        let start = std::time::Instant::now();
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let cutoff = (Utc::now() - chrono::Duration::days(7)).to_rfc3339();
        let decay_factor = 1.0 - self.decay_rate as f64;

        let decayed = conn.execute(
            "UPDATE memories SET confidence = MAX(0.1, confidence * ?1) WHERE deleted = 0 AND accessed_at < ?2 AND confidence > 0.1",
            rusqlite::params![decay_factor, cutoff],
        )?;

        Ok(ConsolidationReport {
            memories_merged: 0,
            memories_decayed: decayed as u64,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::migration::run_migrations;

    fn setup() -> ConsolidationEngine {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        ConsolidationEngine::new(Arc::new(Mutex::new(conn)), 0.1)
    }

    #[test]
    fn test_empty() {
        let e = setup();
        let r = e.consolidate().unwrap();
        assert_eq!(r.memories_decayed, 0);
    }

    #[test]
    fn test_decays_old() {
        let e = setup();
        let conn = e.conn.lock().unwrap();
        let old = (Utc::now() - chrono::Duration::days(30)).to_rfc3339();
        conn.execute(
            "INSERT INTO memories (id,agent_id,content,source,scope,confidence,metadata,created_at,accessed_at,access_count,deleted) VALUES ('t','a','old','\"conversation\"','ep',0.9,'{}',?1,?1,0,0)",
            rusqlite::params![old],
        ).unwrap();
        drop(conn);
        let r = e.consolidate().unwrap();
        assert_eq!(r.memories_decayed, 1);
    }
}
