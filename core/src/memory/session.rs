//! Session management — canonical cross-channel sessions and JSONL mirroring.

use crate::memory::types::*;
use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};

const DEFAULT_CANONICAL_WINDOW: usize = 50;
const DEFAULT_COMPACTION_THRESHOLD: usize = 100;

/// A conversation session with message history.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub agent_id: AgentId,
    pub messages: Vec<Message>,
    pub context_window_tokens: u64,
    pub label: Option<String>,
}

/// A canonical session for cross-channel persistent context.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CanonicalSession {
    pub agent_id: AgentId,
    pub messages: Vec<Message>,
    pub compaction_cursor: usize,
    pub compacted_summary: Option<String>,
    pub updated_at: String,
}

#[derive(Clone)]
pub struct SessionStore {
    conn: Arc<Mutex<Connection>>,
}

impl SessionStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self { Self { conn } }

    pub fn get_session(&self, session_id: SessionId) -> Result<Option<Session>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let mut stmt = conn.prepare("SELECT agent_id,messages,context_window_tokens,label FROM sessions WHERE id=?1")?;
        let result = stmt.query_row(rusqlite::params![session_id.0.to_string()], |row| {
            Ok((row.get::<_,String>(0)?, row.get::<_,Vec<u8>>(1)?, row.get::<_,i64>(2)?, row.get::<_,Option<String>>(3).unwrap_or(None)))
        });
        match result {
            Ok((ag, blob, tokens, label)) => {
                let agent_id = uuid::Uuid::parse_str(&ag).map(AgentId)?;
                let messages: Vec<Message> = rmp_serde::from_slice(&blob)?;
                Ok(Some(Session { id: session_id, agent_id, messages, context_window_tokens: tokens as u64, label }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn save_session(&self, session: &Session) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let blob = rmp_serde::to_vec_named(&session.messages)?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO sessions (id,agent_id,messages,context_window_tokens,label,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?6) ON CONFLICT(id) DO UPDATE SET messages=?3,context_window_tokens=?4,label=?5,updated_at=?6",
            rusqlite::params![session.id.0.to_string(), session.agent_id.0.to_string(), blob, session.context_window_tokens as i64, session.label.as_deref(), now],
        )?;
        Ok(())
    }

    pub fn list_sessions(&self, agent_id: AgentId) -> Result<Vec<Session>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let mut stmt = conn.prepare("SELECT id,messages,context_window_tokens,label FROM sessions WHERE agent_id=?1 ORDER BY created_at DESC")?;
        let rows = stmt.query_map(rusqlite::params![agent_id.0.to_string()], |row| {
            Ok((row.get::<_,String>(0)?, row.get::<_,Vec<u8>>(1)?, row.get::<_,i64>(2)?, row.get::<_,Option<String>>(3).unwrap_or(None)))
        })?;
        
        let mut sessions = Vec::new();
        for result in rows {
            let (id_str, blob, tokens, label) = result?;
            let id = SessionId(uuid::Uuid::parse_str(&id_str).unwrap_or_default());
            let messages: Vec<Message> = rmp_serde::from_slice(&blob).unwrap_or_default();
            sessions.push(Session { id, agent_id, messages, context_window_tokens: tokens as u64, label });
        }
        Ok(sessions)
    }

    pub fn create_session(&self, agent_id: AgentId) -> Result<Session> {
        let session = Session { id: SessionId::new(), agent_id, messages: Vec::new(), context_window_tokens: 0, label: None };
        self.save_session(&session)?;
        Ok(session)
    }

    pub fn delete_session(&self, session_id: SessionId) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        conn.execute("DELETE FROM sessions WHERE id=?1", rusqlite::params![session_id.0.to_string()])?;
        Ok(())
    }

    pub fn delete_agent_sessions(&self, agent_id: AgentId) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        conn.execute("DELETE FROM sessions WHERE agent_id=?1", rusqlite::params![agent_id.0.to_string()])?;
        Ok(())
    }

    // -- Canonical (cross-channel) session methods --

    pub fn load_canonical(&self, agent_id: AgentId) -> Result<CanonicalSession> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let mut stmt = conn.prepare("SELECT messages,compaction_cursor,compacted_summary,updated_at FROM canonical_sessions WHERE agent_id=?1")?;
        let result = stmt.query_row(rusqlite::params![agent_id.0.to_string()], |row| {
            Ok((row.get::<_,Vec<u8>>(0)?, row.get::<_,i64>(1)?, row.get::<_,Option<String>>(2)?, row.get::<_,String>(3)?))
        });
        match result {
            Ok((blob, cursor, summary, updated_at)) => {
                let messages: Vec<Message> = rmp_serde::from_slice(&blob)?;
                Ok(CanonicalSession { agent_id, messages, compaction_cursor: cursor as usize, compacted_summary: summary, updated_at })
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                Ok(CanonicalSession { agent_id, messages: Vec::new(), compaction_cursor: 0, compacted_summary: None, updated_at: Utc::now().to_rfc3339() })
            }
            Err(e) => Err(e.into()),
        }
    }

    pub fn append_canonical(&self, agent_id: AgentId, new_messages: &[Message], compaction_threshold: Option<usize>) -> Result<CanonicalSession> {
        let mut canonical = self.load_canonical(agent_id)?;
        canonical.messages.extend(new_messages.iter().cloned());
        let threshold = compaction_threshold.unwrap_or(DEFAULT_COMPACTION_THRESHOLD);

        if canonical.messages.len() > threshold {
            let keep = DEFAULT_CANONICAL_WINDOW;
            let to_compact = canonical.messages.len().saturating_sub(keep);
            if to_compact > canonical.compaction_cursor {
                let compacting = &canonical.messages[canonical.compaction_cursor..to_compact];
                let mut parts: Vec<String> = Vec::new();
                if let Some(ref existing) = canonical.compacted_summary { parts.push(existing.clone()); }
                for msg in compacting {
                    let role = match msg.role { Role::User => "User", Role::Assistant => "Assistant", Role::System => "System" };
                    let text = &msg.content;
                    if !text.is_empty() {
                        let t = if text.len() > 200 { format!("{}...", truncate_str(text, 200)) } else { text.clone() };
                        parts.push(format!("{role}: {t}"));
                    }
                }
                let mut summary = parts.join("\n");
                if summary.len() > 4000 {
                    let start = summary.len() - 4000;
                    let safe = (start..summary.len()).find(|&i| summary.is_char_boundary(i)).unwrap_or(summary.len());
                    summary = summary[safe..].to_string();
                }
                canonical.compacted_summary = Some(summary);
                canonical.messages = canonical.messages.split_off(to_compact);
                canonical.compaction_cursor = 0;
            }
        }
        canonical.updated_at = Utc::now().to_rfc3339();
        self.save_canonical(&canonical)?;
        Ok(canonical)
    }

    pub fn canonical_context(&self, agent_id: AgentId, window_size: Option<usize>) -> Result<(Option<String>, Vec<Message>)> {
        let canonical = self.load_canonical(agent_id)?;
        let window = window_size.unwrap_or(DEFAULT_CANONICAL_WINDOW);
        let start = canonical.messages.len().saturating_sub(window);
        Ok((canonical.compacted_summary.clone(), canonical.messages[start..].to_vec()))
    }

    /// Store an LLM-generated summary, replacing older messages.
    pub fn store_llm_summary(&self, agent_id: AgentId, summary: &str, kept_messages: Vec<Message>) -> Result<()> {
        let mut canonical = self.load_canonical(agent_id)?;
        canonical.compacted_summary = Some(summary.to_string());
        canonical.messages = kept_messages;
        canonical.compaction_cursor = 0;
        canonical.updated_at = Utc::now().to_rfc3339();
        self.save_canonical(&canonical)
    }

    fn save_canonical(&self, canonical: &CanonicalSession) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let blob = rmp_serde::to_vec(&canonical.messages)?;
        conn.execute(
            "INSERT INTO canonical_sessions (agent_id,messages,compaction_cursor,compacted_summary,updated_at) VALUES (?1,?2,?3,?4,?5) ON CONFLICT(agent_id) DO UPDATE SET messages=?2,compaction_cursor=?3,compacted_summary=?4,updated_at=?5",
            rusqlite::params![canonical.agent_id.0.to_string(), blob, canonical.compaction_cursor as i64, canonical.compacted_summary, canonical.updated_at],
        )?;
        Ok(())
    }

    /// Write a human-readable JSONL mirror of a session to disk.
    pub fn write_jsonl_mirror(&self, session: &Session, sessions_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(sessions_dir)?;
        let path = sessions_dir.join(format!("{}.jsonl", session.id.0));
        let mut file = std::fs::File::create(&path)?;
        let now = Utc::now().to_rfc3339();
        for msg in &session.messages {
            let role = match msg.role { Role::User => "user", Role::Assistant => "assistant", Role::System => "system" };
            let line = serde_json::json!({ "timestamp": now, "role": role, "content": msg.content });
            serde_json::to_writer(&mut file, &line)?;
            file.write_all(b"\n")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::migration::run_migrations;

    fn setup() -> SessionStore {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        SessionStore::new(Arc::new(Mutex::new(conn)))
    }

    #[test]
    fn test_create_and_load() {
        let store = setup();
        let aid = AgentId::new();
        let s = store.create_session(aid).unwrap();
        let loaded = store.get_session(s.id).unwrap().unwrap();
        assert_eq!(loaded.agent_id, aid);
    }

    #[test]
    fn test_canonical_cross_channel() {
        let store = setup();
        let aid = AgentId::new();
        store.append_canonical(aid, &[Message::user("Hello from Telegram"), Message::assistant("Hi!")], None).unwrap();
        let (_, recent) = store.canonical_context(aid, None).unwrap();
        assert_eq!(recent.len(), 2);
        assert!(recent[0].content.contains("Telegram"));
    }

    #[test]
    fn test_canonical_compaction() {
        let store = setup();
        let aid = AgentId::new();
        let msgs: Vec<Message> = (0..120).map(|i| Message::user(format!("Message {i}"))).collect();
        let c = store.append_canonical(aid, &msgs, Some(100)).unwrap();
        assert!(c.messages.len() <= 60);
        assert!(c.compacted_summary.is_some());
    }
}
