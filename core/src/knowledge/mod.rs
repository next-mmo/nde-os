use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

/// SQLite-backed knowledge graph for entity-relation storage.
/// Agents can store and query structured knowledge across sessions.
pub struct KnowledgeGraph {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Entity {
    pub id: String,
    pub entity_type: String,
    pub name: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Relation {
    pub source_id: String,
    pub target_id: String,
    pub relation_type: String,
    pub metadata: serde_json::Value,
}

impl KnowledgeGraph {
    pub fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(db_path.as_ref())
            .context("Failed to open knowledge database")?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS entities (
                id TEXT PRIMARY KEY,
                entity_type TEXT NOT NULL,
                name TEXT NOT NULL,
                metadata TEXT DEFAULT '{}',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS relations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                relation_type TEXT NOT NULL,
                metadata TEXT DEFAULT '{}',
                created_at TEXT NOT NULL,
                FOREIGN KEY (source_id) REFERENCES entities(id),
                FOREIGN KEY (target_id) REFERENCES entities(id)
            );
            CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type);
            CREATE INDEX IF NOT EXISTS idx_relations_source ON relations(source_id);
            CREATE INDEX IF NOT EXISTS idx_relations_target ON relations(target_id);"
        )?;

        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Add or update an entity.
    pub fn upsert_entity(&self, entity: &Entity) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let meta = serde_json::to_string(&entity.metadata)?;
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO entities (id, entity_type, name, metadata, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)
             ON CONFLICT(id) DO UPDATE SET name = ?3, metadata = ?4, updated_at = ?5",
            rusqlite::params![entity.id, entity.entity_type, entity.name, meta, now],
        )?;
        Ok(())
    }

    /// Add a relation between entities.
    pub fn add_relation(&self, relation: &Relation) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let meta = serde_json::to_string(&relation.metadata)?;
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO relations (source_id, target_id, relation_type, metadata, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![relation.source_id, relation.target_id, relation.relation_type, meta, now],
        )?;
        Ok(())
    }

    /// Find entities by type.
    pub fn find_by_type(&self, entity_type: &str) -> Result<Vec<Entity>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, entity_type, name, metadata FROM entities WHERE entity_type = ?1"
        )?;

        let rows = stmt.query_map(rusqlite::params![entity_type], |row| {
            let meta_str: String = row.get(3)?;
            Ok(Entity {
                id: row.get(0)?,
                entity_type: row.get(1)?,
                name: row.get(2)?,
                metadata: serde_json::from_str(&meta_str).unwrap_or_default(),
            })
        })?;

        let mut entities = Vec::new();
        for row in rows {
            entities.push(row?);
        }
        Ok(entities)
    }

    /// Get relations for an entity.
    pub fn get_relations(&self, entity_id: &str) -> Result<Vec<Relation>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT source_id, target_id, relation_type, metadata
             FROM relations WHERE source_id = ?1 OR target_id = ?1"
        )?;

        let rows = stmt.query_map(rusqlite::params![entity_id], |row| {
            let meta_str: String = row.get(3)?;
            Ok(Relation {
                source_id: row.get(0)?,
                target_id: row.get(1)?,
                relation_type: row.get(2)?,
                metadata: serde_json::from_str(&meta_str).unwrap_or_default(),
            })
        })?;

        let mut rels = Vec::new();
        for row in rows {
            rels.push(row?);
        }
        Ok(rels)
    }

    /// Search entities by name (case-insensitive).
    pub fn search(&self, query: &str) -> Result<Vec<Entity>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, entity_type, name, metadata FROM entities WHERE name LIKE ?1"
        )?;

        let pattern = format!("%{}%", query);
        let rows = stmt.query_map(rusqlite::params![pattern], |row| {
            let meta_str: String = row.get(3)?;
            Ok(Entity {
                id: row.get(0)?,
                entity_type: row.get(1)?,
                name: row.get(2)?,
                metadata: serde_json::from_str(&meta_str).unwrap_or_default(),
            })
        })?;

        let mut entities = Vec::new();
        for row in rows {
            entities.push(row?);
        }
        Ok(entities)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_graph() {
        let dir = tempfile::tempdir().unwrap();
        let kg = KnowledgeGraph::new(dir.path().join("knowledge.db")).unwrap();

        let entity = Entity {
            id: "app-1".into(),
            entity_type: "app".into(),
            name: "Stable Diffusion".into(),
            metadata: serde_json::json!({"version": "1.5"}),
        };
        kg.upsert_entity(&entity).unwrap();

        let found = kg.find_by_type("app").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "Stable Diffusion");

        let searched = kg.search("Stable").unwrap();
        assert_eq!(searched.len(), 1);
    }
}
