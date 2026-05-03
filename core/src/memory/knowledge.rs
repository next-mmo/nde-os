//! Knowledge graph backed by SQLite.

use crate::memory::types::*;
use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Clone)]
pub struct KnowledgeStore {
    conn: Arc<Mutex<Connection>>,
}

impl KnowledgeStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self { Self { conn } }

    pub fn add_entity(&self, entity: Entity) -> Result<String> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let id = if entity.id.is_empty() { Uuid::new_v4().to_string() } else { entity.id.clone() };
        let etype = serde_json::to_string(&entity.entity_type)?;
        let props = serde_json::to_string(&entity.properties)?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO entities (id,entity_type,name,properties,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?5) ON CONFLICT(id) DO UPDATE SET name=?3,properties=?4,updated_at=?5",
            rusqlite::params![id, etype, entity.name, props, now],
        )?;
        Ok(id)
    }

    pub fn add_relation(&self, relation: Relation) -> Result<String> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let id = Uuid::new_v4().to_string();
        let rtype = serde_json::to_string(&relation.relation)?;
        let props = serde_json::to_string(&relation.properties)?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO relations (id,source_entity,relation_type,target_entity,properties,confidence,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            rusqlite::params![id, relation.source, rtype, relation.target, props, relation.confidence as f64, now],
        )?;
        Ok(id)
    }

    pub fn query_graph(&self, pattern: GraphPattern) -> Result<Vec<GraphMatch>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let mut sql = String::from(
            "SELECT s.id,s.entity_type,s.name,s.properties,s.created_at,s.updated_at,r.id,r.source_entity,r.relation_type,r.target_entity,r.properties,r.confidence,r.created_at,t.id,t.entity_type,t.name,t.properties,t.created_at,t.updated_at FROM relations r JOIN entities s ON r.source_entity=s.id JOIN entities t ON r.target_entity=t.id WHERE 1=1"
        );
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut idx = 1;

        if let Some(ref source) = pattern.source {
            sql.push_str(&format!(" AND (s.id=?{} OR s.name=?{})", idx, idx+1));
            params.push(Box::new(source.clone()));
            params.push(Box::new(source.clone()));
            idx += 2;
        }
        if let Some(ref rel) = pattern.relation {
            sql.push_str(&format!(" AND r.relation_type=?{idx}"));
            params.push(Box::new(serde_json::to_string(rel)?));
            idx += 1;
        }
        if let Some(ref target) = pattern.target {
            sql.push_str(&format!(" AND (t.id=?{} OR t.name=?{})", idx, idx+1));
            params.push(Box::new(target.clone()));
            params.push(Box::new(target.clone()));
            idx += 2;
        }
        let _ = idx;
        sql.push_str(" LIMIT 100");

        let mut stmt = conn.prepare(&sql)?;
        let prefs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(prefs.as_slice(), |row| {
            Ok((
                row.get::<_,String>(0)?, row.get::<_,String>(1)?, row.get::<_,String>(2)?,
                row.get::<_,String>(3)?, row.get::<_,String>(4)?, row.get::<_,String>(5)?,
                row.get::<_,String>(7)?, row.get::<_,String>(8)?,
                row.get::<_,String>(9)?, row.get::<_,String>(10)?, row.get::<_,f64>(11)?,
                row.get::<_,String>(12)?,
                row.get::<_,String>(13)?, row.get::<_,String>(14)?, row.get::<_,String>(15)?,
                row.get::<_,String>(16)?, row.get::<_,String>(17)?, row.get::<_,String>(18)?,
            ))
        })?;

        let mut matches = Vec::new();
        for r in rows {
            let (si,st,sn,sp,sc,su, rs,rt,rg,rp,rcf,rc, ti,tt,tn,tp,tc,tu) = r?;
            matches.push(GraphMatch {
                source: parse_entity(&si,&st,&sn,&sp,&sc,&su),
                relation: parse_relation(&rs,&rt,&rg,&rp,rcf,&rc),
                target: parse_entity(&ti,&tt,&tn,&tp,&tc,&tu),
            });
        }
        Ok(matches)
    }
}

fn parse_entity(id: &str, etype: &str, name: &str, props: &str, created: &str, updated: &str) -> Entity {
    Entity {
        id: id.to_string(),
        entity_type: serde_json::from_str(etype).unwrap_or(EntityType::Custom("unknown".into())),
        name: name.to_string(),
        properties: serde_json::from_str(props).unwrap_or_default(),
        created_at: chrono::DateTime::parse_from_rfc3339(created).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
        updated_at: chrono::DateTime::parse_from_rfc3339(updated).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
    }
}

fn parse_relation(source: &str, rtype: &str, target: &str, props: &str, conf: f64, created: &str) -> Relation {
    Relation {
        source: source.to_string(),
        relation: serde_json::from_str(rtype).unwrap_or(RelationType::RelatedTo),
        target: target.to_string(),
        properties: serde_json::from_str(props).unwrap_or_default(),
        confidence: conf as f32,
        created_at: chrono::DateTime::parse_from_rfc3339(created).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::migration::run_migrations;

    fn setup() -> KnowledgeStore {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        KnowledgeStore::new(Arc::new(Mutex::new(conn)))
    }

    #[test]
    fn test_add_entity() {
        let store = setup();
        let id = store.add_entity(Entity {
            id: String::new(), entity_type: EntityType::Person, name: "Alice".into(),
            properties: HashMap::new(), created_at: Utc::now(), updated_at: Utc::now(),
        }).unwrap();
        assert!(!id.is_empty());
    }

    #[test]
    fn test_add_relation_and_query() {
        let store = setup();
        store.add_entity(Entity { id: "alice".into(), entity_type: EntityType::Person, name: "Alice".into(), properties: HashMap::new(), created_at: Utc::now(), updated_at: Utc::now() }).unwrap();
        store.add_entity(Entity { id: "acme".into(), entity_type: EntityType::Organization, name: "Acme Corp".into(), properties: HashMap::new(), created_at: Utc::now(), updated_at: Utc::now() }).unwrap();
        store.add_relation(Relation { source: "alice".into(), relation: RelationType::WorksAt, target: "acme".into(), properties: HashMap::new(), confidence: 0.95, created_at: Utc::now() }).unwrap();
        let m = store.query_graph(GraphPattern { source: Some("alice".into()), relation: Some(RelationType::WorksAt), target: None, max_depth: 1 }).unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].target.name, "Acme Corp");
    }
}
