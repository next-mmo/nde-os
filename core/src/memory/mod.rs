pub mod conversation;
pub mod kv;

pub use conversation::ConversationStore;
pub use kv::KvStore;

use anyhow::Result;
use std::path::Path;

/// Unified memory manager — owns conversation store and KV store.
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
