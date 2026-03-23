use crate::llm::ToolDef;
use crate::sandbox::Sandbox;
use crate::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;

/// Lists installed apps and their status.
pub struct AppListTool;

#[async_trait]
impl Tool for AppListTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "app_list".into(),
            description: "List all installed AI apps and their status (installed, running). Also shows the app catalog of available apps.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "include_catalog": {
                        "type": "boolean",
                        "description": "If true, also show available (not installed) apps from the catalog (default: false)",
                        "default": false
                    }
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, sandbox: &Sandbox) -> Result<String> {
        // Read registry.json from the apps base directory
        // The apps base dir is typically at sandbox root's parent, but we read from sandbox data
        let _registry_path = sandbox.root().join("data").join("app_registry_snapshot.json");

        // Fallback: read from the well-known registry location
        let alt_paths = [
            sandbox.root().join("..").join("registry.json"),
            sandbox.root().join("registry.json"),
        ];

        let mut output = String::from("=== NDE-OS App Status ===\n\n");

        // Try to read registry from accessible locations
        let mut found_registry = false;
        for path in &alt_paths {
            if let Ok(canonical) = path.canonicalize() {
                if let Ok(content) = std::fs::read_to_string(&canonical) {
                    if let Ok(registry) = serde_json::from_str::<serde_json::Value>(&content) {
                        found_registry = true;
                        if let Some(obj) = registry.as_object() {
                            output.push_str(&format!("Installed apps: {}\n\n", obj.len()));
                            for (id, app) in obj {
                                let name = app.get("manifest")
                                    .and_then(|m| m.get("name"))
                                    .and_then(|n| n.as_str())
                                    .unwrap_or(id);
                                let status = app.get("status")
                                    .map(|s| format!("{}", s))
                                    .unwrap_or_else(|| "unknown".into());
                                let workspace = app.get("workspace")
                                    .and_then(|w| w.as_str())
                                    .unwrap_or("?");
                                output.push_str(&format!("  [{}] {} — status: {}\n    workspace: {}\n\n", id, name, status, workspace));
                            }
                        }
                        break;
                    }
                }
            }
        }

        if !found_registry {
            output.push_str("No apps installed yet.\n");
        }

        let include_catalog = args.get("include_catalog")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if include_catalog {
            output.push_str("\n=== Available Catalog ===\n");
            output.push_str("Use the app_install tool to install any of these:\n\n");
            output.push_str("  • stable-diffusion-webui — Stable Diffusion image generation\n");
            output.push_str("  • ollama — Local LLM server\n");
            output.push_str("  • sample-counter — Simple Python counter demo\n");
            output.push_str("  • sample-node-fullstack — Node.js fullstack demo\n");
        }

        Ok(output)
    }
}

/// Installs an app from the catalog by ID.
pub struct AppInstallTool;

#[async_trait]
impl Tool for AppInstallTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "app_install".into(),
            description: "Install an AI app by its catalog ID. This creates a sandboxed workspace, sets up Python venv via uv, and installs dependencies.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "app_id": {
                        "type": "string",
                        "description": "The app ID from the catalog (e.g. 'stable-diffusion-webui', 'ollama')"
                    }
                },
                "required": ["app_id"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, sandbox: &Sandbox) -> Result<String> {
        let app_id = args.get("app_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'app_id' argument"))?;

        // Store the intent — the actual install goes through the server API
        // This tool records the install request for the server to process
        let request = serde_json::json!({
            "action": "install",
            "app_id": app_id,
            "requested_at": chrono::Utc::now().to_rfc3339(),
        });

        let actions_dir = sandbox.root().join("data").join("pending_actions");
        std::fs::create_dir_all(&actions_dir)?;

        let action_file = actions_dir.join(format!("install_{}.json", app_id));
        std::fs::write(&action_file, serde_json::to_string_pretty(&request)?)?;

        // Also try to call the REST API directly
        Ok(format!(
            "Install request queued for '{}'. The server will process the installation:\n\
             1. Create sandbox workspace\n\
             2. Setup Python venv via uv\n\
             3. Install pip dependencies\n\
             4. App will appear in app_list once installed\n\n\
             To check status, use: app_list",
            app_id
        ))
    }
}

/// Launches a running app by ID.
pub struct AppLaunchTool;

#[async_trait]
impl Tool for AppLaunchTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "app_launch".into(),
            description: "Launch an installed AI app. The app runs in its sandboxed workspace with its own venv.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "app_id": {
                        "type": "string",
                        "description": "The installed app ID to launch"
                    }
                },
                "required": ["app_id"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, sandbox: &Sandbox) -> Result<String> {
        let app_id = args.get("app_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'app_id' argument"))?;

        let request = serde_json::json!({
            "action": "launch",
            "app_id": app_id,
            "requested_at": chrono::Utc::now().to_rfc3339(),
        });

        let actions_dir = sandbox.root().join("data").join("pending_actions");
        std::fs::create_dir_all(&actions_dir)?;

        let action_file = actions_dir.join(format!("launch_{}.json", app_id));
        std::fs::write(&action_file, serde_json::to_string_pretty(&request)?)?;

        Ok(format!(
            "Launch request queued for '{}'. The server will start the process.\n\
             The app will be accessible on its configured port once running.\n\
             Use app_list to check status.",
            app_id
        ))
    }
}

/// Stops a running app by ID.
pub struct AppStopTool;

#[async_trait]
impl Tool for AppStopTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "app_stop".into(),
            description: "Stop a running AI app by its ID.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "app_id": {
                        "type": "string",
                        "description": "The running app ID to stop"
                    }
                },
                "required": ["app_id"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, sandbox: &Sandbox) -> Result<String> {
        let app_id = args.get("app_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'app_id' argument"))?;

        let request = serde_json::json!({
            "action": "stop",
            "app_id": app_id,
            "requested_at": chrono::Utc::now().to_rfc3339(),
        });

        let actions_dir = sandbox.root().join("data").join("pending_actions");
        std::fs::create_dir_all(&actions_dir)?;

        let action_file = actions_dir.join(format!("stop_{}.json", app_id));
        std::fs::write(&action_file, serde_json::to_string_pretty(&request)?)?;

        Ok(format!("Stop request queued for '{}'.", app_id))
    }
}
