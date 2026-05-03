#![recursion_limit = "512"]
mod actors;
mod agent;
mod apps;
mod config;
mod gateway;
mod kanban;
mod models;
mod openapi;
mod plugins;
mod response;
mod router;
mod subsystems;

use agent::AgentState;
use ai_launcher_core::actor::runner::ActorRunner;
use ai_launcher_core::agent::manager::{AgentManager, ManagerConfig};
use ai_launcher_core::app_manager::AppManager;
use ai_launcher_core::downloader::{DownloadEngine, JobStore};
use ai_launcher_core::llm::manager::LlmManager;
use ai_launcher_core::media::ffmpeg::ensure_ffmpeg;
use ai_launcher_core::plugins::PluginEngine;
use router::AppState;
use std::sync::{Arc, Mutex};
use tiny_http::Server;

/// Fixed-size worker count for the request thread pool.
const WORKER_THREADS: usize = 8;

// ── Banner ──────────────────────────────────────────────────────────────────

const BANNER: &str = "\
  +=============================================+
  |  NDE-OS AI Operating System v0.2.0          |
  |  Phase 2 — Gateway + LLM + Plugins + MCP   |
  +=============================================+";

const ROUTES: &str = "\
  Core:
    GET    /api/health
    GET    /api/system
    GET    /api/system/resources
    GET    /api/catalog
    GET    /api/apps
    POST   /api/apps
    GET    /api/apps/{id}
    DELETE /api/apps/{id}
    POST   /api/apps/{id}/launch
    POST   /api/apps/{id}/stop
    GET    /api/sandbox/{id}/verify
    GET    /api/sandbox/{id}/disk
    POST   /api/store/upload

  Agent:
    POST   /api/agent/chat
    POST   /api/agent/chat/stream          <- real SSE streaming
    GET    /api/agent/conversations
    GET    /api/agent/conversations/{id}/messages
    GET    /api/agent/config
    GET    /api/agent/tools

  Agent Tasks (Phase 3):
    POST   /api/agent/tasks                <- spawn task
    GET    /api/agent/tasks                <- list tasks
    GET    /api/agent/tasks/{id}           <- task status
    GET    /api/agent/tasks/{id}/stream    <- real SSE stream
    POST   /api/agent/tasks/{id}/cancel    <- cancel task

  Plugins:
    GET    /api/plugins
    GET    /api/plugins/{id}
    POST   /api/plugins/discover
    POST   /api/plugins/{id}/install
    POST   /api/plugins/{id}/start
    POST   /api/plugins/{id}/stop
    GET    /api/plugins/{id}/logs
    DELETE /api/plugins/{id}/logs

  Models:
    GET    /api/models
    GET    /api/models/active
    GET    /api/models/recommendations
    GET    /api/models/local
    POST   /api/models/switch
    POST   /api/models/providers
    POST   /api/models/verify
    DELETE /api/models/providers/{name}

  Codex OAuth:
    POST   /api/codex/oauth/start
    GET    /api/codex/oauth/status

  Channels:
    GET    /api/channels

  MCP:
    GET    /api/mcp/tools
    GET    /api/mcp/servers

  Skills:
    GET    /api/skills                    <- real SkillLoader

  Knowledge:
    GET    /api/knowledge                 <- real KnowledgeGraph
    GET    /api/knowledge/search?q={query}

  Memory:
    GET    /api/memory                    <- real KvStore
    GET    /api/memory/{key}


  Kanban:
    GET    /api/kanban/tasks                <- list all tasks
    POST   /api/kanban/tasks                <- create task
    PUT    /api/kanban/tasks/{file}         <- update status
    DELETE /api/kanban/tasks/{file}         <- delete task
    GET    /api/kanban/tasks/{file}/content <- read content
    PUT    /api/kanban/tasks/{file}/content <- write content

  KFA (Khmer Forced Aligner):
    POST   /api/kfa/align                   <- multipart WAV + text → timestamps
    POST   /api/kfa/align-json              <- JSON base64 WAV + text → timestamps
";

fn main() {
    let base_dir = config::base_dir();
    std::fs::create_dir_all(&base_dir).ok();

    let mgr = Arc::new(AppManager::new(&base_dir).expect("Failed to init AppManager"));
    let agent = Arc::new(Mutex::new(
        AgentState::new(&base_dir).expect("Failed to init AgentState"),
    ));

    // Tokio runtime for async operations
    let rt = Arc::new(tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime"));

    // Plugin engine
    let plugins_dir = {
        let repo_plugins = std::env::current_dir().unwrap_or_default().join("plugins");
        if repo_plugins.exists() && repo_plugins.is_dir() {
            println!(
                "  [plugins] Dev mode: using repo plugins/ at {}",
                repo_plugins.display()
            );
            repo_plugins
        } else {
            base_dir.join("plugins")
        }
    };
    std::fs::create_dir_all(&plugins_dir).ok();
    let mut engine = PluginEngine::new(&plugins_dir);

    // Register bundled plugins directory
    let cwd_plugins = std::env::current_dir()
        .map(|d| d.join("plugins"))
        .unwrap_or_default();
    if cwd_plugins.exists() && cwd_plugins != plugins_dir {
        engine.add_search_dir(cwd_plugins);
    }

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

    // LLM Manager
    let llm_config_path = base_dir.join("llm_providers.json");
    let llm_mgr = LlmManager::load_from_disk(&llm_config_path).unwrap_or_else(|_| {
        let mut new_mgr = LlmManager::new();
        new_mgr.set_persistence_path(llm_config_path);
        new_mgr
    });
    let llm_manager = Arc::new(Mutex::new(llm_mgr));

    // Agent Manager (24/7 task runtime)
    let agent_manager = {
        let agent_state_ref = agent.lock().unwrap();
        let mut agent_config = agent_state_ref.config.clone();
        drop(agent_state_ref);
        agent::handler::sync_model_config(&mut agent_config, &llm_manager);

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
                let fallback_config = ai_launcher_core::agent::config::AgentConfig::default();
                let mgr_config = ManagerConfig::default();
                let mut fb_config = fallback_config;
                fb_config.workspace = base_dir.join("workspace").to_string_lossy().into();
                let mut fb_mgr_config = mgr_config;
                fb_mgr_config.executor.audit_dir = base_dir.join("audit");
                match AgentManager::new(fb_mgr_config, fb_config, &base_dir) {
                    Ok(mgr) => Arc::new(tokio::sync::Mutex::new(mgr)),
                    Err(_) => {
                        panic!("Cannot initialize AgentManager even with defaults: {}", e);
                    }
                }
            }
        }
    };

    // Boot agent manager (recover crashed tasks, start heartbeat)
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

    // Shared gateway log buffer
    let log_buffer = gateway::new_shared();

    // Desktop action queue (shared between gateway and REST API)
    let desktop_actions: router::DesktopActionQueue = Arc::new(Mutex::new(Vec::new()));

    // Memory Substrate
    let memory_db_path = base_dir.join("memory.db");
    let memory_substrate = Arc::new(ai_launcher_core::memory::MemorySubstrate::open(&memory_db_path).expect("Failed to init MemorySubstrate"));
    println!("  Memory:      initialized (MemorySubstrate)");

    // Start memory consolidation background task (runs every hour)
    {
        let memory = memory_substrate.clone();
        rt.spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
            loop {
                interval.tick().await;
                if let Err(e) = memory.consolidation.consolidate() {
                    eprintln!("  Memory consolidation failed: {}", e);
                }
            }
        });
    }

    // Telegram gateway
    let tg_state = Arc::new(gateway::GatewayState::new());

    if let Some(tg_config) = gateway::TelegramGatewayConfig::load(&base_dir) {
        gateway::start_telegram_gateway(
            tg_config,
            agent_manager.clone(),
            llm_manager.clone(),
            rt.handle().clone(),
            tg_state.clone(),
            log_buffer.clone(),
            desktop_actions.clone(),
            memory_substrate.clone(),
        );
    } else {
        println!("  Telegram:    not configured (set via Channels settings)");
    }

    gateway::log_info(&log_buffer, "system", "NDE-OS server started");


    // Actor Runner (Shield Actor system)
    let actor_runner = Arc::new(tokio::sync::Mutex::new(ActorRunner::new(&base_dir)));
    println!("  Actors:      initialized (Shield Actor system)");

    // Download Engine
    let job_store = Arc::new(JobStore::open(&base_dir).expect("Failed to init JobStore"));
    let ffmpeg_bins = ensure_ffmpeg(&base_dir).expect("Failed to get ffmpeg");
    let download_engine = Arc::new(DownloadEngine::new(job_store, ffmpeg_bins));
    println!("  Downloads:   initialized (DownloadEngine)");

    // Build shared AppState
    let state = Arc::new(AppState {
        mgr,
        agent,
        rt,
        plugin_engine,
        llm_manager,
        data_dir: base_dir.clone(),
        agent_manager,
        tg_state,
        log_buffer,
        actor_runner,
        desktop_actions,
        download_engine,
        memory_substrate,
    });

    let server = Server::http("0.0.0.0:8080").expect("Failed to bind :8080");

    // Print banner
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    println!("\n{}\n", BANNER);
    println!("  Platform:    {}/{}", os, arch);
    println!("  Data dir:    {}", base_dir.display());
    println!("  Server:      http://localhost:8080");
    println!("  Swagger UI:  http://localhost:8080/swagger-ui/");
    println!("  CLI:         nde --help\n");
    print!("{}\n", ROUTES);

    // ── Thread-pool request loop ────────────────────────────────────────────
    // Pre-spawn a fixed number of worker threads. Each thread pulls requests
    // from a channel, preventing unbounded thread creation under load.
    let (tx, rx) = std::sync::mpsc::sync_channel::<tiny_http::Request>(64);
    let rx = Arc::new(Mutex::new(rx));

    for _ in 0..WORKER_THREADS {
        let rx = rx.clone();
        let state = state.clone();
        std::thread::spawn(move || {
            loop {
                let mut request = match rx.lock().unwrap().recv() {
                    Ok(req) => req,
                    Err(_) => break, // Sender dropped — server shutting down
                };
                let response = router::route(&mut request, &state);
                if let Err(e) = request.respond(response) {
                    eprintln!("Response error: {}", e);
                }
            }
        });
    }

    // Main thread: accept requests and dispatch to worker pool
    loop {
        match server.recv() {
            Ok(request) => {
                if let Err(e) = tx.try_send(request) {
                    // Pool is saturated — still handle it but log
                    match e {
                        std::sync::mpsc::TrySendError::Full(mut req) => {
                            // Blocking send as backpressure
                            let state = state.clone();
                            std::thread::spawn(move || {
                                let response = router::route(&mut req, &state);
                                if let Err(e) = req.respond(response) {
                                    eprintln!("Response error: {}", e);
                                }
                            });
                        }
                        std::sync::mpsc::TrySendError::Disconnected(_) => break,
                    }
                }
            }
            Err(e) => {
                eprintln!("Recv error: {}", e);
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    }
}
