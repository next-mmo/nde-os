//! Semantic memory store with vector embedding support.

use crate::memory::types::{AgentId, MemoryFilter, MemoryFragment, MemoryId, MemorySource};
use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::debug;

#[derive(Clone)]
pub struct SemanticStore {
    conn: Arc<Mutex<Connection>>,
}

impl SemanticStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn remember(
        &self, agent_id: AgentId, content: &str, source: MemorySource,
        scope: &str, metadata: HashMap<String, serde_json::Value>,
    ) -> Result<MemoryId> {
        self.remember_with_embedding(agent_id, content, source, scope, metadata, None)
    }

    pub fn remember_with_embedding(
        &self, agent_id: AgentId, content: &str, source: MemorySource,
        scope: &str, metadata: HashMap<String, serde_json::Value>,
        embedding: Option<&[f32]>,
    ) -> Result<MemoryId> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let id = MemoryId::new();
        let now = Utc::now().to_rfc3339();
        let source_str = serde_json::to_string(&source)?;
        let meta_str = serde_json::to_string(&metadata)?;
        let emb_bytes: Option<Vec<u8>> = embedding.map(embedding_to_bytes);
        conn.execute(
            "INSERT INTO memories (id,agent_id,content,source,scope,confidence,metadata,created_at,accessed_at,access_count,deleted,embedding) VALUES (?1,?2,?3,?4,?5,1.0,?6,?7,?7,0,0,?8)",
            rusqlite::params![id.0.to_string(), agent_id.0.to_string(), content, source_str, scope, meta_str, now, emb_bytes],
        )?;
        Ok(id)
    }

    pub fn recall(&self, query: &str, limit: usize, filter: Option<MemoryFilter>) -> Result<Vec<MemoryFragment>> {
        self.recall_with_embedding(query, limit, filter, None)
    }

    pub fn recall_with_embedding(
        &self, query: &str, limit: usize, filter: Option<MemoryFilter>,
        query_embedding: Option<&[f32]>,
    ) -> Result<Vec<MemoryFragment>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let fetch_limit = if query_embedding.is_some() { (limit * 10).max(100) } else { limit };

        let mut sql = String::from("SELECT id,agent_id,content,source,scope,confidence,metadata,created_at,accessed_at,access_count,embedding FROM memories WHERE deleted=0");
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut pi = 1;

        if query_embedding.is_none() && !query.is_empty() {
            sql.push_str(&format!(" AND content LIKE ?{pi}"));
            params.push(Box::new(format!("%{query}%")));
            pi += 1;
        }
        if let Some(ref f) = filter {
            if let Some(aid) = f.agent_id {
                sql.push_str(&format!(" AND agent_id=?{pi}"));
                params.push(Box::new(aid.0.to_string()));
                pi += 1;
            }
            if let Some(ref scope) = f.scope {
                sql.push_str(&format!(" AND scope=?{pi}"));
                params.push(Box::new(scope.clone()));
                pi += 1;
            }
            if let Some(mc) = f.min_confidence {
                sql.push_str(&format!(" AND confidence>=?{pi}"));
                params.push(Box::new(mc as f64));
                pi += 1;
            }
            if let Some(ref src) = f.source {
                sql.push_str(&format!(" AND source=?{pi}"));
                params.push(Box::new(serde_json::to_string(src)?));
                let _ = pi;
            }
        }
        sql.push_str(&format!(" ORDER BY accessed_at DESC,access_count DESC LIMIT {fetch_limit}"));

        let mut stmt = conn.prepare(&sql)?;
        let prefs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(prefs.as_slice(), |row| {
            Ok((row.get::<_,String>(0)?, row.get::<_,String>(1)?, row.get::<_,String>(2)?,
                row.get::<_,String>(3)?, row.get::<_,String>(4)?, row.get::<_,f64>(5)?,
                row.get::<_,String>(6)?, row.get::<_,String>(7)?, row.get::<_,String>(8)?,
                row.get::<_,i64>(9)?, row.get::<_,Option<Vec<u8>>>(10)?))
        })?;

        let mut fragments = Vec::new();
        for r in rows {
            let (id_s,ag_s,content,src_s,scope,conf,meta_s,cr_s,ac_s,ac_cnt,emb_b) = r?;
            let id = uuid::Uuid::parse_str(&id_s).map(MemoryId)?;
            let agent_id = uuid::Uuid::parse_str(&ag_s).map(AgentId)?;
            let source: MemorySource = serde_json::from_str(&src_s).unwrap_or(MemorySource::System);
            let metadata: HashMap<String, serde_json::Value> = serde_json::from_str(&meta_s).unwrap_or_default();
            let created_at = chrono::DateTime::parse_from_rfc3339(&cr_s).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now());
            let accessed_at = chrono::DateTime::parse_from_rfc3339(&ac_s).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now());
            fragments.push(MemoryFragment {
                id, agent_id, content, embedding: emb_b.as_deref().map(embedding_from_bytes),
                metadata, source, confidence: conf as f32, created_at, accessed_at,
                access_count: ac_cnt as u64, scope,
            });
        }

        if let Some(qe) = query_embedding {
            fragments.sort_by(|a, b| {
                let sa = a.embedding.as_deref().map(|e| cosine_similarity(qe, e)).unwrap_or(-1.0);
                let sb = b.embedding.as_deref().map(|e| cosine_similarity(qe, e)).unwrap_or(-1.0);
                sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
            });
            fragments.truncate(limit);
            debug!("Vector recall: {} results from {} candidates", fragments.len(), fetch_limit);
        }

        for frag in &fragments {
            let _ = conn.execute(
                "UPDATE memories SET access_count=access_count+1,accessed_at=?1 WHERE id=?2",
                rusqlite::params![Utc::now().to_rfc3339(), frag.id.0.to_string()],
            );
        }
        Ok(fragments)
    }

    pub fn forget(&self, id: MemoryId) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        conn.execute("UPDATE memories SET deleted=1 WHERE id=?1", rusqlite::params![id.0.to_string()])?;
        Ok(())
    }

    pub fn update_embedding(&self, id: MemoryId, embedding: &[f32]) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        conn.execute("UPDATE memories SET embedding=?1 WHERE id=?2", rusqlite::params![embedding_to_bytes(embedding), id.0.to_string()])?;
        Ok(())
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() { return 0.0; }
    let (mut dot, mut na, mut nb) = (0.0f32, 0.0f32, 0.0f32);
    for i in 0..a.len() { dot += a[i]*b[i]; na += a[i]*a[i]; nb += b[i]*b[i]; }
    let d = na.sqrt() * nb.sqrt();
    if d < f32::EPSILON { 0.0 } else { dot / d }
}

fn embedding_to_bytes(e: &[f32]) -> Vec<u8> {
    let mut b = Vec::with_capacity(e.len()*4);
    for &v in e { b.extend_from_slice(&v.to_le_bytes()); }
    b
}

fn embedding_from_bytes(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4).map(|c| f32::from_le_bytes([c[0],c[1],c[2],c[3]])).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::migration::run_migrations;

    fn setup() -> SemanticStore {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        SemanticStore::new(Arc::new(Mutex::new(conn)))
    }

    #[test]
    fn test_remember_and_recall() {
        let store = setup();
        let aid = AgentId::new();
        store.remember(aid, "The user likes Rust", MemorySource::Conversation, "episodic", HashMap::new()).unwrap();
        let r = store.recall("Rust", 10, None).unwrap();
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn test_forget() {
        let store = setup();
        let aid = AgentId::new();
        let id = store.remember(aid, "To forget", MemorySource::Conversation, "episodic", HashMap::new()).unwrap();
        store.forget(id).unwrap();
        assert!(store.recall("To forget", 10, None).unwrap().is_empty());
    }

    #[test]
    fn test_vector_ranking() {
        let store = setup();
        let aid = AgentId::new();
        store.remember_with_embedding(aid, "Rust systems", MemorySource::Conversation, "episodic", HashMap::new(), Some(&[0.9,0.1,0.0,0.0])).unwrap();
        store.remember_with_embedding(aid, "Python interp", MemorySource::Conversation, "episodic", HashMap::new(), Some(&[0.0,0.0,0.9,0.1])).unwrap();
        let r = store.recall_with_embedding("", 3, None, Some(&[0.85,0.15,0.0,0.0])).unwrap();
        assert!(r[0].content.contains("Rust"));
    }
}
