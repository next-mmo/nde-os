/// LLM model management handlers.
use ai_launcher_core::llm::manager::LlmManager;
use std::io::Cursor;
use std::sync::Mutex;
use tiny_http::{Request, Response};

use crate::response::*;

/// GET /api/models — list providers.
pub fn list_models(manager: &Mutex<LlmManager>) -> Response<Cursor<Vec<u8>>> {
    match manager.lock() {
        Ok(m) => ok("LLM providers", m.status()),
        Err(_) => err(500, "LLM manager lock failed"),
    }
}

/// GET /api/models/active — current active model.
pub fn active_model(manager: &Mutex<LlmManager>) -> Response<Cursor<Vec<u8>>> {
    match manager.lock() {
        Ok(m) => ok("Active provider", m.active_name()),
        Err(_) => err(500, "LLM manager lock failed"),
    }
}

/// POST /api/models/switch — switch active provider.
pub fn switch_model(req: &mut Request, manager: &Mutex<LlmManager>) -> Response<Cursor<Vec<u8>>> {
    let body = match read_body(req) {
        Some(b) => b,
        None => return err(400, "Missing request body"),
    };

    #[derive(serde::Deserialize)]
    struct SwitchReq {
        name: String,
    }

    let switch: SwitchReq = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => return err(400, &format!("Invalid JSON: {}", e)),
    };

    match manager.lock() {
        Ok(mut m) => match m.switch(&switch.name) {
            Ok(()) => ok(&format!("Switched to '{}'", switch.name), switch.name),
            Err(e) => err(400, &e.to_string()),
        },
        Err(_) => err(500, "LLM manager lock failed"),
    }
}

/// POST /api/models/providers — add a new provider.
pub fn add_provider(req: &mut Request, manager: &Mutex<LlmManager>) -> Response<Cursor<Vec<u8>>> {
    let body = match read_body(req) {
        Some(b) => b,
        None => return err(400, "Missing request body"),
    };

    let config: ai_launcher_core::llm::manager::ProviderConfig = match serde_json::from_str(&body) {
        Ok(c) => c,
        Err(e) => return err(400, &format!("Invalid JSON: {}", e)),
    };

    let name = config.name.clone();
    match manager.lock() {
        Ok(mut m) => match m.add_from_config(config) {
            Ok(()) => ok(&format!("Added provider '{}'", name), name),
            Err(e) => err(400, &e.to_string()),
        },
        Err(_) => err(500, "LLM manager lock failed"),
    }
}

/// DELETE /api/models/providers/{name} — remove a provider.
pub fn remove_provider(name: &str, manager: &Mutex<LlmManager>) -> Response<Cursor<Vec<u8>>> {
    match manager.lock() {
        Ok(mut m) => {
            if m.remove(name) {
                ok(&format!("Removed provider '{}'", name), name)
            } else {
                err(404, &format!("Provider '{}' not found", name))
            }
        }
        Err(_) => err(500, "LLM manager lock failed"),
    }
}

/// POST /api/codex/oauth/start — initiate Codex OAuth flow.
pub fn codex_oauth_start(
    manager: &Mutex<LlmManager>,
    rt: &tokio::runtime::Runtime,
    data_dir: &std::path::Path,
) -> Response<Cursor<Vec<u8>>> {
    use ai_launcher_core::llm::codex_oauth;

    let flow = codex_oauth::start_oauth_flow();
    let auth_url = flow.auth_url.clone();

    // Spawn the callback server and token exchange in the background
    let data_dir_owned = data_dir.to_path_buf();
    let manager_clone = manager as *const Mutex<LlmManager>;
    let flow_clone = flow.clone();

    // SAFETY: the server runs in a single-threaded loop and manager outlives this task.
    let manager_ref = unsafe { &*manager_clone };

    rt.spawn(async move {
        match codex_oauth::complete_oauth_flow(&flow_clone, &data_dir_owned).await {
            Ok(store) => {
                tracing::info!(email = ?store.email, "Codex OAuth flow completed, adding provider");
                // Auto-add the codex_oauth provider
                let config = ai_launcher_core::llm::manager::ProviderConfig {
                    name: "codex-chatgpt".into(),
                    provider_type: "codex_oauth".into(),
                    model: "codex-mini-latest".into(),
                    base_url: None,
                    api_key: None,
                    api_key_env: None,
                    max_tokens: 16384,
                };
                if let Ok(mut m) = manager_ref.lock() {
                    // Remove existing codex-chatgpt if present
                    m.remove("codex-chatgpt");
                    if let Err(e) = m.add_from_config(config) {
                        tracing::error!("Failed to add codex_oauth provider: {}", e);
                    } else {
                        let _ = m.switch("codex-chatgpt");
                        tracing::info!("Codex OAuth provider added and activated");
                    }
                }
            }
            Err(e) => {
                tracing::error!("Codex OAuth flow failed: {}", e);
            }
        }
    });

    // Open user's browser
    let _ = open_browser(&auth_url);

    #[derive(serde::Serialize)]
    struct StartResp {
        auth_url: String,
    }

    ok("OAuth flow started — complete login in your browser", StartResp { auth_url })
}

/// GET /api/codex/oauth/status — check Codex OAuth auth status.
pub fn codex_oauth_status(data_dir: &std::path::Path) -> Response<Cursor<Vec<u8>>> {
    use ai_launcher_core::llm::codex_oauth::CodexOAuthStatus;
    let status = CodexOAuthStatus::from_store(data_dir);
    ok("Codex OAuth status", status)
}

/// Open a URL in the user's default browser.
fn open_browser(url: &str) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd").args(["/C", "start", "", url]).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}
