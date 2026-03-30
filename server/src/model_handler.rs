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
pub fn add_provider(
    req: &mut Request,
    manager: &Mutex<LlmManager>,
    rt: &tokio::runtime::Runtime,
) -> Response<Cursor<Vec<u8>>> {
    let body = match crate::response::read_body(req) {
        Some(b) => b,
        None => return err(400, "Missing request body"),
    };

    let config: ai_launcher_core::llm::manager::ProviderConfig = match serde_json::from_str(&body) {
        Ok(c) => c,
        Err(e) => return err(400, &format!("Invalid JSON: {}", e)),
    };

    let name = config.name.clone();

    // --- Verify provider is usable before adding ---
    let verify_res = rt.block_on(async {
        tracing::info!(
            "Verifying provider '{}' (type={})...",
            config.name,
            config.provider_type
        );
        ai_launcher_core::llm::verify_provider_config(&config).await
    });

    if let Err(e) = verify_res {
        return err(400, &format!("Verification failed: {}", e));
    }

    match manager.lock() {
        Ok(mut m) => match m.add_from_config(config) {
            Ok(()) => ok(&format!("Provider '{}' verified and added", name), name),
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

/// POST /api/codex/oauth/start — sign in with ChatGPT.
/// If already authenticated (Codex CLI or previous flow), auto-adds provider.
/// Otherwise starts a built-in PKCE OAuth flow.
#[derive(serde::Deserialize)]
struct CodexAuthReq {
    model: Option<String>,
}

pub fn codex_oauth_start(
    req: &mut Request,
    manager: std::sync::Arc<Mutex<LlmManager>>,
    rt: &tokio::runtime::Runtime,
    _data_dir: &std::path::Path,
) -> Response<Cursor<Vec<u8>>> {
    use ai_launcher_core::llm::codex_oauth;

    let mut model_name = "gpt-4o-mini".to_string();
    if let Some(body) = crate::response::read_body(req) {
        if let Ok(req_body) = serde_json::from_str::<CodexAuthReq>(&body) {
            if let Some(m) = req_body.model {
                model_name = m;
            }
        }
    }

    // Check if already authenticated
    if codex_oauth::get_codex_access_token().is_ok() {
        // Already logged in — auto-add the codex_oauth provider
        add_codex_provider(&manager, &model_name);

        #[derive(serde::Serialize)]
        struct StartResp {
            auth_url: String,
            already_authenticated: bool,
        }

        return ok(
            "Already authenticated — provider added",
            StartResp {
                auth_url: String::new(),
                already_authenticated: true,
            },
        );
    }

    // Not authenticated — start built-in PKCE OAuth flow
    let flow = codex_oauth::start_oauth_flow();
    let auth_url = flow.auth_url.clone();

    // Open browser for the user
    let _ = open_browser(&auth_url);

    // Spawn callback server + token exchange in background
    let manager_clone = manager.clone();

    rt.spawn(async move {
        match codex_oauth::complete_oauth_flow(&flow).await {
            Ok(auth) => {
                let email = auth
                    .tokens
                    .as_ref()
                    .and_then(|t| t.id_token.as_deref())
                    .and_then(|t| {
                        // Extract email for logging
                        let parts: Vec<&str> = t.split('.').collect();
                        if parts.len() < 2 {
                            return None;
                        }
                        Some(parts[1].to_string())
                    });
                tracing::info!(?email, "OAuth flow completed, adding provider");
                add_codex_provider(&manager_clone, &model_name);
            }
            Err(e) => {
                tracing::error!("OAuth flow failed: {}", e);
            }
        }
    });

    #[derive(serde::Serialize)]
    struct StartResp {
        auth_url: String,
        already_authenticated: bool,
    }

    ok(
        "OAuth flow started — complete login in your browser",
        StartResp {
            auth_url,
            already_authenticated: false,
        },
    )
}

/// Helper: add/replace the codex-chatgpt provider and make it active.
fn add_codex_provider(manager: &Mutex<LlmManager>, model: &str) {
    let config = ai_launcher_core::llm::manager::ProviderConfig {
        name: "codex-chatgpt".into(),
        provider_type: "codex_oauth".into(),
        model: model.into(),
        base_url: None,
        api_key: None,
        api_key_env: None,
        max_tokens: 16384,
    };
    if let Ok(mut m) = manager.lock() {
        m.remove("codex-chatgpt");
        if let Err(e) = m.add_from_config(config) {
            tracing::error!("Failed to add codex_oauth provider: {}", e);
        } else {
            let _ = m.switch("codex-chatgpt");
            tracing::info!("Codex OAuth provider added and activated");
        }
    }
}

/// GET /api/codex/oauth/status — check auth status.
pub fn codex_oauth_status(data_dir: &std::path::Path) -> Response<Cursor<Vec<u8>>> {
    use ai_launcher_core::llm::codex_oauth::CodexOAuthStatus;
    let status = CodexOAuthStatus::from_store(data_dir);
    ok("Codex OAuth status", status)
}

/// Open a URL in the user's default browser.
fn open_browser(url: &str) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!("Start-Process '{}'", url),
            ])
            .spawn();
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

/// GET /api/models/recommendations — get recommended GGUF models based on system RAM.
pub fn recommend_gguf_models(
    mgr: &ai_launcher_core::app_manager::AppManager,
) -> Response<Cursor<Vec<u8>>> {
    use ai_launcher_core::llm::gguf::GgufModelRecommendation;
    use ai_launcher_core::system_metrics::snapshot_resource_usage;

    let ram_bytes = match snapshot_resource_usage(mgr.base_dir()) {
        Ok(usage) => usage.memory_total_bytes,
        Err(_) => 0, // Default to 0, which returns TinyLlama
    };

    let models = GgufModelRecommendation::recommend_models(ram_bytes, None);
    ok("Recommended GGUF models", models)
}

/// GET /api/models/local — list locally-available GGUF model files.
pub fn list_local_models() -> Response<Cursor<Vec<u8>>> {
    use ai_launcher_core::llm::gguf::GgufProvider;

    let data_dir = std::env::var("AI_LAUNCHER_DATA_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            if cfg!(windows) {
                std::env::var("LOCALAPPDATA")
                    .map(|p| std::path::PathBuf::from(p).join("ai-launcher"))
                    .unwrap_or_else(|_| std::path::PathBuf::from("C:\\ai-launcher-data"))
            } else {
                std::env::var("HOME")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
                    .join(".ai-launcher")
            }
        });

    let models = GgufProvider::list_local_models(&data_dir);
    ok("Local GGUF models", models)
}

/// POST /api/models/verify — verify a provider config can run without errors.
/// Tests the actual connection (API key, model availability, server reachability)
/// without saving the provider.
pub fn verify_provider(
    req: &mut Request,
    rt: &tokio::runtime::Runtime,
) -> Response<Cursor<Vec<u8>>> {
    let body = match crate::response::read_body(req) {
        Some(b) => b,
        None => return err(400, "Missing request body"),
    };

    let config: ai_launcher_core::llm::manager::ProviderConfig = match serde_json::from_str(&body) {
        Ok(c) => c,
        Err(e) => return err(400, &format!("Invalid JSON: {}", e)),
    };

    #[derive(serde::Serialize)]
    struct VerifyResult {
        ok: bool,
        error: Option<String>,
        model_exists: bool,
        server_available: bool,
    }

    // For GGUF, use the detailed GGUF verify which checks model file + server binary
    if matches!(
        config.provider_type.as_str(),
        "gguf" | "llama-cpp" | "llama.cpp"
    ) {
        let result = rt.block_on(async {
            let data_dir = std::env::var("AI_LAUNCHER_DATA_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| {
                    if cfg!(windows) {
                        std::env::var("LOCALAPPDATA")
                            .map(|p| std::path::PathBuf::from(p).join("ai-launcher"))
                            .unwrap_or_else(|_| std::path::PathBuf::from("C:\\ai-launcher-data"))
                    } else {
                        std::env::var("HOME")
                            .map(std::path::PathBuf::from)
                            .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
                            .join(".ai-launcher")
                    }
                });
            let provider = ai_launcher_core::llm::gguf::GgufProvider::new(
                &data_dir,
                &config.model,
                config.base_url.as_deref(),
                None,
            );
            provider.verify().await
        });
        return ok("GGUF verification result", result);
    }

    // For all other providers (Ollama, OpenAI, Anthropic, Groq, etc.),
    // use the same verify_provider_config that add_provider uses.
    // This actually tests the connection: pings Ollama, sends a test message to APIs, etc.
    let verify_res = rt.block_on(async {
        tracing::info!(
            "Verify-only test for '{}' (type={})...",
            config.name,
            config.provider_type
        );
        ai_launcher_core::llm::verify_provider_config(&config).await
    });

    match verify_res {
        Ok(()) => ok(
            "Provider verification passed",
            VerifyResult {
                ok: true,
                error: None,
                model_exists: true,
                server_available: true,
            },
        ),
        Err(e) => ok(
            "Provider verification failed",
            VerifyResult {
                ok: false,
                error: Some(e.to_string()),
                model_exists: false,
                server_available: false,
            },
        ),
    }
}
