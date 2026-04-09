/// Shared gateway command handlers.
///
/// These functions are transport-agnostic and can be reused across
/// Telegram, Discord, Slack, or other chat gateways.
use ai_launcher_core::agent::manager::AgentManager;
use ai_launcher_core::agent::protocol::AgentEvent;
use ai_launcher_core::llm::manager::LlmManager;
use ai_launcher_core::shield::browser::BrowserEngine;
use ai_launcher_core::shield::emulator::EmulatorManager;
use ai_launcher_core::shield::profile::{ProfileManager, ShieldProfile};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::router::{DesktopAction, DesktopActionQueue};

/// Canonical list of all static app IDs known to the desktop shell.
/// Keep in sync with `desktop/src/configs/apps/apps-config.ts`.
pub(crate) const STATIC_APP_IDS: &[(&str, &str)] = &[
    ("ai-launcher", "AI Launcher"),
    ("browser", "Browser"),
    ("logs", "Logs"),
    ("settings", "Settings"),
    ("chat", "NDE Chat"),
    ("app-store", "App Store"),
    ("terminal", "Terminal"),
    ("code-editor", "Code Editor"),
    ("command-center", "Command Center"),
    ("model-settings", "LLM Providers"),
    ("plugins", "Plugins"),
    ("channels", "Channels"),
    ("mcp-tools", "MCP Tools"),
    ("skills", "Skills"),
    ("knowledge", "Knowledge"),
    ("architecture", "Architecture"),
    ("shield-browser", "Shield Browser"),
    ("file-explorer", "File Explorer"),
    ("vibe-studio", "Vibe Code Studio"),
    ("screenshot", "Screenshot Results"),
    ("service-hub", "Service Hub"),
    ("freecut", "FreeCut"),
];

/// Result of an emulator command.
pub(crate) enum EmulatorAction {
    Reply(String),
    SendScreenshot { path: PathBuf, caption: String },
}

/// Returns the welcome message shown on `/start`.
pub(crate) fn welcome_message() -> String {
    "👋 Welcome to NDE-OS Agent!\n\n\
    📋 Kanban:\n\
    /todo_list — List all tasks\n\
    /todo_add <title> — Create task\n\
    /todo_done <file> — Mark done\n\n\
    🖥️ Desktop Apps:\n\
    /apps — List all desktop apps\n\
    /app:<id> — Open an app (e.g. /app:vibe-studio)\n\n\
    🛡️ Shield Browser:\n\
    /profiles — List browser profiles\n\
    /profile <name> — Profile details\n\
    /profile_create <name> — New profile\n\n\
    🧠 LLM Models:\n\
    /models — List all providers\n\
    /model — Show active model\n\
    /model <name> — Change model (e.g. gpt-4o)\n\
    /model_switch <name> — Switch provider\n\n\
    🔍 Research:\n\
    /research <topic> — AI web research\n\
    /research_shield <topic> — Research via anti-detect browser\n\n\
    /help — All commands\n\
    Or just type any message → AI agent."
        .into()
}

/// Returns the full help text shown on `/help`.
pub(crate) fn help_message() -> String {
    "🤖 NDE-OS Agent Commands:\n\n\
    📋 Kanban:\n\
    /todo_list — List all tasks\n\
    /todo_add <title> — Create task\n\
    /todo_done <file> — Mark done\n\n\
    🖥️ Desktop Apps:\n\
    /apps — List all desktop apps\n\
    /app:<id> — Open an app (e.g. /app:vibe-studio)\n\n\
    🛡️ Shield Browser:\n\
    /profiles — List all browser profiles\n\
    /profile <name> — Show profile details\n\
    /profile_create <name> — Create new profile\n\n\
    🧠 LLM Models:\n\
    /models — List all providers\n\
    /model — Show active model\n\
    /model <name> — Change model (e.g. gpt-4o)\n\
    /model_switch <name> — Switch provider\n\n\
    🔍 Research:\n\
    /research <topic> — AI-powered web research\n\
    /research_shield <topic> — Research via Shield Browser (anti-detect)\n\n\
    💬 Any other message will be processed by the AI agent with 30+ tools \
    (file I/O, shell, web search, git, and more)."
        .into()
}

/// Handle kanban slash commands directly without the LLM.
pub(crate) fn try_kanban(text: &str) -> Option<String> {
    let t = text.trim();

    if t == "/start" {
        return Some(welcome_message());
    }
    if t == "/help" {
        return Some(help_message());
    }

    if t == "/todo_list" || t.starts_with("/todo_list ") {
        match ai_launcher_core::mcp::kanban::execute("nde_kanban_get_tasks", &serde_json::json!({}))
        {
            Ok(result) => {
                if let Ok(tasks) = serde_json::from_str::<Vec<serde_json::Value>>(&result) {
                    if tasks.is_empty() {
                        return Some(
                            "📋 No tasks found.\nUse /todo_add <title> to create one.".into(),
                        );
                    }

                    let mut lines = vec![format!("📋 Kanban Board ({} tasks)\n", tasks.len())];
                    for task in &tasks {
                        let status = task
                            .get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Plan");
                        let title = task
                            .get("title")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Untitled");
                        let filename = task.get("filename").and_then(|v| v.as_str()).unwrap_or("");
                        let emoji = match status {
                            "Plan" => "🔴",
                            "YOLO mode" => "🟡",
                            "Done by AI" => "🟢",
                            "Verified" => "✅",
                            "Re-open" => "🔴",
                            "Waiting Approval" => "🟠",
                            _ => "⚪",
                        };
                        lines.push(format!("{} {} — {} ({})", emoji, title, status, filename));
                    }
                    return Some(lines.join("\n"));
                }
                Some(result)
            }
            Err(e) => Some(format!("❌ Failed to list tasks: {}", e)),
        }
    } else if t.starts_with("/todo_add ") {
        let title = t["/todo_add ".len()..].trim();
        if title.is_empty() {
            return Some("❌ Usage: /todo_add <task title>".into());
        }

        let params = serde_json::json!({ "title": title });
        match ai_launcher_core::mcp::kanban::execute("nde_kanban_create_task", &params) {
            Ok(result) => {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&result) {
                    let fname = data.get("filename").and_then(|v| v.as_str()).unwrap_or("");
                    Some(format!("✅ Task created: {}\n📄 File: {}", title, fname))
                } else {
                    Some(format!("✅ Task created: {}", title))
                }
            }
            Err(e) => Some(format!("❌ Failed to create task: {}", e)),
        }
    } else if t.starts_with("/todo_done ") {
        let filename = t["/todo_done ".len()..].trim();
        if filename.is_empty() {
            return Some("❌ Usage: /todo_done <filename.md>".into());
        }

        let fname = if filename.ends_with(".md") {
            filename.to_string()
        } else {
            format!("{}.md", filename)
        };
        let params = serde_json::json!({ "filename": fname, "status": "Done by AI" });
        match ai_launcher_core::mcp::kanban::execute("nde_kanban_update_task", &params) {
            Ok(_) => Some(format!("✔️ Task marked as done: {}", fname)),
            Err(e) => Some(format!("❌ Failed to update task: {}", e)),
        }
    } else {
        None
    }
}

/// Handle desktop app commands: `/apps` and `/app:<id>`.
pub(crate) fn try_desktop_commands(
    text: &str,
    desktop_actions: &DesktopActionQueue,
) -> Option<String> {
    let t = text.trim();

    // /apps — list all available desktop apps
    if t == "/apps" {
        let mut lines = vec![format!("🖥️ Desktop Apps ({})\n", STATIC_APP_IDS.len())];
        for (id, title) in STATIC_APP_IDS {
            lines.push(format!("  • {} — /app:{}", title, id));
        }
        lines.push(String::new());
        lines.push("Tap a command or type /app:<id> to open.".into());
        return Some(lines.join("\n"));
    }

    // /app:<id> — open a specific app
    if let Some(app_id) = t.strip_prefix("/app:") {
        let app_id = app_id.trim();
        if app_id.is_empty() {
            return Some("❌ Usage: /app:<id>\nExample: /app:vibe-studio\nUse /apps to see all available apps.".into());
        }

        // Validate against known app IDs (case-insensitive)
        let lower = app_id.to_lowercase();
        let matched = STATIC_APP_IDS
            .iter()
            .find(|(id, _)| id.to_lowercase() == lower);

        return Some(match matched {
            Some((canonical_id, title)) => {
                // Push action to the queue for the frontend to pick up
                if let Ok(mut q) = desktop_actions.lock() {
                    q.push(DesktopAction {
                        kind: "open_app".to_string(),
                        app_id: canonical_id.to_string(),
                    });
                }
                format!("✅ Opening {} on desktop…", title)
            }
            None => {
                // Try fuzzy match
                let suggestions: Vec<String> = STATIC_APP_IDS
                    .iter()
                    .filter(|(id, title)| {
                        id.contains(&lower) || title.to_lowercase().contains(&lower)
                    })
                    .map(|(id, title)| format!("  • {} — /app:{}", title, id))
                    .collect();

                if suggestions.is_empty() {
                    format!(
                        "❌ Unknown app '{}'. Use /apps to see all available apps.",
                        app_id
                    )
                } else {
                    format!(
                        "❌ Unknown app '{}'. Did you mean:\n{}\n\nUse /apps for the full list.",
                        app_id,
                        suggestions.join("\n")
                    )
                }
            }
        });
    }

    None
}

/// Handle Shield Browser commands.
pub(crate) fn try_shield(text: &str, data_dir: &Path) -> Option<String> {
    let t = text.trim();

    if t == "/profiles" {
        let pmgr = ProfileManager::new(data_dir);
        return Some(match pmgr.list_profiles() {
            Ok(profiles) => {
                if profiles.is_empty() {
                    "🛡️ No Shield Browser profiles found.\n\
                    Use /profile_create <name> to create one."
                        .into()
                } else {
                    let mut lines =
                        vec![format!("🛡️ Shield Browser Profiles ({})\n", profiles.len())];
                    for p in &profiles {
                        let icon = if p.engine == BrowserEngine::Camoufox {
                            "🦊"
                        } else {
                            "🌐"
                        };
                        let status = if p.is_running() {
                            "🟢 Running"
                        } else {
                            "⚪ Idle"
                        };
                        let proxy = if p.proxy.is_some() { " 🔒" } else { "" };
                        lines.push(format!(
                            "{} {} — {} v{} [{}]{}",
                            icon,
                            p.name,
                            p.engine.display_name(),
                            p.engine_version,
                            status,
                            proxy,
                        ));
                        lines.push(format!("   ID: {}", p.id));
                    }
                    lines.join("\n")
                }
            }
            Err(e) => format!("❌ Failed to list profiles: {}", e),
        });
    }

    if t.starts_with("/profile ") && !t.starts_with("/profile_") {
        let query = t["/profile ".len()..].trim();
        if query.is_empty() {
            return Some("❌ Usage: /profile <name or id>".into());
        }

        let pmgr = ProfileManager::new(data_dir);
        return Some(match find_profile_by_query(&pmgr, query) {
            Ok(p) => {
                let icon = if p.engine == BrowserEngine::Camoufox {
                    "🦊"
                } else {
                    "🌐"
                };
                let status = if p.is_running() {
                    "🟢 Running"
                } else {
                    "⚪ Idle"
                };
                let proxy_str = match &p.proxy {
                    Some(px) => format!("🔒 {}:{}", px.host, px.port),
                    None => "⚠️ None".into(),
                };
                let fp_os = p.fingerprint.os.as_deref().unwrap_or("Auto");
                let tags = if p.tags.is_empty() {
                    "None".to_string()
                } else {
                    p.tags.join(", ")
                };
                let created = format_epoch(p.created_at);
                let last_launch = p
                    .last_launch
                    .map(format_epoch)
                    .unwrap_or_else(|| "Never".into());

                format!(
                    "{} {} — {}\n\n\
                    ID: {}\n\
                    Engine: {} v{}\n\
                    Status: {}\n\
                    Proxy: {}\n\
                    Fingerprint OS: {}\n\
                    Tags: {}\n\
                    Created: {}\n\
                    Last Launch: {}",
                    icon,
                    p.name,
                    status,
                    p.id,
                    p.engine.display_name(),
                    p.engine_version,
                    status,
                    proxy_str,
                    fp_os,
                    tags,
                    created,
                    last_launch,
                )
            }
            Err(msg) => msg,
        });
    }

    if t.starts_with("/profile_create ") {
        let name = t["/profile_create ".len()..].trim();
        if name.is_empty() {
            return Some(
                "❌ Usage: /profile_create <name>\nExample: /profile_create US Business".into(),
            );
        }

        let pmgr = ProfileManager::new(data_dir);
        let profile = ShieldProfile::new(
            name.to_string(),
            BrowserEngine::Camoufox,
            "latest".to_string(),
        );
        return Some(match pmgr.create_profile(&profile) {
            Ok(()) => format!(
                "✅ Profile created: {}\n\
                🆔 ID: {}\n\
                🦊 Engine: Camoufox\n\n\
                Launch it from the Shield Browser desktop app, \
                or use /profile {} to view details.",
                name, profile.id, name
            ),
            Err(e) => format!("❌ Failed to create profile: {}", e),
        });
    }

    None
}

/// Handle LLM commands and indicate whether the active model/provider changed.
pub(crate) fn try_llm(text: &str, llm_manager: &Arc<Mutex<LlmManager>>) -> Option<(String, bool)> {
    let t = text.trim();

    if t == "/models" {
        let mgr = match llm_manager.lock() {
            Ok(m) => m,
            Err(_) => return Some(("❌ LLM manager lock failed".into(), false)),
        };
        let providers = mgr.status();
        if providers.is_empty() {
            return Some((
                "🧠 No LLM providers configured.\n\
                Add one from Settings → Models in the desktop app."
                    .into(),
                false,
            ));
        }

        let mut lines = vec![format!("🧠 LLM Providers ({})\n", providers.len())];
        for p in &providers {
            let active = if p.is_active { " ✅ active" } else { "" };
            lines.push(format!("  • {} ({}){}", p.name, p.provider_type, active));
        }
        lines.push(String::new());
        lines.push("Use /model_switch <name> to change.".into());
        return Some((lines.join("\n"), false));
    }

    if t == "/model" {
        let mgr = match llm_manager.lock() {
            Ok(m) => m,
            Err(_) => return Some(("❌ LLM manager lock failed".into(), false)),
        };
        let active = mgr.active_name();
        if active.is_empty() {
            return Some((
                "🧠 No active LLM provider. Use /models to see available ones.".into(),
                false,
            ));
        }

        let detail = mgr.configs().iter().find(|c| c.name == active);
        return Some((
            match detail {
                Some(cfg) => format!(
                    "🧠 Active LLM Provider\n\n\
                    Name: {}\n\
                    Type: {}\n\
                    Model: {}\n\
                    Max tokens: {}\n\n\
                    Use /model <name> to change the model.",
                    cfg.name, cfg.provider_type, cfg.model, cfg.max_tokens,
                ),
                None => format!("🧠 Active: {}", active),
            },
            false,
        ));
    }

    if t.starts_with("/model ") && !t.starts_with("/model_") && !t.starts_with("/models") {
        let new_model = t["/model ".len()..].trim();
        if new_model.is_empty() {
            return Some((
                "❌ Usage: /model <model_name>\nExample: /model gpt-4o".into(),
                false,
            ));
        }

        let mut mgr = match llm_manager.lock() {
            Ok(m) => m,
            Err(_) => return Some(("❌ LLM manager lock failed".into(), false)),
        };
        let active = mgr.active_name().to_string();
        if active.is_empty() {
            return Some((
                "🧠 No active provider. Add one first via Settings → Models.".into(),
                false,
            ));
        }

        return Some(match mgr.update_active_model(new_model) {
            Ok(()) => (
                format!("✅ Model changed to: {}\nProvider: {}", new_model, active),
                true,
            ),
            Err(e) => (format!("❌ Failed to change model: {}", e), false),
        });
    }

    if t.starts_with("/model_switch ") {
        let name = t["/model_switch ".len()..].trim();
        if name.is_empty() {
            return Some((
                "❌ Usage: /model_switch <provider_name>\n\
                Use /models to see available providers."
                    .into(),
                false,
            ));
        }

        let mut mgr = match llm_manager.lock() {
            Ok(m) => m,
            Err(_) => return Some(("❌ LLM manager lock failed".into(), false)),
        };

        let target = {
            let names = mgr.provider_names();
            if names.iter().any(|n| n == name) {
                Some(name.to_string())
            } else {
                let lower = name.to_lowercase();
                names.into_iter().find(|n| n.to_lowercase() == lower)
            }
        };

        return Some(match target {
            Some(target_name) => match mgr.switch(&target_name) {
                Ok(()) => (format!("✅ Switched active LLM to: {}", target_name), true),
                Err(e) => (format!("❌ Failed to switch: {}", e), false),
            },
            None => {
                let available: Vec<String> = mgr.provider_names();
                (
                    format!(
                        "❌ Provider '{}' not found.\n\nAvailable: {}",
                        name,
                        if available.is_empty() {
                            "(none configured)".to_string()
                        } else {
                            available.join(", ")
                        }
                    ),
                    false,
                )
            }
        });
    }

    None
}

/// Sync the AgentManager provider from the currently active LLM config.
pub(crate) async fn sync_agent_provider_from_llm(
    agent_manager: &Arc<tokio::sync::Mutex<AgentManager>>,
    llm_manager: &Arc<Mutex<LlmManager>>,
) {
    let (provider_type, model, base_url, api_key) = {
        let mgr = match llm_manager.lock() {
            Ok(m) => m,
            Err(_) => {
                tracing::warn!("Failed to lock LlmManager for agent sync");
                return;
            }
        };
        let active = mgr.active_name().to_string();
        match mgr.configs().iter().find(|c| c.name == active) {
            Some(cfg) => (
                cfg.provider_type.clone(),
                cfg.model.clone(),
                cfg.base_url.clone(),
                cfg.api_key.clone().or_else(|| {
                    cfg.api_key_env
                        .as_ref()
                        .and_then(|env_name| std::env::var(env_name).ok())
                }),
            ),
            None => return,
        }
    };

    let mut mgr = agent_manager.lock().await;
    let mut config = mgr.agent_config().clone();
    config.model_provider = provider_type;
    config.model_name = model.clone();
    config.base_url = base_url;
    config.api_key = api_key;

    match mgr.update_provider(&config) {
        Ok(()) => tracing::info!(model = %model, "Gateway: AgentManager provider synced"),
        Err(e) => tracing::warn!(error = %e, "Gateway: Failed to sync AgentManager provider"),
    }
}

/// Handle Android Emulator commands.
pub(crate) fn try_emulator(text: &str, data_dir: &Path) -> Option<EmulatorAction> {
    let t = text.trim();

    if !t.starts_with("/emulator") {
        return None;
    }

    let emu_mgr = match EmulatorManager::new(data_dir) {
        Ok(mgr) => mgr,
        Err(e) => {
            return Some(EmulatorAction::Reply(format!(
                "❌ Emulator subsystem offline: {}",
                e
            )))
        }
    };

    if t == "/emulators" {
        let mut msg = String::new();

        match emu_mgr.list_devices() {
            Ok(devices) => {
                let running: Vec<_> = devices.iter().filter(|d| d.is_emulator()).collect();
                msg.push_str(&format!("📱 Running Devices ({})\n", running.len()));
                for d in running {
                    let status = match d.status {
                        ai_launcher_core::shield::emulator::DeviceStatus::Online => "🟢 Online",
                        ai_launcher_core::shield::emulator::DeviceStatus::Offline => "⚪ Offline",
                        ai_launcher_core::shield::emulator::DeviceStatus::Booting => "🟡 Booting",
                        ai_launcher_core::shield::emulator::DeviceStatus::Unauthorized => {
                            "⛔ Unauthorized"
                        }
                    };
                    msg.push_str(&format!("  • {} [{}]\n", d.display_name(), status));
                }
            }
            Err(e) => msg.push_str(&format!("❌ Failed to list devices: {}\n", e)),
        }

        msg.push('\n');

        match emu_mgr.list_avds() {
            Ok(avds) => {
                msg.push_str(&format!("📦 Available AVDs ({})\n", avds.len()));
                for a in avds {
                    msg.push_str(&format!("  • {}\n", a.name));
                }
            }
            Err(e) => msg.push_str(&format!("❌ Failed to list AVDs: {}\n", e)),
        }

        if msg.trim().is_empty() {
            msg = "No Android SDK components detected.".into();
        }

        return Some(EmulatorAction::Reply(msg));
    }

    if t.starts_with("/emulator_launch ") {
        let avd = t["/emulator_launch ".len()..].trim();
        if avd.is_empty() {
            return Some(EmulatorAction::Reply(
                "❌ Usage: /emulator_launch <avd_name>".into(),
            ));
        }

        return Some(EmulatorAction::Reply(match emu_mgr.launch_avd(avd) {
            Ok(()) => format!("✅ Launching AVD '{}'... This may take a minute.", avd),
            Err(e) => format!("❌ Failed to launch emulator: {}", e),
        }));
    }

    if t.starts_with("/emulator_stop ") {
        let serial = t["/emulator_stop ".len()..].trim();
        if serial.is_empty() {
            return Some(EmulatorAction::Reply(
                "❌ Usage: /emulator_stop <serial>\nExample: /emulator_stop emulator-5554".into(),
            ));
        }

        return Some(EmulatorAction::Reply(match emu_mgr.stop_device(serial) {
            Ok(()) => format!("✅ Stopped device '{}'", serial),
            Err(e) => format!("❌ Failed to stop device: {}", e),
        }));
    }

    if t.starts_with("/emulator_open ") {
        let parts: Vec<&str> = t["/emulator_open ".len()..].splitn(2, ' ').collect();
        if parts.len() < 2 {
            return Some(EmulatorAction::Reply(
                "❌ Usage: /emulator_open <serial> <url>".into(),
            ));
        }

        let serial = parts[0].trim();
        let url = parts[1].trim();
        return Some(EmulatorAction::Reply(match emu_mgr.open_url(serial, url) {
            Ok(()) => format!("✅ Opened URL on '{}'", serial),
            Err(e) => format!("❌ Failed to open URL: {}", e),
        }));
    }

    if t.starts_with("/emulator_screenshot ") {
        let serial = t["/emulator_screenshot ".len()..].trim();
        if serial.is_empty() {
            return Some(EmulatorAction::Reply(
                "❌ Usage: /emulator_screenshot <serial>".into(),
            ));
        }

        return Some(match emu_mgr.take_screenshot(serial) {
            Ok(path) => EmulatorAction::SendScreenshot {
                caption: format!("Screenshot: {}", serial),
                path,
            },
            Err(e) => EmulatorAction::Reply(format!("❌ Failed to take screenshot: {}", e)),
        });
    }

    None
}

/// Build a deep autonomous research prompt for the AgentManager.
///
/// Multi-phase methodology: discovery → deep reading → cross-referencing →
/// CRAAP evaluation → cited synthesis report.
pub(crate) fn build_research_prompt(topic: &str) -> String {
    format!(
        r#"You are a deep autonomous research agent. Your job is to produce a rigorous, \
fact-checked, cited research report on the topic below.

TOPIC: {topic}

══════════════════════════════════════════
PHASE 1 — DISCOVERY (multiple search angles)
══════════════════════════════════════════
Run at least 2-3 different web_search queries from different angles:
  • Main query: the topic directly
  • Alternative phrasing or a more specific sub-question
  • A query targeting recent news/developments (add "2025" or "2026" or "latest")
Collect at least 8-10 candidate URLs across all searches.

══════════════════════════════════════════
PHASE 2 — DEEP READING (extract & attribute)
══════════════════════════════════════════
Use web_browse (or http_fetch) on the 5-7 most promising URLs.
For each source, mentally note:
  • Key claims or data points it makes
  • Author/publisher and publication date
  • Whether it cites its own sources

══════════════════════════════════════════
PHASE 3 — CROSS-REFERENCE & FACT-CHECK
══════════════════════════════════════════
Compare claims across sources:
  • Which claims appear in 2+ independent sources? (high confidence)
  • Which claims appear in only 1 source? (flag as unverified)
  • Are there contradictions between sources? (note both sides)

══════════════════════════════════════════
PHASE 4 — CRAAP EVALUATION
══════════════════════════════════════════
Rate each source on the CRAAP framework (Currency, Relevance, Authority, Accuracy, Purpose).
Assign each source a trust tier:
  ⬢ HIGH — authoritative, recent, corroborated
  ◈ MEDIUM — partially corroborated or older
  ◇ LOW — single-source, opinion, or biased

══════════════════════════════════════════
PHASE 5 — FINAL REPORT (output this)
══════════════════════════════════════════
Format your final response EXACTLY as:

🔬 Deep Research: {topic}
━━━━━━━━━━━━━━━━━━━━━━━━

📌 Executive Summary
(2-3 sentence overview of the most important findings)

📊 Key Findings
(Numbered list. Each finding must cite its source as [N]. Mark confidence: ✅ confirmed by 2+ sources, ⚠️ single-source, ❌ contradicted)

🔍 Analysis & Cross-References
(Where sources agree, disagree, or fill different gaps. Note any contradictions.)

📋 Source Quality (CRAAP)
[1] URL — ⬢/◈/◇ TRUST — brief justification
[2] URL — ⬢/◈/◇ TRUST — brief justification
(etc.)

⚠️ Caveats & Limitations
(What you could not verify, what's missing, what needs human review)

RULES:
• Every factual claim MUST have a [N] citation.
• Never fabricate a URL or claim. If you cannot find information, say so.
• Keep total output under 6000 characters.
• Use plain text with emoji — no markdown headers or bold.
• Be direct, factual, and critical — not promotional."#,
        topic = topic
    )
}

/// Build a deep research prompt that uses the Shield Browser for anti-detect browsing.
///
/// Same multi-phase methodology as `build_research_prompt` but instructs the agent
/// to use `shield_browse` (headless anti-detect browser) instead of `web_browse`
/// for accessing sites that may block regular scrapers.
pub(crate) fn build_research_shield_prompt(topic: &str) -> String {
    format!(
        r#"You are a deep autonomous research agent with access to the Shield Browser — \
an anti-detect headless browser with C++-level fingerprint spoofing that renders \
JavaScript, bypasses bot detection, and defeats anti-scraping measures.

Your job is to produce a rigorous, fact-checked, cited research report.

TOPIC: {topic}

══════════════════════════════════════════
PHASE 1 — DISCOVERY (multiple search angles)
══════════════════════════════════════════
Run at least 2-3 different web_search queries from different angles:
  • Main query: the topic directly
  • Alternative phrasing or a more specific sub-question
  • A query targeting recent news/developments (add "2025" or "2026" or "latest")
Collect at least 8-10 candidate URLs across all searches.

══════════════════════════════════════════
PHASE 2 — DEEP READING via Shield Browser
══════════════════════════════════════════
Use shield_browse (NOT web_browse) on the 5-7 most promising URLs.
shield_browse launches a real browser with fingerprint spoofing — it can access
sites that block regular scrapers and fully renders JavaScript content.
If shield_browse fails on a URL, fall back to web_browse for that URL only.

For each source, mentally note:
  • Key claims or data points it makes
  • Author/publisher and publication date
  • Whether it cites its own sources

══════════════════════════════════════════
PHASE 3 — CROSS-REFERENCE & FACT-CHECK
══════════════════════════════════════════
Compare claims across sources:
  • Which claims appear in 2+ independent sources? (high confidence)
  • Which claims appear in only 1 source? (flag as unverified)
  • Are there contradictions between sources? (note both sides)

══════════════════════════════════════════
PHASE 4 — CRAAP EVALUATION
══════════════════════════════════════════
Rate each source on the CRAAP framework (Currency, Relevance, Authority, Accuracy, Purpose).
Assign each source a trust tier:
  ⬢ HIGH — authoritative, recent, corroborated
  ◈ MEDIUM — partially corroborated or older
  ◇ LOW — single-source, opinion, or biased

══════════════════════════════════════════
PHASE 5 — FINAL REPORT (output this)
══════════════════════════════════════════
Format your final response EXACTLY as:

🛡️🔬 Shield Deep Research: {topic}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📌 Executive Summary
(2-3 sentence overview of the most important findings)

📊 Key Findings
(Numbered list. Each finding must cite its source as [N]. Mark confidence: ✅ confirmed by 2+ sources, ⚠️ single-source, ❌ contradicted)

🔍 Analysis & Cross-References
(Where sources agree, disagree, or fill different gaps. Note any contradictions.)

📋 Source Quality (CRAAP)
[1] URL — ⬢/◈/◇ TRUST — brief justification
[2] URL — ⬢/◈/◇ TRUST — brief justification
(etc.)

⚠️ Caveats & Limitations
(What you could not verify, what's missing, what needs human review)

RULES:
• Every factual claim MUST have a [N] citation.
• Never fabricate a URL or claim. If you cannot find information, say so.
• Keep total output under 6000 characters.
• Use plain text with emoji — no markdown headers or bold.
• Be direct, factual, and critical — not promotional.
• Prefer shield_browse over web_browse for ALL page reads."#,
        topic = topic
    )
}

/// Format a research response for chat output.
///
/// The deep research prompts produce self-contained reports with headers,
/// so we only add a fallback header if the response doesn't already have one.
/// Truncates to Telegram's 4096-char limit with a continuation hint.
pub(crate) fn format_research_response(topic: &str, raw_response: &str) -> String {
    // The deep researcher prompt formats its own header; only add one if missing
    let has_header = raw_response.contains("Deep Research:")
        || raw_response.contains("Shield Deep Research:")
        || raw_response.contains("Executive Summary");

    let full = if has_header {
        raw_response.to_string()
    } else {
        format!("🔍 Research: {}\n{}\n\n{}", topic, "─".repeat(20), raw_response)
    };

    if full.len() > 4050 {
        format!(
            "{}…\n\n(truncated — ask a follow-up for the rest)",
            &full[..4000]
        )
    } else {
        full
    }
}

/// Route a message through the AgentManager for LLM processing.
pub(crate) async fn process_with_agent(
    message: &str,
    agent_manager: &Arc<tokio::sync::Mutex<AgentManager>>,
) -> String {
    run_agent_task(message, agent_manager, 120, 4000).await
}

/// Route a deep research task — longer timeout (5 min) and larger output cap.
pub(crate) async fn process_research_with_agent(
    message: &str,
    agent_manager: &Arc<tokio::sync::Mutex<AgentManager>>,
) -> String {
    run_agent_task(message, agent_manager, 300, 7000).await
}

/// Core agent task runner with configurable timeout and output limits.
async fn run_agent_task(
    message: &str,
    agent_manager: &Arc<tokio::sync::Mutex<AgentManager>>,
    timeout_secs: u64,
    max_output_chars: usize,
) -> String {
    let mgr = agent_manager.lock().await;
    let mut rx = mgr.subscribe();

    let task_id = match mgr.spawn(message).await {
        Ok(id) => id,
        Err(e) => return format!("❌ Agent error: {}", e),
    };
    drop(mgr);

    let timeout = tokio::time::Duration::from_secs(timeout_secs);
    let mut final_output = String::new();
    let mut error_output = String::new();

    loop {
        match tokio::time::timeout(timeout, rx.recv()).await {
            Ok(Ok(event)) => {
                if event.task_id() != task_id {
                    continue;
                }
                match &event {
                    AgentEvent::TaskCompleted { ref output, .. } => {
                        final_output = output.clone();
                    }
                    AgentEvent::TaskFailed { ref error, .. } => {
                        error_output = error.clone();
                    }
                    AgentEvent::TaskCancelled { .. } => {
                        error_output = "Task was cancelled.".to_string();
                    }
                    AgentEvent::TaskTimedOut { timeout_secs, .. } => {
                        error_output = format!("Task timed out after {}s.", timeout_secs);
                    }
                    _ => {}
                }
                if event.is_terminal() {
                    break;
                }
            }
            Ok(Err(_)) => break,
            Err(_) => return format!(
                "⏰ Agent timed out ({}s). Try a simpler request.",
                timeout_secs
            ),
        }
    }

    if !final_output.is_empty() {
        if final_output.len() > max_output_chars {
            format!("{}…\n\n(truncated)", &final_output[..max_output_chars])
        } else {
            final_output
        }
    } else if !error_output.is_empty() {
        format!("❌ Agent error: {}", error_output)
    } else {
        "🤖 Agent completed but produced no output.".into()
    }
}

/// Find a profile by name (case-insensitive) or ID prefix.
fn find_profile_by_query(pmgr: &ProfileManager, query: &str) -> Result<ShieldProfile, String> {
    let profiles = pmgr
        .list_profiles()
        .map_err(|e| format!("❌ Failed to list profiles: {}", e))?;

    if let Some(p) = profiles.iter().find(|p| p.id == query) {
        return Ok(p.clone());
    }

    let by_prefix: Vec<_> = profiles
        .iter()
        .filter(|p| p.id.starts_with(query))
        .collect();
    if by_prefix.len() == 1 {
        return Ok(by_prefix[0].clone());
    }

    let query_lower = query.to_lowercase();
    if let Some(p) = profiles
        .iter()
        .find(|p| p.name.to_lowercase() == query_lower)
    {
        return Ok(p.clone());
    }

    let by_name: Vec<_> = profiles
        .iter()
        .filter(|p| p.name.to_lowercase().contains(&query_lower))
        .collect();
    if by_name.len() == 1 {
        return Ok(by_name[0].clone());
    }
    if by_name.len() > 1 {
        let names: Vec<_> = by_name.iter().map(|p| format!("  • {}", p.name)).collect();
        return Err(format!(
            "Multiple profiles match \"{}\":\n{}",
            query,
            names.join("\n")
        ));
    }

    Err(format!(
        "❌ No profile found matching \"{}\". Use /profiles to see all.",
        query
    ))
}

/// Format an epoch timestamp as a human-readable date.
pub(crate) fn format_epoch(epoch: u64) -> String {
    let secs = epoch as i64;
    let days_since_epoch = secs / 86400;
    let year = 1970 + (days_since_epoch * 400 / 146097) as u32;
    let remaining = days_since_epoch
        - ((year as i64 - 1970) * 365 + (year as i64 - 1970) / 4 - (year as i64 - 1970) / 100
            + (year as i64 - 1970) / 400);
    let month = (remaining / 30).clamp(1, 12) as u32;
    let day = (remaining % 30 + 1).clamp(1, 31) as u32;
    format!("{:04}-{:02}-{:02}", year, month, day)
}
