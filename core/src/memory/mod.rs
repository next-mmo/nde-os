//! NDE-OS Persistent Memory Substrate
//!
//! SQLite-backed storage with vector embeddings. Cross-channel canonical
//! sessions, automatic LLM-based compaction, knowledge graph, and
//! usage tracking. Agents recall context across conversations and channels.
//!
//! # Architecture
//!
//! ```text
//! MemorySubstrate (facade)
//! ├── StructuredStore   — per-agent key-value pairs
//! ├── SemanticStore     — memories with optional vector embeddings
//! ├── KnowledgeStore    — entity/relation graph
//! ├── SessionStore      — sessions + canonical cross-channel memory
//! ├── ConsolidationEngine — decay old memories
//! └── UsageStore        — LLM cost tracking
//! ```
//!
//! The legacy `MemoryManager`, `ConversationStore`, and `KvStore` are
//! preserved for backward compatibility but new code should use
//! `MemorySubstrate` directly.

// Legacy stores (backward compat)
pub mod conversation;
pub mod kv;

// New memory substrate
pub mod consolidation;
pub mod knowledge;
pub mod migration;
pub mod semantic;
pub mod session;
pub mod structured;
pub mod substrate;
pub mod types;
pub mod usage;

// Re-exports — legacy
pub use conversation::ConversationStore;
pub use kv::KvStore;

// Re-exports — new substrate
pub use substrate::MemorySubstrate;
pub use types::*;

use anyhow::Result;
use std::path::Path;

/// Legacy unified memory manager — owns conversation store and KV store.
///
/// **Deprecated**: prefer `MemorySubstrate` for new code.
pub struct MemoryManager {
    pub conversations: ConversationStore,
    pub kv: KvStore,
}

impl MemoryManager {
    pub fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        let db_path = db_path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conversations = ConversationStore::new(db_path)?;
        let kv = KvStore::new(db_path)?;

        Ok(Self { conversations, kv })
    }
}
