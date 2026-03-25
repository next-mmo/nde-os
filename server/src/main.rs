mod agent_handler;
mod handlers;
mod model_handler;
mod openapi;
mod plugin_handler;
mod response;
mod stream_handler;
mod subsystem_handler;

use agent_handler::AgentState;
use ai_launcher_core::agent::manager::{AgentManager, ManagerConfig};
use ai_launcher_core::app_manager::AppManager;
use ai_launcher_core::llm::manager::LlmManager;
use ai_launcher_core::plugins::PluginEngine;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tiny_http::{Method, Request, Server};

use response::*;

/// Cross-platform base directory
fn get_base_dir() -> PathBuf {
    if cfg!(windows) {
        std::env::var("LOCALAPPDATA")
            .map(|p| PathBuf::from(p).join("ai-launcher"))
            .unwrap_or_else(|_| PathBuf::from("C:\\ai-launcher-data"))
    } else {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp"))
            .join(".ai-launcher")
    }
}

/// Route a request to the appropriate handler
fn route(
    req: &mut Request,
    mgr: &AppManager,
    agent: &Mutex<AgentState>,
    rt: &tokio::runtime::Runtime,
    plugin_engine: &Mutex<PluginEngine>,
    llm_manager: Arc<Mutex<LlmManager>>,
    data_dir: &std::path::Path,
    agent_manager: &Arc<tokio::sync::Mutex<AgentManager>>,
) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    let method = req.method().clone();
    let url = req.url().to_string();
    let path = url.split('?').next().unwrap_or(&url);

    // CORS preflight
    if matches!(method, Method::Options) {
        return handlers::cors_preflight();
    }

    // Static routes
    match (method.clone(), path) {
        (Method::Get, "/swagger-ui" | "/swagger-ui/" | "/docs" | "/docs/") => {
            return html(openapi::SWAGGER_HTML);
        }
        (Method::Get, "/api-docs/openapi.json") => {
            return json_resp(200, &openapi::openapi_spec());
        }
        (Method::Get, "/" | "") => {
            return html("<html><meta http-equiv='refresh' content='0;url=/swagger-ui/'></html>");
        }
        (Method::Get, "/api/health") => return handlers::health(),
        (Method::Get, "/api/system") => return handlers::system_info(mgr),
        (Method::Get, "/api/system/resources") => return handlers::system_resources(mgr),
        (Method::Get, "/api/catalog") => return handlers::catalog(mgr),
        (Method::Get, "/api/apps") => return handlers::list_apps(mgr),
        (Method::Post, "/api/apps") => return handlers::install_app(req, mgr),
        (Method::Post, "/api/store/upload") => return handlers::store_upload(req, mgr),
        // Agent chat API
        (Method::Post, "/api/agent/chat") => return agent_handler::agent_chat(req, agent, &llm_manager),
        (Method::Post, "/api/agent/chat/stream") => {
            return stream_handler::handle_stream_chat(req, rt, agent, &llm_manager, Some(agent_manager));
        }
        // Task-based agent API (Phase 3)
        (Method::Post, "/api/agent/tasks") => {
            return stream_handler::spawn_task(req, rt, agent_manager);
        }
        (Method::Get, "/api/agent/tasks") => {
            return stream_handler::list_tasks(agent_manager, rt);
        }
        (Method::Get, "/api/agent/conversations") => return agent_handler::list_conversations(agent),
        (Method::Get, "/api/agent/config") => return agent_handler::agent_config(agent, &llm_manager),
        (Method::Get, "/api/agent/tools") => return subsystem_handler::list_agent_tools(),
        // Plugin API
        (Method::Get, "/api/plugins") => return plugin_handler::list_plugins(plugin_engine),
        (Method::Post, "/api/plugins/discover") => return plugin_handler::discover_plugins(rt, plugin_engine),
        // Model/Provider API
        (Method::Get, "/api/models") => return model_handler::list_models(&llm_manager),
        (Method::Get, "/api/models/active") => return model_handler::active_model(&llm_manager),
        (Method::Get, "/api/models/recommendations") => return model_handler::recommend_gguf_models(mgr),
        (Method::Get, "/api/models/local") => return model_handler::list_local_models(),
        (Method::Post, "/api/models/switch") => return model_handler::switch_model(req, &llm_manager),
        (Method::Post, "/api/models/providers") => return model_handler::add_provider(req, &llm_manager, rt),
        (Method::Post, "/api/models/verify") => return model_handler::verify_provider(req, rt),
        // Codex OAuth
        (Method::Post, "/api/codex/oauth/start") => return model_handler::codex_oauth_start(req, llm_manager.clone(), rt, data_dir),
        (Method::Get, "/api/codex/oauth/status") => return model_handler::codex_oauth_status(data_dir),
        // Channels
        (Method::Get, "/api/channels") => return subsystem_handler::list_channels(data_dir),
        // MCP
        (Method::Get, "/api/mcp/tools") => return subsystem_handler::list_mcp_tools(),
        (Method::Get, "/api/mcp/servers") => return subsystem_handler::list_mcp_servers(),
        // Skills
        (Method::Get, "/api/skills") => return subsystem_handler::list_skills(),
        // Knowledge
        (Method::Get, "/api/knowledge") => return subsystem_handler::list_knowledge(data_dir),
        // Memory
        (Method::Get, "/api/memory") => return subsystem_handler::list_memory(data_dir),
        // OpenViking
        (Method::Get, "/api/viking/status") => return subsystem_handler::viking_status(rt),
        _ => {}
    }

    // Agent task routes: /api/agent/tasks/{id}/...
    if path.starts_with("/api/agent/tasks/") {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        return match (method.clone(), parts.as_slice()) {
            (Method::Get, ["api", "agent", "tasks", id]) => {
                stream_handler::get_task(id, agent_manager, rt)
            }
            (Method::Get, ["api", "agent", "tasks", id, "stream"]) => {
                stream_handler::stream_task(id, rt, agent_manager)
            }
            (Method::Post, ["api", "agent", "tasks", id, "cancel"]) => {
                stream_handler::cancel_task(id, rt, agent_manager)
            }
            _ => err(404, &format!("Not found: {}", path)),
        };
    }

    // Agent conversation messages: /api/agent/conversations/{id}/messages
    if path.starts_with("/api/agent/conversations/") {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        return match (method.clone(), parts.as_slice()) {
            (Method::Get, ["api", "agent", "conversations", id, "messages"]) => {
                agent_handler::get_conversation_messages(id, agent)
            }
            _ => err(404, &format!("Not found: {}", path)),
        };
    }

    // Plugin dynamic routes: /api/plugins/{id}/...
    if path.starts_with("/api/plugins/") {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        return match (method.clone(), parts.as_slice()) {
            (Method::Get, ["api", "plugins", id]) => plugin_handler::get_plugin(id, plugin_engine),
            (Method::Post, ["api", "plugins", id, "install"]) => {
                plugin_handler::install_plugin(id, rt, plugin_engine)
            }
            (Method::Post, ["api", "plugins", id, "start"]) => {
                plugin_handler::start_plugin(id, rt, plugin_engine)
            }
            (Method::Post, ["api", "plugins", id, "stop"]) => {
                plugin_handler::stop_plugin(id, rt, plugin_engine)
            }
            (Method::Get, ["api", "plugins", id, "logs"]) => {
                plugin_handler::get_plugin_logs(id, plugin_engine)
            }
            (Method::Delete, ["api", "plugins", id, "logs"]) => {
                plugin_handler::clear_plugin_logs(id, plugin_engine)
            }
            _ => err(404, &format!("Not found: {}", path)),
        };
    }

    // Dynamic provider routes: /api/models/providers/{name}
    if path.starts_with("/api/models/providers/") {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        return match (method.clone(), parts.as_slice()) {
            (Method::Delete, ["api", "models", "providers", name]) => {
                model_handler::remove_provider(name, &llm_manager)
            }
            _ => err(404, &format!("Not found: {}", path)),
        };
    }

    // Dynamic channel routes: /api/channels/{name}/configure
    if path.starts_with("/api/channels/") {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        return match (method.clone(), parts.as_slice()) {
            (Method::Post, ["api", "channels", _name, "configure"]) => {
                subsystem_handler::configure_channel(req, data_dir)
            }
            _ => err(404, &format!("Not found: {}", path)),
        };
    }

    // Dynamic routes: /api/apps/{id}/... and /api/sandbox/{id}/...
    if path.starts_with("/api/apps/") || path.starts_with("/api/sandbox/") {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        return match (method, parts.as_slice()) {
            (Method::Get,    ["api", "apps", id])            => handlers::get_app(id, mgr),
            (Method::Delete, ["api", "apps", id])            => handlers::uninstall_app(id, mgr),
            (Method::Post,   ["api", "apps", id, "launch"])  => handlers::launch_app(id, mgr),
            (Method::Post,   ["api", "apps", id, "stop"])    => handlers::stop_app(id, mgr),
            (Method::Get,    ["api", "sandbox", id, "verify"]) => handlers::verify_sandbox(id, mgr),
            (Method::Get,    ["api", "sandbox", id, "disk"])   => handlers::disk_usage(id, mgr),
            _ => err(404, &format!("Not found: {}", path)),
        };
    }

    if path.starts_with("/api/store/") {
        return err(404, &format!("Not found: {}", path));
    }

    // Knowledge search: /api/knowledge/search?q=...
    if path.starts_with("/api/knowledge/search") {
        let query = url.split("q=").nth(1).unwrap_or("");
        let decoded = urlencoding::decode(query).unwrap_or_default();
        return subsystem_handler::search_knowledge(&decoded, data_dir);
    }

    // Memory by key: /api/memory/{key}
    if path.starts_with("/api/memory/") {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        if let ["api", "memory", key] = parts.as_slice() {
            return subsystem_handler::get_memory(key, data_dir);
        }
    }

    err(404, &format!("Not found: {}", path))
}

fn main() {
    let base_dir = get_base_dir();
    std::fs::create_dir_all(&base_dir).ok();

    let mgr = Arc::new(AppManager::new(&base_dir).expect("Failed to init AppManager"));
    let agent = Arc::new(Mutex::new(
        AgentState::new(&base_dir).expect("Failed to init AgentState")
    ));

    // Phase 2: Tokio runtime for async operations
    let rt = Arc::new(
        tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime"),
    );

    // Phase 2: Plugin engine
    // In dev mode (running from repo root), use repo plugins/ directly for hot reload.
    // In production, fall back to base_dir/plugins.
    let plugins_dir = {
        let repo_plugins = std::env::current_dir()
            .unwrap_or_default()
            .join("plugins");
        if repo_plugins.exists() && repo_plugins.is_dir() {
            println!("  [plugins] Dev mode: using repo plugins/ at {}", repo_plugins.display());
            repo_plugins
        } else {
            base_dir.join("plugins")
        }
    };
    std::fs::create_dir_all(&plugins_dir).ok();
    let mut engine = PluginEngine::new(&plugins_dir);

    // Register bundled plugins directory (shipped with the repo).
    // In dev mode the server runs from the repo root via dev.sh,
    // so `./plugins` relative to CWD contains built-in plugins.
    // In production, look relative to the executable.
    let cwd_plugins = std::env::current_dir()
        .map(|d| d.join("plugins"))
        .unwrap_or_default();
    if cwd_plugins.exists() && cwd_plugins != plugins_dir {
        engine.add_search_dir(cwd_plugins);
    }

    // Also check next to the executable (for release builds)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let exe_plugins = exe_dir.join("plugins");
            if exe_plugins.exists() && exe_plugins != plugins_dir {
                engine.add_search_dir(exe_plugins);
            }
        }
    }

    let plugin_engine = Arc::new(Mutex::new(engine));

    // Auto-discover plugins on startup
    if let Ok(mut engine) = plugin_engine.lock() {
        match engine.discover() {
            Ok(manifests) => {
                if !manifests.is_empty() {
                    println!("  Plugins:     {} discovered", manifests.len());
                }
            }
            Err(e) => eprintln!("  Plugin discovery error: {}", e),
        }
    }

    // Phase 2: LLM Manager
    let llm_config_path = base_dir.join("llm_providers.json");
    let mut llm_mgr = LlmManager::load_from_disk(&llm_config_path).unwrap_or_else(|_| {
        let mut new_mgr = LlmManager::new();
        new_mgr.set_persistence_path(llm_config_path);
        new_mgr
    });

    // Auto-add default GGUF and Ollama provider if none exist (zero-config local inference)
    if llm_mgr.configs().is_empty() {
        let _ = llm_mgr.add_from_config(ai_launcher_core::llm::manager::ProviderConfig {
            name: "local-gguf".into(),
            provider_type: "gguf".into(),
            model: "tinyllama-1.1b".into(),
            base_url: None,
            api_key: None,
            api_key_env: None,
            max_tokens: 2048,
        });
        let _ = llm_mgr.add_from_config(ai_launcher_core::llm::manager::ProviderConfig {
            name: "local-ollama".into(),
            provider_type: "ollama".into(),
            model: "llama3.2".into(),
            base_url: None,
            api_key: None,
            api_key_env: None,
            max_tokens: 4096,
        });
    }

    let llm_manager = Arc::new(Mutex::new(llm_mgr));

    // Phase 3: Agent Manager (24/7 task runtime)
    let agent_manager = {
        let agent_state_ref = agent.lock().unwrap();
        let mut agent_config = agent_state_ref.config.clone();
        drop(agent_state_ref);
        // Sync with active LLM provider
        agent_handler::sync_model_config(&mut agent_config, &llm_manager);

        let mut mgr_config = ManagerConfig::default();
        mgr_config.executor.audit_dir = base_dir.join("audit");
        std::fs::create_dir_all(&mgr_config.executor.audit_dir).ok();

        match AgentManager::new(mgr_config, agent_config, &base_dir) {
            Ok(mgr) => {
                println!("  Agent Mgr:   initialized (task-based runtime)");
                Arc::new(tokio::sync::Mutex::new(mgr))
            }
            Err(e) => {
                eprintln!("  Agent Mgr:   FAILED — {}", e);
                eprintln!("               (falling back to legacy per-request runtime)");
                // Create a dummy config to avoid crash — manager will error on spawn
                let fallback_config = ai_launcher_core::agent::config::AgentConfig::default();
                let mgr_config = ManagerConfig::default();
                // Try with absolute workspace path
                let mut fb_config = fallback_config;
                fb_config.workspace = base_dir.join("workspace").to_string_lossy().into();
                let mut fb_mgr_config = mgr_config;
                fb_mgr_config.executor.audit_dir = base_dir.join("audit");
                match AgentManager::new(fb_mgr_config, fb_config, &base_dir) {
                    Ok(mgr) => Arc::new(tokio::sync::Mutex::new(mgr)),
                    Err(_) => {
                        // Last resort: we need to provide something
                        panic!("Cannot initialize AgentManager even with defaults: {}", e);
                    }
                }
            }
        }
    };

    // Boot the agent manager (recover crashed tasks, start heartbeat)
    {
        let mgr = agent_manager.clone();
        let rt_ref = rt.clone();
        rt_ref.block_on(async {
            let m = mgr.lock().await;
            if let Err(e) = m.on_boot().await {
                eprintln!("  Agent boot error: {}", e);
            }
        });
    }

    let server = Server::http("0.0.0.0:8080").expect("Failed to bind :8080");

    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    println!();
    println!("  +=============================================+");
    println!("  |  NDE-OS AI Operating System v0.2.0          |");
    println!("  |  Phase 2 — Gateway + LLM + Plugins + MCP   |");
    println!("  +=============================================+");
    println!();
    println!("  Platform:    {}/{}", os, arch);
    println!("  Data dir:    {}", base_dir.display());
    println!("  Server:      http://localhost:8080");
    println!("  Swagger UI:  http://localhost:8080/swagger-ui/");
    println!("  CLI:         nde --help");
    println!();
    println!("  Core:");
    println!("    GET    /api/health");
    println!("    GET    /api/system");
    println!("    GET    /api/system/resources");
    println!("    GET    /api/catalog");
    println!("    GET    /api/apps");
    println!("    POST   /api/apps");
    println!("    GET    /api/apps/{{id}}");
    println!("    DELETE /api/apps/{{id}}");
    println!("    POST   /api/apps/{{id}}/launch");
    println!("    POST   /api/apps/{{id}}/stop");
    println!("    GET    /api/sandbox/{{id}}/verify");
    println!("    GET    /api/sandbox/{{id}}/disk");
    println!("    POST   /api/store/upload");
    println!();
    println!("  Agent:");
    println!("    POST   /api/agent/chat");
    println!("    POST   /api/agent/chat/stream          <- real SSE streaming");
    println!("    GET    /api/agent/conversations");
    println!("    GET    /api/agent/conversations/{{id}}/messages");
    println!("    GET    /api/agent/config");
    println!("    GET    /api/agent/tools");
    println!();
    println!("  Agent Tasks (Phase 3):");
    println!("    POST   /api/agent/tasks                <- spawn task");
    println!("    GET    /api/agent/tasks                <- list tasks");
    println!("    GET    /api/agent/tasks/{{id}}           <- task status");
    println!("    GET    /api/agent/tasks/{{id}}/stream    <- real SSE stream");
    println!("    POST   /api/agent/tasks/{{id}}/cancel    <- cancel task");
    println!();
    println!("  Plugins:");
    println!("    GET    /api/plugins");
    println!("    GET    /api/plugins/{{id}}");
    println!("    POST   /api/plugins/discover");
    println!("    POST   /api/plugins/{{id}}/install");
    println!("    POST   /api/plugins/{{id}}/start");
    println!("    POST   /api/plugins/{{id}}/stop");
    println!("    GET    /api/plugins/{{id}}/logs");
    println!("    DELETE /api/plugins/{{id}}/logs");
    println!();
    println!("  Models:");
    println!("    GET    /api/models");
    println!("    GET    /api/models/active");
    println!("    GET    /api/models/recommendations");
    println!("    GET    /api/models/local");
    println!("    POST   /api/models/switch");
    println!("    POST   /api/models/providers");
    println!("    POST   /api/models/verify");
    println!("    DELETE /api/models/providers/{{name}}");
    println!();
    println!("  Codex OAuth:");
    println!("    POST   /api/codex/oauth/start");
    println!("    GET    /api/codex/oauth/status");
    println!();
    println!("  Channels:");
    println!("    GET    /api/channels");
    println!();
    println!("  MCP:");
    println!("    GET    /api/mcp/tools");
    println!("    GET    /api/mcp/servers");
    println!();
    println!("  Skills:");
    println!("    GET    /api/skills                    <- real SkillLoader");
    println!();
    println!("  Knowledge:");
    println!("    GET    /api/knowledge                 <- real KnowledgeGraph");
    println!("    GET    /api/knowledge/search?q={{query}}");
    println!();
    println!("  Memory:");
    println!("    GET    /api/memory                    <- real KvStore");
    println!("    GET    /api/memory/{{key}}");
    println!();
    println!("  OpenViking:");
    println!("    GET    /api/viking/status              <- context database");
    println!();

    loop {
        match server.recv() {
            Ok(mut request) => {
                let mgr = mgr.clone();
                let agent = agent.clone();
                let rt = rt.clone();
                let plugin_engine = plugin_engine.clone();
                let llm_manager = llm_manager.clone();
                let base_dir = base_dir.clone();
                let agent_manager = agent_manager.clone();

                std::thread::spawn(move || {
                    let response = route(
                        &mut request,
                        &mgr,
                        &agent,
                        &rt,
                        &plugin_engine,
                        llm_manager,
                        &base_dir,
                        &agent_manager,
                    );
                    if let Err(e) = request.respond(response) {
                        eprintln!("Response error: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("Recv error: {}", e);
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    }
}
