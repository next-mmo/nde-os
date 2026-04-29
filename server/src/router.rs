/// HTTP router — matches method + path to handler functions.
///
/// All shared server state is bundled into `AppState` so `route()` takes
/// a single `&AppState` instead of 11 individual parameters.
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use ai_launcher_core::actor::runner::ActorRunner;
use ai_launcher_core::agent::manager::AgentManager;
use ai_launcher_core::app_manager::AppManager;
use ai_launcher_core::downloader::DownloadEngine;
use ai_launcher_core::llm::manager::LlmManager;
use ai_launcher_core::plugins::PluginEngine;
use tiny_http::{Method, Request};

use crate::agent::AgentState;
use crate::gateway::{GatewayState, SharedLogBuffer};
use crate::response::*;
use crate::{actors, agent, apps, gateway, kanban, models, openapi, plugins, subsystems};

/// Thread-safe queue for desktop actions pushed by gateways (Telegram, etc).
/// The frontend polls `GET /api/desktop/pending-actions` to drain and execute.
pub type DesktopActionQueue = Arc<Mutex<Vec<DesktopAction>>>;

/// A pending desktop action (e.g. open an app window).
#[derive(Clone, serde::Serialize)]
pub struct DesktopAction {
    pub kind: String,
    pub app_id: String,
}

/// Bundles all shared server state into a single struct.
pub struct AppState {
    pub mgr: Arc<AppManager>,
    pub agent: Arc<Mutex<AgentState>>,
    pub rt: Arc<tokio::runtime::Runtime>,
    pub plugin_engine: Arc<Mutex<PluginEngine>>,
    pub llm_manager: Arc<Mutex<LlmManager>>,
    pub data_dir: PathBuf,
    pub agent_manager: Arc<tokio::sync::Mutex<AgentManager>>,
    pub viking: Arc<Mutex<ai_launcher_core::openviking::VikingProcess>>,
    pub tg_state: Arc<GatewayState>,
    pub log_buffer: SharedLogBuffer,
    pub actor_runner: Arc<tokio::sync::Mutex<ActorRunner>>,
    pub desktop_actions: DesktopActionQueue,
    pub download_engine: Arc<DownloadEngine>,
}

impl AppState {
    /// Sync the AgentManager's internal LLM provider with the current active model.
    /// Called after any model switch/add/OAuth so chat streaming uses the right provider.
    pub fn sync_agent_provider(&self) {
        let mut config = {
            let state = self.agent.lock().unwrap();
            state.config.clone()
        };
        agent::handler::sync_model_config(&mut config, &self.llm_manager);
        self.rt.block_on(async {
            let mut mgr = self.agent_manager.lock().await;
            if let Err(e) = mgr.update_provider(&config) {
                tracing::warn!("Failed to sync AgentManager provider: {}", e);
            } else {
                tracing::info!(
                    provider = %config.model_provider,
                    model = %config.model_name,
                    "AgentManager provider synced"
                );
            }
        });
    }
}

/// Route a request to the appropriate handler.
pub fn route(req: &mut Request, state: &AppState) -> HttpResponse {
    let method = req.method().clone();
    let url = req.url().to_string();
    let path = url.split('?').next().unwrap_or(&url);

    // CORS preflight
    if matches!(method, Method::Options) {
        return apps::cors_preflight();
    }

    // ── Static routes ───────────────────────────────────────────────────────

    match (method.clone(), path) {
        // Docs
        (Method::Get, "/swagger-ui" | "/swagger-ui/" | "/docs" | "/docs/") => {
            return html(openapi::SWAGGER_HTML);
        }
        (Method::Get, "/api-docs/openapi.json") => {
            return json_resp(200, &openapi::openapi_spec());
        }
        (Method::Get, "/" | "") => {
            return html("<html><meta http-equiv='refresh' content='0;url=/swagger-ui/'></html>");
        }
        // System
        (Method::Get, "/api/health") => return apps::health(),
        (Method::Get, "/api/system") => return apps::system_info(&state.mgr),
        (Method::Get, "/api/system/resources") => return apps::system_resources(&state.mgr),
        (Method::Get, "/api/catalog") => return apps::catalog(&state.mgr),
        // Desktop remote actions (polled by the Svelte frontend)
        (Method::Get, "/api/desktop/pending-actions") => {
            let actions: Vec<DesktopAction> = {
                let mut q = state.desktop_actions.lock().unwrap();
                q.drain(..).collect()
            };
            return ok(&format!("{} actions", actions.len()), actions);
        }
        // Apps
        (Method::Get, "/api/apps") => return apps::list_apps(&state.mgr),
        (Method::Post, "/api/apps") => return apps::install_app(req, &state.mgr),
        (Method::Post, "/api/store/upload") => return apps::store_upload(req, &state.mgr),
        // Agent chat
        (Method::Post, "/api/agent/chat") => {
            return agent::handler::agent_chat(req, &state.agent, &state.llm_manager)
        }
        (Method::Post, "/api/agent/chat/stream") => {
            return agent::stream::handle_stream_chat(
                req,
                &state.rt,
                &state.agent,
                &state.llm_manager,
                Some(&state.agent_manager),
            );
        }
        (Method::Post, "/api/agent/autocomplete") => {
            return agent::handler::agent_autocomplete(req, &state.llm_manager, &state.rt)
        }
        // Agent tasks
        (Method::Post, "/api/agent/tasks") => {
            return agent::stream::spawn_task(req, &state.rt, &state.agent_manager);
        }
        (Method::Get, "/api/agent/tasks") => {
            return agent::stream::list_tasks(&state.agent_manager, &state.rt);
        }
        (Method::Get, "/api/agent/conversations") => {
            return agent::handler::list_conversations(&state.agent)
        }
        (Method::Get, "/api/agent/config") => {
            return agent::handler::agent_config(&state.agent, &state.llm_manager)
        }
        (Method::Get, "/api/agent/tools") => return subsystems::list_agent_tools(),
        // Plugins
        (Method::Get, "/api/plugins") => return plugins::list_plugins(&state.plugin_engine),
        (Method::Post, "/api/plugins/discover") => {
            return plugins::discover_plugins(&state.rt, &state.plugin_engine)
        }
        // Models
        (Method::Get, "/api/models") => return models::list_models(&state.llm_manager),
        (Method::Get, "/api/models/active") => return models::active_model(&state.llm_manager),
        (Method::Get, "/api/models/recommendations") => {
            return models::recommend_gguf_models(&state.mgr)
        }
        (Method::Get, "/api/models/local") => return models::list_local_models(&state.data_dir),
        (Method::Post, "/api/models/switch") => {
            let resp = models::switch_model(req, &state.llm_manager);
            state.sync_agent_provider();
            return resp;
        }
        (Method::Post, "/api/models/providers") => {
            let resp = models::add_provider(req, &state.llm_manager, &state.rt);
            state.sync_agent_provider();
            return resp;
        }
        (Method::Post, "/api/models/verify") => {
            return models::verify_provider(req, &state.rt, &state.data_dir)
        }
        // Codex OAuth
        (Method::Post, "/api/codex/oauth/start") => {
            let resp = models::codex_oauth_start(
                req,
                state.llm_manager.clone(),
                &state.rt,
                &state.data_dir,
            );
            state.sync_agent_provider();
            return resp;
        }
        (Method::Get, "/api/codex/oauth/status") => {
            return models::codex_oauth_status(&state.data_dir)
        }
        // Channels
        (Method::Get, "/api/channels") => {
            return subsystems::list_channels(&state.data_dir, &state.tg_state)
        }
        // Gateway logs
        (Method::Get, "/api/logs") => {
            let since_id: u64 = url
                .split("since=")
                .nth(1)
                .and_then(|s| s.split('&').next())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            if let Ok(buf) = state.log_buffer.lock() {
                let entries = buf.since(since_id);
                return ok(
                    &format!("{} log entries", entries.len()),
                    serde_json::json!(entries),
                );
            }
            return err(500, "Log buffer lock failed");
        }
        // MCP
        (Method::Get, "/api/mcp/tools") => return subsystems::list_mcp_tools(),
        (Method::Get, "/api/mcp/servers") => return subsystems::list_mcp_servers(),
        // Skills
        (Method::Get, "/api/skills") => return subsystems::list_skills(),
        // Knowledge
        (Method::Get, "/api/knowledge") => return subsystems::list_knowledge(&state.data_dir),
        // Memory
        (Method::Get, "/api/memory") => return subsystems::list_memory(&state.data_dir),
        // Kanban static
        (Method::Get, "/api/kanban/tasks") => {
            return kanban::execute_tool("nde_kanban_get_tasks", None)
        }
        (Method::Post, "/api/kanban/tasks") => {
            return kanban::execute_tool("nde_kanban_create_task", Some(req))
        }
        // OpenViking
        (Method::Get, "/api/viking/status") => {
            return subsystems::viking_status(&state.rt, &state.viking)
        }
        (Method::Post, "/api/viking/install") => {
            return subsystems::viking_install(&state.rt, &state.viking)
        }
        (Method::Post, "/api/viking/start") => {
            return subsystems::viking_start(&state.rt, &state.viking)
        }
        (Method::Post, "/api/viking/stop") => {
            return subsystems::viking_stop(&state.rt, &state.viking)
        }
        // Actors (Shield Actor system)
        (Method::Get, "/api/actors") => return actors::list_actors(&state.data_dir),
        (Method::Post, "/api/actors/templates") => return actors::list_templates(),
        (Method::Post, "/api/actors/scaffold") => {
            return actors::scaffold_actor(req, &state.data_dir)
        }
        // FreeCut / Movie Dub
        (Method::Post, "/api/freecut/dub") => {
            return subsystems::freecut::handle_dub(req, &state.data_dir, &state.rt)
        }
        (Method::Post, "/api/freecut/dub/split") => {
            return subsystems::freecut::handle_split(req, &state.data_dir, &state.rt)
        }
        (Method::Post, "/api/freecut/dub/split-stream") => {
            return subsystems::freecut::handle_split_stream(req, &state.data_dir, &state.rt)
        }
        (Method::Post, "/api/freecut/dub/part") => {
            return subsystems::freecut::handle_dub_part(req, &state.data_dir, &state.rt)
        }
        (Method::Post, "/api/freecut/dub/merge") => {
            return subsystems::freecut::handle_merge(req, &state.data_dir, &state.rt)
        }
        (Method::Post, "/api/freecut/dub/all") => {
            return subsystems::freecut::handle_dub_all(req, &state.data_dir, &state.rt)
        }
        (Method::Post, "/api/freecut/dub/srt/read") => {
            return subsystems::freecut::handle_read_srt(req, &state.data_dir)
        }
        (Method::Post, "/api/freecut/dub/srt/save") => {
            return subsystems::freecut::handle_save_srt(req, &state.data_dir)
        }
        // ── FFmpeg Tools ────────────────────────────────────────────
        (Method::Post, "/api/freecut/ffmpeg/convert") => {
            return subsystems::freecut::handle_ffmpeg_convert(req, &state.data_dir, &state.rt)
        }
        (Method::Post, "/api/freecut/ffmpeg/extract-audio") => {
            return subsystems::freecut::handle_ffmpeg_extract_audio(req, &state.data_dir, &state.rt)
        }
        (Method::Post, "/api/freecut/ffmpeg/trim") => {
            return subsystems::freecut::handle_ffmpeg_trim(req, &state.data_dir, &state.rt)
        }
        (Method::Post, "/api/freecut/ffmpeg/compress") => {
            return subsystems::freecut::handle_ffmpeg_compress(req, &state.data_dir, &state.rt)
        }
        (Method::Post, "/api/freecut/ffmpeg/resize") => {
            return subsystems::freecut::handle_ffmpeg_resize(req, &state.data_dir, &state.rt)
        }
        (Method::Post, "/api/freecut/ffmpeg/remove-audio") => {
            return subsystems::freecut::handle_ffmpeg_remove_audio(req, &state.data_dir, &state.rt)
        }
        (Method::Post, "/api/freecut/ffmpeg/gif") => {
            return subsystems::freecut::handle_ffmpeg_gif(req, &state.data_dir, &state.rt)
        }
        (Method::Post, "/api/freecut/ffmpeg/info") => {
            return subsystems::freecut::handle_ffmpeg_info(req, &state.data_dir, &state.rt)
        }
        // ── Downloads ───────────────────────────────────────────────
        (Method::Get, "/api/downloads/providers") => {
            return subsystems::downloads::list_providers()
        }
        (Method::Post, "/api/downloads/resolve") => {
            return subsystems::downloads::resolve(req, &state.rt)
        }
        (Method::Post, "/api/downloads") => {
            return subsystems::downloads::start(req, &state.data_dir, &state.rt, &state.download_engine)
        }
        (Method::Get, "/api/downloads") => {
            return subsystems::downloads::list(&state.download_engine)
        }
        // ── KFA (Khmer Forced Aligner) ──────────────────────────────────────
        (Method::Post, "/api/kfa/align") => {
            return subsystems::kfa::handle_align_multipart(req, &state.data_dir)
        }
        (Method::Post, "/api/kfa/align-json") => {
            return subsystems::kfa::handle_align_json(req, &state.data_dir)
        }
        (Method::Post, "/api/kfa/align-srt") => {
            return subsystems::kfa::handle_align_srt_multipart(req, &state.data_dir)
        }
        (Method::Post, "/api/kfa/align-srt-json") => {
            return subsystems::kfa::handle_align_srt_json(req, &state.data_dir)
        }
        (Method::Post, "/api/kfa/transcribe") => {
            return subsystems::kfa::handle_transcribe_multipart(req, &state.data_dir)
        }
        (Method::Post, "/api/kfa/transcribe-json") => {
            return subsystems::kfa::handle_transcribe_json(req, &state.data_dir)
        }
        // ── Translate (SRT translation service) ─────────────────────────────
        (Method::Get, "/api/translate/providers") => {
            return subsystems::translate::handle_list_providers()
        }
        (Method::Post, "/api/translate/srt") => {
            return subsystems::translate::handle_translate_srt(req, &state.data_dir, &state.rt, &state.llm_manager)
        }
        (Method::Post, "/api/translate/srt-multipart") => {
            return subsystems::translate::handle_translate_srt_multipart(req, &state.data_dir, &state.rt, &state.llm_manager)
        }
        (Method::Post, "/api/translate/text") => {
            return subsystems::translate::handle_translate_text(req, &state.data_dir, &state.rt, &state.llm_manager)
        }
        // ── Whisper ───────────────────────────────────────────────────────────
        (Method::Post, "/api/transcript") => {
            return subsystems::whisper::handle_transcript(req, &state.data_dir, &state.rt)
        }
        _ => {}
    }

    // ── Dynamic routes (path params) ────────────────────────────────────────

    // FreeCut dub job status: /api/freecut/dub/job/{job_id}
    if path.starts_with("/api/freecut/dub/job/") {
        let job_id = path.trim_start_matches("/api/freecut/dub/job/");
        if !job_id.is_empty() {
            return subsystems::freecut::handle_job_status(job_id, &state.data_dir);
        }
    }

    // Agent task routes: /api/agent/tasks/{id}/...
    if path.starts_with("/api/agent/tasks/") {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        return match (method.clone(), parts.as_slice()) {
            (Method::Get, ["api", "agent", "tasks", id]) => {
                agent::stream::get_task(id, &state.agent_manager, &state.rt)
            }
            (Method::Get, ["api", "agent", "tasks", id, "stream"]) => {
                agent::stream::stream_task(id, &state.rt, &state.agent_manager)
            }
            (Method::Post, ["api", "agent", "tasks", id, "cancel"]) => {
                agent::stream::cancel_task(id, &state.rt, &state.agent_manager)
            }
            _ => err(404, &format!("Not found: {}", path)),
        };
    }

    // Agent conversations: /api/agent/conversations/{id}/messages
    if path.starts_with("/api/agent/conversations/") {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        return match (method.clone(), parts.as_slice()) {
            (Method::Get, ["api", "agent", "conversations", id, "messages"]) => {
                agent::handler::get_conversation_messages(id, &state.agent)
            }
            _ => err(404, &format!("Not found: {}", path)),
        };
    }

    // Plugin dynamic routes: /api/plugins/{id}/...
    if path.starts_with("/api/plugins/") {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        return match (method.clone(), parts.as_slice()) {
            (Method::Get, ["api", "plugins", id]) => plugins::get_plugin(id, &state.plugin_engine),
            (Method::Post, ["api", "plugins", id, "install"]) => {
                plugins::install_plugin(id, &state.rt, &state.plugin_engine)
            }
            (Method::Post, ["api", "plugins", id, "start"]) => {
                plugins::start_plugin(id, &state.rt, &state.plugin_engine)
            }
            (Method::Post, ["api", "plugins", id, "stop"]) => {
                plugins::stop_plugin(id, &state.rt, &state.plugin_engine)
            }
            (Method::Get, ["api", "plugins", id, "logs"]) => {
                plugins::get_plugin_logs(id, &state.plugin_engine)
            }
            (Method::Delete, ["api", "plugins", id, "logs"]) => {
                plugins::clear_plugin_logs(id, &state.plugin_engine)
            }
            _ => err(404, &format!("Not found: {}", path)),
        };
    }

    // Provider routes: /api/models/providers/{name}
    if path.starts_with("/api/models/providers/") {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        return match (method.clone(), parts.as_slice()) {
            (Method::Delete, ["api", "models", "providers", name]) => {
                models::remove_provider(name, &state.llm_manager)
            }
            _ => err(404, &format!("Not found: {}", path)),
        };
    }

    // Channel configure: /api/channels/{name}/configure
    if path.starts_with("/api/channels/") {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        return match (method.clone(), parts.as_slice()) {
            (Method::Post, ["api", "channels", _name, "configure"]) => {
                let resp = subsystems::configure_channel(req, &state.data_dir);

                // Check if Telegram should be started or stopped
                if let Some(tg_config) = gateway::TelegramGatewayConfig::load(&state.data_dir) {
                    gateway::start_telegram_gateway(
                        tg_config,
                        state.agent_manager.clone(),
                        state.llm_manager.clone(),
                        state.rt.handle().clone(),
                        state.tg_state.clone(),
                        state.log_buffer.clone(),
                        state.desktop_actions.clone(),
                    );
                } else {
                    state.tg_state.shutdown();
                }

                resp
            }
            _ => err(404, &format!("Not found: {}", path)),
        };
    }

    // Actors: /api/actors/{id}/...
    if path.starts_with("/api/actors/") {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        return match (method.clone(), parts.as_slice()) {
            (Method::Get, ["api", "actors", id]) => actors::get_actor(id, &state.data_dir),
            (Method::Delete, ["api", "actors", id]) => actors::delete_actor(id, &state.data_dir),
            (Method::Post, ["api", "actors", id, "run"]) => {
                actors::run_actor(id, req, &state.rt, &state.actor_runner)
            }
            (Method::Post, ["api", "actors", id, "stop"]) => {
                // id here is the run_id
                actors::stop_actor(id, &state.rt, &state.actor_runner)
            }
            (Method::Get, ["api", "actors", id, "runs"]) => {
                actors::list_runs(id, &state.rt, &state.actor_runner)
            }
            (Method::Get, ["api", "actors", id, "runs", run_id]) => {
                actors::get_run(id, run_id, &state.rt, &state.actor_runner)
            }
            (Method::Get, ["api", "actors", id, "runs", run_id, "dataset"]) => {
                actors::get_run_dataset(id, run_id, &state.data_dir)
            }
            (Method::Get, ["api", "actors", id, "runs", run_id, "log"]) => {
                actors::get_run_log(id, run_id, &state.data_dir)
            }
            (Method::Post, ["api", "actors", id, "export-apify"]) => {
                actors::export_apify(id, &state.data_dir)
            }
            _ => err(404, &format!("Not found: {}", path)),
        };
    }

    // Apps + sandbox: /api/apps/{id}/... and /api/sandbox/{id}/...
    if path.starts_with("/api/apps/") || path.starts_with("/api/sandbox/") {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        return match (method, parts.as_slice()) {
            (Method::Get, ["api", "apps", id]) => apps::get_app(id, &state.mgr),
            (Method::Delete, ["api", "apps", id]) => apps::uninstall_app(id, &state.mgr),
            (Method::Post, ["api", "apps", id, "launch"]) => apps::launch_app(id, &state.mgr),
            (Method::Post, ["api", "apps", id, "stop"]) => apps::stop_app(id, &state.mgr),
            (Method::Get, ["api", "sandbox", id, "verify"]) => apps::verify_sandbox(id, &state.mgr),
            (Method::Get, ["api", "sandbox", id, "disk"]) => apps::disk_usage(id, &state.mgr),
            _ => err(404, &format!("Not found: {}", path)),
        };
    }

    if path.starts_with("/api/store/") {
        return err(404, &format!("Not found: {}", path));
    }

    // Kanban dynamic routes: /api/kanban/tasks/{filename}/...
    if path.starts_with("/api/kanban/tasks/") {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        return match (method.clone(), parts.as_slice()) {
            (Method::Get, ["api", "kanban", "tasks", filename, "content"]) => {
                kanban::get_task_content(filename)
            }
            (Method::Put, ["api", "kanban", "tasks", filename, "content"]) => {
                kanban::update_task_content(filename, req)
            }
            (Method::Put, ["api", "kanban", "tasks", filename]) => {
                kanban::update_task(filename, req)
            }
            (Method::Delete, ["api", "kanban", "tasks", filename]) => kanban::delete_task(filename),
            _ => err(404, &format!("Not found: {}", path)),
        };
    }

    // Downloads dynamic routes: /api/downloads/{id}/...
    if path.starts_with("/api/downloads/") {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        return match (method.clone(), parts.as_slice()) {
            (Method::Get, ["api", "downloads", "providers"]) => {
                subsystems::downloads::list_providers()
            }
            (Method::Get, ["api", "downloads", id]) => {
                subsystems::downloads::get(id, &state.download_engine)
            }
            (Method::Post, ["api", "downloads", id, "pause"]) => {
                subsystems::downloads::pause(id, &state.rt, &state.download_engine)
            }
            (Method::Post, ["api", "downloads", id, "resume"]) => {
                subsystems::downloads::resume(id, &state.rt, &state.download_engine)
            }
            (Method::Post, ["api", "downloads", id, "cancel"]) => {
                subsystems::downloads::cancel(id, &state.download_engine)
            }
            (Method::Delete, ["api", "downloads", id]) => {
                subsystems::downloads::delete(id, &state.download_engine)
            }
            _ => err(404, &format!("Not found: {}", path)),
        };
    }

    // Knowledge search: /api/knowledge/search?q=...
    if path.starts_with("/api/knowledge/search") {
        let query = url.split("q=").nth(1).unwrap_or("");
        let decoded = urlencoding::decode(query).unwrap_or_default();
        return subsystems::search_knowledge(&decoded, &state.data_dir);
    }

    // Memory by key: /api/memory/{key}
    if path.starts_with("/api/memory/") {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        if let ["api", "memory", key] = parts.as_slice() {
            return subsystems::get_memory(key, &state.data_dir);
        }
    }

    err(404, &format!("Not found: {}", path))
}
