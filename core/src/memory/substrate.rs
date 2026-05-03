//! Unified memory substrate — façade over all specialized stores.
//!
//! This is the main entry point for all memory operations in NDE-OS.
//! It manages a single SQLite connection shared across all stores.

use crate::memory::{
    consolidation::ConsolidationEngine,
    knowledge::KnowledgeStore,
    migration::run_migrations,
    semantic::SemanticStore,
    session::SessionStore,
    structured::StructuredStore,
    usage::UsageStore,
};
use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Unified memory substrate — owns all specialized stores.
///
/// Created once at startup and shared via `Arc<MemorySubstrate>`.
/// All stores share a single SQLite connection for consistency.
#[derive(Clone)]
pub struct MemorySubstrate {
    /// Per-agent key-value store.
    pub structured: StructuredStore,
    /// Semantic memory with optional vector search.
    pub semantic: SemanticStore,
    /// Knowledge graph (entities + relations).
    pub knowledge: KnowledgeStore,
    /// Session management + canonical cross-channel sessions.
    pub session: SessionStore,
    /// Memory decay and consolidation.
    pub consolidation: ConsolidationEngine,
    /// LLM usage tracking and cost monitoring.
    pub usage: UsageStore,
}

impl MemorySubstrate {
    /// Open (or create) the memory database at the given path.
    ///
    /// Runs all schema migrations on first boot.
    pub fn open(db_path: impl AsRef<Path>) -> Result<Self> {
        let db_path = db_path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn =
            Connection::open(db_path).context("Failed to open memory substrate database")?;

        // WAL mode for better concurrent read performance
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

        // Run schema migrations
        run_migrations(&conn)?;

        let conn = Arc::new(Mutex::new(conn));

        Ok(Self {
            structured: StructuredStore::new(Arc::clone(&conn)),
            semantic: SemanticStore::new(Arc::clone(&conn)),
            knowledge: KnowledgeStore::new(Arc::clone(&conn)),
            session: SessionStore::new(Arc::clone(&conn)),
            consolidation: ConsolidationEngine::new(Arc::clone(&conn), 0.05),
            usage: UsageStore::new(conn),
        })
    }

    /// Open an in-memory database (for testing).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        run_migrations(&conn)?;
        let conn = Arc::new(Mutex::new(conn));

        Ok(Self {
            structured: StructuredStore::new(Arc::clone(&conn)),
            semantic: SemanticStore::new(Arc::clone(&conn)),
            knowledge: KnowledgeStore::new(Arc::clone(&conn)),
            session: SessionStore::new(Arc::clone(&conn)),
            consolidation: ConsolidationEngine::new(Arc::clone(&conn), 0.05),
            usage: UsageStore::new(conn),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::types::*;
    use std::collections::HashMap;

    #[test]
    fn test_substrate_roundtrip() {
        let sub = MemorySubstrate::open_in_memory().unwrap();
        let aid = AgentId::new();

        // KV
        sub.structured
            .set(aid, "name", serde_json::json!("NDE-OS"))
            .unwrap();
        assert_eq!(
            sub.structured.get(aid, "name").unwrap(),
            Some(serde_json::json!("NDE-OS"))
        );

        // Semantic
        sub.semantic
            .remember(
                aid,
                "NDE-OS is a sandboxed virtual OS",
                MemorySource::System,
                "facts",
                HashMap::new(),
            )
            .unwrap();
        let recalled = sub.semantic.recall("sandboxed", 10, None).unwrap();
        assert_eq!(recalled.len(), 1);

        // Session
        let session = sub.session.create_session(aid).unwrap();
        assert!(sub.session.get_session(session.id).unwrap().is_some());

        // Canonical
        sub.session
            .append_canonical(
                aid,
                &[Message::user("Hello"), Message::assistant("Hi!")],
                None,
            )
            .unwrap();
        let (_, recent) = sub.session.canonical_context(aid, None).unwrap();
        assert_eq!(recent.len(), 2);

        // Usage
        sub.usage
            .record(&UsageRecord {
                agent_id: aid,
                model: "test".into(),
                input_tokens: 100,
                output_tokens: 50,
                cost_usd: 0.001,
                tool_calls: 1,
            })
            .unwrap();
        let summary = sub.usage.query_summary(Some(aid)).unwrap();
        assert_eq!(summary.call_count, 1);

        // Consolidation
        let report = sub.consolidation.consolidate().unwrap();
        assert_eq!(report.memories_decayed, 0);
    }
}
