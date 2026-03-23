use ai_launcher_core::agent;
use ai_launcher_core::agent::config::AgentConfig;
use ai_launcher_core::memory::{ConversationStore, MemoryManager};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

use crate::response::*;
use serde_json::json;
use std::io::Cursor;
use tiny_http::{Request, Response};

/// Shared agent state held by the server.
pub struct AgentState {
    pub memory: MemoryManager,
    pub config: AgentConfig,
    runtime: tokio::runtime::Runtime,
}

impl AgentState {
    pub fn new(data_dir: &Path) -> anyhow::Result<Self> {
        let db_path = data_dir.join("agent.db");
        let memory = MemoryManager::new(&db_path)?;

        // Load agent config from file or defaults
        let config_path = data_dir.join("agent.toml");
        let config = if config_path.exists() {
            AgentConfig::from_file(&config_path).unwrap_or_default()
        } else {
            AgentConfig::default()
        };

        let runtime = tokio::runtime::Runtime::new()?;

        Ok(Self { memory, config, runtime })
    }
}

// ── Request/Response types ───────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub conversation_id: Option<String>,
}

#[derive(Serialize)]
pub struct ChatResponse {
    pub response: String,
    pub conversation_id: String,
}

#[derive(Deserialize)]
pub struct CreateConversationRequest {
    pub title: Option<String>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

pub fn sync_model_config(config: &mut AgentConfig, manager: &std::sync::Arc<Mutex<ai_launcher_core::llm::manager::LlmManager>>) {
    if let Ok(m) = manager.lock() {
        let active_name = m.status().into_iter().find(|s| s.is_active).map(|s| s.name);
        if let Some(active) = active_name {
            if let Some(c) = m.configs().iter().find(|c| c.name == active) {
                config.model_provider = c.provider_type.clone();
                config.model_name = c.model.clone();
            }
        }
    }
}

/// POST /api/agent/chat — send message, get response
pub fn agent_chat(req: &mut Request, state: &Mutex<AgentState>, llm_manager: &std::sync::Arc<Mutex<ai_launcher_core::llm::manager::LlmManager>>) -> Response<Cursor<Vec<u8>>> {
    let body = match crate::response::read_body(req) {
        Some(b) => b,
        None => return err(400, "Missing request body"),
    };

    let chat_req: ChatRequest = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => return err(400, &format!("Invalid JSON: {}", e)),
    };

    let state = state.lock().unwrap();

    // Get or create conversation
    let conv_id = match chat_req.conversation_id {
        Some(id) => id,
        None => {
            match state.memory.conversations.create_conversation(
                &chat_req.message.chars().take(50).collect::<String>(),
                "nde-chat",
            ) {
                Ok(id) => id,
                Err(e) => return err(500, &format!("Failed to create conversation: {}", e)),
            }
        }
    };

    // Save user message
    if let Err(e) = state.memory.conversations.save_message(
        &conv_id, "user", Some(&chat_req.message), None, None,
    ) {
        return err(500, &format!("Failed to save message: {}", e));
    }

    // Build agent runtime and run
    let response_text = {
        let mut config = state.config.clone();
        sync_model_config(&mut config, llm_manager);
        state.runtime.block_on(async {
            match agent::AgentRuntime::from_config(config) {
                Ok(mut runtime) => runtime.run(&chat_req.message).await,
                Err(e) => Err(e),
            }
        })
    };

    match response_text {
        Ok(text) => {
            // Save assistant response
            state.memory.conversations.save_message(
                &conv_id, "assistant", Some(&text), None, None,
            ).ok();

            ok("Chat response", ChatResponse {
                response: text,
                conversation_id: conv_id,
            })
        }
        Err(e) => {
            // Return error but still provide conversation_id
            let error_msg = format!("Agent error: {}. Check that your LLM provider is running.", e);
            json_resp(502, &json!({
                "success": false,
                "message": error_msg,
                "data": {
                    "conversation_id": conv_id,
                    "response": null
                }
            }))
        }
    }
}

/// GET /api/agent/conversations — list conversations
pub fn list_conversations(state: &Mutex<AgentState>) -> Response<Cursor<Vec<u8>>> {
    let state = state.lock().unwrap();
    match state.memory.conversations.list_conversations(50) {
        Ok(convs) => ok(&format!("{} conversation(s)", convs.len()), convs),
        Err(e) => err(500, &format!("Failed to list conversations: {}", e)),
    }
}

/// GET /api/agent/conversations/{id}/messages — get conversation history
pub fn get_conversation_messages(id: &str, state: &Mutex<AgentState>) -> Response<Cursor<Vec<u8>>> {
    let state = state.lock().unwrap();
    match state.memory.conversations.get_messages(id) {
        Ok(msgs) => ok(&format!("{} message(s)", msgs.len()), msgs),
        Err(e) => err(500, &format!("Failed to get messages: {}", e)),
    }
}

/// GET /api/agent/config — get current agent config
pub fn agent_config(state: &Mutex<AgentState>, llm_manager: &std::sync::Arc<Mutex<ai_launcher_core::llm::manager::LlmManager>>) -> Response<Cursor<Vec<u8>>> {
    let state = state.lock().unwrap();
    let mut config = state.config.clone();
    sync_model_config(&mut config, llm_manager);

    ok("Agent configuration", json!({
        "name": config.name,
        "provider": config.model_provider,
        "model": config.model_name,
        "max_iterations": config.max_iterations,
        "tools": config.enabled_tools,
        "workspace": config.workspace,
    }))
}
