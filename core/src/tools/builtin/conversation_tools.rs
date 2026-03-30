use crate::llm::ToolDef;
use crate::memory::ConversationStore;
use crate::sandbox::Sandbox;
use crate::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;

/// Saves a conversation snapshot to persistent storage.
pub struct ConversationSaveTool;

#[async_trait]
impl Tool for ConversationSaveTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "conversation_save".into(),
            description: "Save a conversation message to persistent storage. Use to build a retrievable conversation history.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "conversation_id": {
                        "type": "string",
                        "description": "Conversation ID (omit to create a new conversation)"
                    },
                    "title": {
                        "type": "string",
                        "description": "Title for a new conversation (required when creating)",
                        "default": "Untitled"
                    },
                    "role": {
                        "type": "string",
                        "enum": ["user", "assistant", "system"],
                        "description": "Message role"
                    },
                    "content": {
                        "type": "string",
                        "description": "Message content"
                    }
                },
                "required": ["role", "content"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, sandbox: &Sandbox) -> Result<String> {
        let role = args
            .get("role")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'role' argument"))?;

        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'content' argument"))?;

        let db_path = sandbox.root().join("data").join("conversations.db");
        let store = ConversationStore::new(&db_path)?;

        let conv_id = if let Some(id) = args.get("conversation_id").and_then(|v| v.as_str()) {
            id.to_string()
        } else {
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled");
            store.create_conversation(title, "agent")?
        };

        store.save_message(&conv_id, role, Some(content), None, None)?;

        Ok(format!(
            "Saved {} message to conversation {}",
            role, conv_id
        ))
    }
}

/// Searches/retrieves past conversations.
pub struct ConversationSearchTool;

#[async_trait]
impl Tool for ConversationSearchTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "conversation_search".into(),
            description: "Search or list past conversations. Returns conversation summaries or messages from a specific conversation.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "conversation_id": {
                        "type": "string",
                        "description": "Get full messages for a specific conversation ID"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Number of recent conversations to list (default: 10)",
                        "default": 10
                    }
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, sandbox: &Sandbox) -> Result<String> {
        let db_path = sandbox.root().join("data").join("conversations.db");
        let store = ConversationStore::new(&db_path)?;

        if let Some(conv_id) = args.get("conversation_id").and_then(|v| v.as_str()) {
            let messages = store.get_messages(conv_id)?;
            if messages.is_empty() {
                return Ok(format!("No messages found for conversation {}", conv_id));
            }

            let mut output = format!("Conversation {}:\n\n", conv_id);
            for msg in &messages {
                let content = msg.content.as_deref().unwrap_or("[tool call]");
                // Truncate very long messages in display
                let display = if content.len() > 500 {
                    format!("{}...", &content[..500])
                } else {
                    content.to_string()
                };
                output.push_str(&format!("[{}] {}: {}\n", msg.created_at, msg.role, display));
            }
            Ok(output)
        } else {
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

            let convs = store.list_conversations(limit)?;
            if convs.is_empty() {
                return Ok("No conversations found.".into());
            }

            let mut output = format!("Recent conversations ({}):\n\n", convs.len());
            for conv in &convs {
                output.push_str(&format!(
                    "  {} — {} (channel: {}, updated: {})\n",
                    conv.id, conv.title, conv.channel, conv.updated_at
                ));
            }
            Ok(output)
        }
    }
}
