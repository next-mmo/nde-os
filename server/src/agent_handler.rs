use ai_launcher_core::agent;
use ai_launcher_core::agent::config::AgentConfig;
use ai_launcher_core::memory::MemoryManager;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
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
        let mut config = if config_path.exists() {
            AgentConfig::from_file(&config_path).unwrap_or_default()
        } else {
            AgentConfig::default()
        };

        // Resolve relative workspace paths under data_dir so the sandbox
        // is created in AppData, not relative to the process CWD.
        if !PathBuf::from(&config.workspace).is_absolute() {
            config.workspace = data_dir.join(&config.workspace).to_string_lossy().into();
        }

        let runtime = tokio::runtime::Runtime::new()?;

        Ok(Self {
            memory,
            config,
            runtime,
        })
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

#[derive(Deserialize)]
pub struct AutocompleteRequest {
    pub prefix: String,
    pub suffix: String,
    pub filename: Option<String>,
}

#[derive(Serialize)]
pub struct AutocompleteResponse {
    pub completion: String,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

pub fn sync_model_config(
    config: &mut AgentConfig,
    manager: &std::sync::Arc<Mutex<ai_launcher_core::llm::manager::LlmManager>>,
) {
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
pub fn agent_chat(
    req: &mut Request,
    state: &Mutex<AgentState>,
    llm_manager: &std::sync::Arc<Mutex<ai_launcher_core::llm::manager::LlmManager>>,
) -> Response<Cursor<Vec<u8>>> {
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
        &conv_id,
        "user",
        Some(&chat_req.message),
        None,
        None,
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
            state
                .memory
                .conversations
                .save_message(&conv_id, "assistant", Some(&text), None, None)
                .ok();

            ok(
                "Chat response",
                ChatResponse {
                    response: text,
                    conversation_id: conv_id,
                },
            )
        }
        Err(e) => {
            // Return error but still provide conversation_id
            let error_msg = format!(
                "Agent error: {}. Check that your LLM provider is running.",
                e
            );
            json_resp(
                502,
                &json!({
                    "success": false,
                    "message": error_msg,
                    "data": {
                        "conversation_id": conv_id,
                        "response": null
                    }
                }),
            )
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
pub fn agent_config(
    state: &Mutex<AgentState>,
    llm_manager: &std::sync::Arc<Mutex<ai_launcher_core::llm::manager::LlmManager>>,
) -> Response<Cursor<Vec<u8>>> {
    let state = state.lock().unwrap();
    let mut config = state.config.clone();
    sync_model_config(&mut config, llm_manager);

    ok(
        "Agent configuration",
        json!({
            "name": config.name,
            "provider": config.model_provider,
            "model": config.model_name,
            "max_iterations": config.max_iterations,
            "tools": config.enabled_tools,
            "workspace": config.workspace,
        }),
    )
}

/// POST /api/agent/autocomplete — get code suggestions
pub fn agent_autocomplete(
    req: &mut Request,
    llm_manager: &std::sync::Arc<Mutex<ai_launcher_core::llm::manager::LlmManager>>,
    rt: &tokio::runtime::Runtime,
) -> Response<Cursor<Vec<u8>>> {
    let body = match crate::response::read_body(req) {
        Some(b) => b,
        None => return err(400, "Missing request body"),
    };

    let ac_req: AutocompleteRequest = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => return err(400, &format!("Invalid JSON: {}", e)),
    };

    let prompt = format!(
        "You are an expert AI code completion engine for an IDE called OpenCode.\n\n[PREFIX]\n{}\n[SUFFIX]\n{}\n\nProvide only the missing code that goes EXACTLY between [PREFIX] and [SUFFIX]. Do NOT wrap it in markdown backticks. Do NOT provide explanations. Just output the raw code snippet.",
        ac_req.prefix, ac_req.suffix
    );

    let messages = vec![
        ai_launcher_core::llm::Message::system(
            "You are OpenCode autocomplete. Provide raw code ONLY.",
        ),
        ai_launcher_core::llm::Message::user(&prompt),
    ];

    let provider_config = {
        let m = llm_manager.lock().unwrap();
        let active = m.active_name().to_string();
        m.configs().iter().find(|c| c.name == active).cloned()
    };

    let response_text = rt.block_on(async {
        if let Some(c) = provider_config {
            let api_key = c.api_key.clone().or_else(|| {
                c.api_key_env
                    .as_ref()
                    .and_then(|env| std::env::var(env).ok())
            });
            match ai_launcher_core::llm::create_provider(
                &c.provider_type,
                &c.model,
                c.base_url.as_deref(),
                api_key.as_deref(),
            ) {
                Ok(provider) => match provider.chat(&messages, &[]).await {
                    Ok(resp) => Ok(resp.content.unwrap_or_default()),
                    Err(e) => Err(format!("LLM completion failed: {}", e)),
                },
                Err(e) => Err(format!("LLM provider initialization failed: {}", e)),
            }
        } else {
            Err("No active LLM provider configured".to_string())
        }
    });

    match response_text {
        Ok(text) => ok(
            "Autocomplete successful",
            AutocompleteResponse {
                // strip surrounding backticks if the LLM leaked them
                completion: text
                    .trim_start_matches("```")
                    .trim_start_matches("javascript")
                    .trim_start_matches("typescript")
                    .trim_start_matches("rust")
                    .trim_start_matches("svelte")
                    .trim_start_matches("css")
                    .trim_start_matches("html")
                    .trim_start_matches("\n")
                    .trim_end_matches("```")
                    .trim_end_matches("\n")
                    .to_string(),
            },
        ),
        Err(e) => err(500, &e),
    }
}
