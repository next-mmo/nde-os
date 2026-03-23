use crate::knowledge::{Entity, KnowledgeGraph, Relation};
use crate::llm::ToolDef;
use crate::sandbox::Sandbox;
use crate::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;

/// Stores entities and relations in the knowledge graph.
pub struct KnowledgeStoreTool;

#[async_trait]
impl Tool for KnowledgeStoreTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "knowledge_store".into(),
            description: "Store an entity or relation in the knowledge graph. Entities have an id, type, name, and metadata. Relations link entities together.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["entity", "relation"],
                        "description": "Whether to store an entity or a relation"
                    },
                    "id": {
                        "type": "string",
                        "description": "Entity ID (for action=entity)"
                    },
                    "entity_type": {
                        "type": "string",
                        "description": "Entity type (e.g. 'app', 'concept', 'person', 'tool')"
                    },
                    "name": {
                        "type": "string",
                        "description": "Entity name"
                    },
                    "metadata": {
                        "type": "object",
                        "description": "Additional metadata as JSON object"
                    },
                    "source_id": {
                        "type": "string",
                        "description": "Source entity ID (for action=relation)"
                    },
                    "target_id": {
                        "type": "string",
                        "description": "Target entity ID (for action=relation)"
                    },
                    "relation_type": {
                        "type": "string",
                        "description": "Relation type (e.g. 'depends_on', 'created_by', 'part_of')"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, sandbox: &Sandbox) -> Result<String> {
        let action = args.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'action' argument"))?;

        let db_path = sandbox.root().join("data").join("knowledge.db");
        let kg = KnowledgeGraph::new(&db_path)?;

        match action {
            "entity" => {
                let id = args.get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'id' for entity"))?;
                let entity_type = args.get("entity_type")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'entity_type' for entity"))?;
                let name = args.get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'name' for entity"))?;
                let metadata = args.get("metadata")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));

                kg.upsert_entity(&Entity {
                    id: id.into(),
                    entity_type: entity_type.into(),
                    name: name.into(),
                    metadata,
                })?;

                Ok(format!("Stored entity: {} ({}: {})", id, entity_type, name))
            }
            "relation" => {
                let source_id = args.get("source_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'source_id' for relation"))?;
                let target_id = args.get("target_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'target_id' for relation"))?;
                let relation_type = args.get("relation_type")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'relation_type' for relation"))?;
                let metadata = args.get("metadata")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));

                kg.add_relation(&Relation {
                    source_id: source_id.into(),
                    target_id: target_id.into(),
                    relation_type: relation_type.into(),
                    metadata,
                })?;

                Ok(format!("Stored relation: {} --[{}]--> {}", source_id, relation_type, target_id))
            }
            other => Err(anyhow::anyhow!("Unknown action: '{}'. Use 'entity' or 'relation'", other)),
        }
    }
}

/// Queries the knowledge graph for entities and relations.
pub struct KnowledgeQueryTool;

#[async_trait]
impl Tool for KnowledgeQueryTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "knowledge_query".into(),
            description: "Query the knowledge graph. Search entities by name, find by type, or get relations for an entity.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["search", "by_type", "relations"],
                        "description": "Query type: search by name, find by entity type, or get relations"
                    },
                    "query": {
                        "type": "string",
                        "description": "Search query (for action=search)"
                    },
                    "entity_type": {
                        "type": "string",
                        "description": "Entity type to filter by (for action=by_type)"
                    },
                    "entity_id": {
                        "type": "string",
                        "description": "Entity ID to get relations for (for action=relations)"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, sandbox: &Sandbox) -> Result<String> {
        let action = args.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'action' argument"))?;

        let db_path = sandbox.root().join("data").join("knowledge.db");
        let kg = KnowledgeGraph::new(&db_path)?;

        match action {
            "search" => {
                let query = args.get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'query' for search"))?;

                let entities = kg.search(query)?;
                if entities.is_empty() {
                    Ok(format!("No entities found matching '{}'", query))
                } else {
                    let mut output = format!("Found {} entities:\n\n", entities.len());
                    for e in &entities {
                        output.push_str(&format!(
                            "  [{}] {} (type: {}) meta: {}\n",
                            e.id, e.name, e.entity_type,
                            serde_json::to_string(&e.metadata).unwrap_or_default()
                        ));
                    }
                    Ok(output)
                }
            }
            "by_type" => {
                let entity_type = args.get("entity_type")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'entity_type' for by_type"))?;

                let entities = kg.find_by_type(entity_type)?;
                if entities.is_empty() {
                    Ok(format!("No entities of type '{}'", entity_type))
                } else {
                    let mut output = format!("{} entities of type '{}':\n\n", entities.len(), entity_type);
                    for e in &entities {
                        output.push_str(&format!("  [{}] {}\n", e.id, e.name));
                    }
                    Ok(output)
                }
            }
            "relations" => {
                let entity_id = args.get("entity_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'entity_id' for relations"))?;

                let rels = kg.get_relations(entity_id)?;
                if rels.is_empty() {
                    Ok(format!("No relations found for entity '{}'", entity_id))
                } else {
                    let mut output = format!("Relations for '{}':\n\n", entity_id);
                    for r in &rels {
                        output.push_str(&format!(
                            "  {} --[{}]--> {}\n",
                            r.source_id, r.relation_type, r.target_id
                        ));
                    }
                    Ok(output)
                }
            }
            other => Err(anyhow::anyhow!("Unknown action: '{}'. Use 'search', 'by_type', or 'relations'", other)),
        }
    }
}
