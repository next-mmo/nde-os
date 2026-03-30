use crate::llm::ToolDef;
use crate::memory::KvStore;
use crate::sandbox::Sandbox;
use crate::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;

/// Stores a key-value pair in persistent memory.
/// Uses workspace-scoped SQLite — data stays in the sandbox.
pub struct MemoryStoreTool;

#[async_trait]
impl Tool for MemoryStoreTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "memory_store".into(),
            description: "Store a key-value pair in persistent memory. Data persists across conversations. Use for remembering facts, preferences, and state.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "key": {
                        "type": "string",
                        "description": "The key to store under (e.g. 'user.name', 'project.language')"
                    },
                    "value": {
                        "type": "string",
                        "description": "The value to store"
                    }
                },
                "required": ["key", "value"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, sandbox: &Sandbox) -> Result<String> {
        let key = args
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'key' argument"))?;

        let value = args
            .get("value")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'value' argument"))?;

        let db_path = sandbox.root().join("data").join("memory.db");
        let store = KvStore::new(&db_path)?;
        store.set(key, value)?;

        Ok(format!("Stored: {} = {}", key, value))
    }
}

/// Recalls a value from persistent memory by key, or lists keys by prefix.
pub struct MemoryRecallTool;

#[async_trait]
impl Tool for MemoryRecallTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "memory_recall".into(),
            description: "Recall a value from persistent memory by key, or list all keys with a prefix. Returns the stored value or a list of matching keys.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "key": {
                        "type": "string",
                        "description": "Exact key to recall, or a prefix to list all matching keys"
                    },
                    "list_prefix": {
                        "type": "boolean",
                        "description": "If true, treat 'key' as a prefix and list all matching keys (default: false)",
                        "default": false
                    },
                    "delete": {
                        "type": "boolean",
                        "description": "If true, delete the key after returning its value (default: false)",
                        "default": false
                    }
                },
                "required": ["key"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, sandbox: &Sandbox) -> Result<String> {
        let key = args
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'key' argument"))?;

        let list_prefix = args
            .get("list_prefix")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let delete = args
            .get("delete")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let db_path = sandbox.root().join("data").join("memory.db");
        let store = KvStore::new(&db_path)?;

        if list_prefix {
            let keys = store.list_keys(key)?;
            if keys.is_empty() {
                Ok(format!("No keys found with prefix '{}'", key))
            } else {
                Ok(format!("Keys matching '{}':\n{}", key, keys.join("\n")))
            }
        } else {
            match store.get(key)? {
                Some(value) => {
                    if delete {
                        store.delete(key)?;
                        Ok(format!("{} (deleted)", value))
                    } else {
                        Ok(value)
                    }
                }
                None => Ok(format!("No value stored for key '{}'", key)),
            }
        }
    }
}
