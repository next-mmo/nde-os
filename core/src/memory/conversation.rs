use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

/// SQLite-backed conversation persistence.
/// Stores messages cross-channel with metadata.
pub struct ConversationStore {
    conn: Mutex<Connection>,
}

impl ConversationStore {
    pub fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        let conn =
            Connection::open(db_path.as_ref()).context("Failed to open conversation database")?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY,
                title TEXT,
                channel TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                conversation_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT,
                tool_calls TEXT,
                tool_call_id TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY (conversation_id) REFERENCES conversations(id)
            );
            CREATE INDEX IF NOT EXISTS idx_messages_conv ON messages(conversation_id);",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Create a new conversation and return its ID.
    pub fn create_conversation(&self, title: &str, channel: &str) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO conversations (id, title, channel, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, title, channel, now, now],
        )?;

        Ok(id)
    }

    /// Save a message to a conversation.
    pub fn save_message(
        &self,
        conversation_id: &str,
        role: &str,
        content: Option<&str>,
        tool_calls: Option<&str>,
        tool_call_id: Option<&str>,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO messages (conversation_id, role, content, tool_calls, tool_call_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![conversation_id, role, content, tool_calls, tool_call_id, now],
        )?;

        conn.execute(
            "UPDATE conversations SET updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, conversation_id],
        )?;

        Ok(())
    }

    /// List recent conversations.
    pub fn list_conversations(&self, limit: usize) -> Result<Vec<ConversationSummary>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, channel, created_at, updated_at
             FROM conversations ORDER BY updated_at DESC LIMIT ?1",
        )?;

        let rows = stmt.query_map(rusqlite::params![limit], |row| {
            Ok(ConversationSummary {
                id: row.get(0)?,
                title: row.get(1)?,
                channel: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;

        let mut convs = Vec::new();
        for row in rows {
            convs.push(row?);
        }
        Ok(convs)
    }

    /// Get all messages for a conversation.
    pub fn get_messages(&self, conversation_id: &str) -> Result<Vec<StoredMessage>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, role, content, tool_calls, tool_call_id, created_at
             FROM messages WHERE conversation_id = ?1 ORDER BY id ASC",
        )?;

        let rows = stmt.query_map(rusqlite::params![conversation_id], |row| {
            Ok(StoredMessage {
                id: row.get(0)?,
                role: row.get(1)?,
                content: row.get(2)?,
                tool_calls: row.get(3)?,
                tool_call_id: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        let mut msgs = Vec::new();
        for row in rows {
            msgs.push(row?);
        }
        Ok(msgs)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationSummary {
    pub id: String,
    pub title: String,
    pub channel: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StoredMessage {
    pub id: i64,
    pub role: String,
    pub content: Option<String>,
    pub tool_calls: Option<String>,
    pub tool_call_id: Option<String>,
    pub created_at: String,
}
