use ai_launcher_core::agent;
use ai_launcher_core::agent::config::AgentConfig;
use ai_launcher_core::memory::MemoryManager;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::response::*;
use serde_json::json;
use tiny_http::Request;

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
#[allow(dead_code)]
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
    memory_substrate: &std::sync::Arc<ai_launcher_core::memory::MemorySubstrate>,
) -> HttpResponse {
    let chat_req: ChatRequest = match parse_body(req) {
        Ok(r) => r,
        Err(resp) => return resp,
    };

    let agent_id = ai_launcher_core::memory::types::AgentId(uuid::Uuid::nil());

    // Get or create conversation
    let session = match chat_req.conversation_id {
        Some(id_str) => {
            if let Ok(id) = uuid::Uuid::parse_str(&id_str) {
                let s_id = ai_launcher_core::memory::types::SessionId(id);
                memory_substrate.session.get_session(s_id).unwrap_or(None)
            } else { None }
        },
        None => None,
    };
    
    let mut session = match session {
        Some(s) => s,
        None => {
            match memory_substrate.session.create_session(agent_id) {
                Ok(mut s) => {
                    s.label = Some(chat_req.message.chars().take(50).collect());
                    let _ = memory_substrate.session.save_session(&s);
                    s
                },
                Err(e) => return err(500, &format!("Failed to create conversation: {}", e)),
            }
        }
    };

    let conv_id = session.id.0.to_string();

    // Save user message
    session.messages.push(ai_launcher_core::memory::types::Message::user(&chat_req.message));
    let _ = memory_substrate.session.save_session(&session);

    let state_lock = state.lock().unwrap();
    // Build agent runtime and run
    let response_text = {
        let mut config = state_lock.config.clone();
        sync_model_config(&mut config, llm_manager);
        state_lock.runtime.block_on(async {
            match agent::AgentRuntime::from_config(config) {
                Ok(mut runtime) => runtime.run(&chat_req.message).await,
                Err(e) => Err(e),
            }
        })
    };

    match response_text {
        Ok(text) => {
            // Save assistant response
            session.messages.push(ai_launcher_core::memory::types::Message::assistant(&text));
            let _ = memory_substrate.session.save_session(&session);

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
pub fn list_conversations(memory_substrate: &std::sync::Arc<ai_launcher_core::memory::MemorySubstrate>) -> HttpResponse {
    let agent_id = ai_launcher_core::memory::types::AgentId(uuid::Uuid::nil());
    match memory_substrate.session.list_sessions(agent_id) {
        Ok(sessions) => {
            let resp: Vec<serde_json::Value> = sessions.into_iter().map(|s| {
                json!({
                    "id": s.id.0.to_string(),
                    "title": s.label.unwrap_or_else(|| "New Chat".into()),
                    "created_at": chrono::Utc::now().to_rfc3339(), // Approximate
                    "updated_at": chrono::Utc::now().to_rfc3339(),
                })
            }).collect();
            ok(&format!("{} conversation(s)", resp.len()), serde_json::json!(resp))
        },
        Err(e) => err(500, &format!("Failed to list conversations: {}", e)),
    }
}

/// GET /api/agent/conversations/{id}/messages — get conversation history
pub fn get_conversation_messages(id: &str, memory_substrate: &std::sync::Arc<ai_launcher_core::memory::MemorySubstrate>) -> HttpResponse {
    if let Ok(uuid) = uuid::Uuid::parse_str(id) {
        let session_id = ai_launcher_core::memory::types::SessionId(uuid);
        match memory_substrate.session.get_session(session_id) {
            Ok(Some(session)) => {
                let resp: Vec<serde_json::Value> = session.messages.into_iter().map(|m| {
                    json!({
                        "role": match m.role {
                            ai_launcher_core::memory::types::Role::User => "user",
                            ai_launcher_core::memory::types::Role::Assistant => "assistant",
                            ai_launcher_core::memory::types::Role::System => "system",
                        },
                        "content": m.content,
                        "created_at": chrono::Utc::now().to_rfc3339(),
                    })
                }).collect();
                ok(&format!("{} message(s)", resp.len()), serde_json::json!(resp))
            },
            Ok(None) => err(404, "Conversation not found"),
            Err(e) => err(500, &format!("Failed to get messages: {}", e)),
        }
    } else {
        err(400, "Invalid conversation ID format")
    }
}

/// GET /api/agent/config — get current agent config
pub fn agent_config(
    state: &Mutex<AgentState>,
    llm_manager: &std::sync::Arc<Mutex<ai_launcher_core::llm::manager::LlmManager>>,
) -> HttpResponse {
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
) -> HttpResponse {
    let ac_req: AutocompleteRequest = match parse_body(req) {
        Ok(r) => r,
        Err(resp) => return resp,
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
