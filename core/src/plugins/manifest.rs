use serde::{Deserialize, Serialize};

/// Plugin manifest schema v2.
/// Each plugin has a manifest.json that declares its capabilities, dependencies,
/// hooks, tools, and UI panels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Unique plugin identifier (e.g., "gpu-monitor")
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Semantic version (e.g., "1.0.0")
    pub version: String,
    /// Plugin type determines lifecycle
    #[serde(rename = "type")]
    pub plugin_type: PluginType,
    /// Short description
    pub description: String,
    /// Author name or org
    pub author: String,
    /// Entry point script (e.g., "main.py", "index.js")
    #[serde(default)]
    pub entry: Option<String>,
    /// Programming language of entry
    #[serde(default = "default_language")]
    pub language: Language,
    /// Runtime dependencies (pip packages, npm packages)
    #[serde(default)]
    pub deps: Vec<String>,
    /// Event hooks this plugin subscribes to
    #[serde(default)]
    pub hooks: Vec<super::HookType>,
    /// Tools this plugin contributes to the agent
    #[serde(default)]
    pub provides_tools: Vec<PluginToolDef>,
    /// HTTP API routes this plugin serves
    #[serde(default)]
    pub api_routes: Vec<ApiRoute>,
    /// Svelte UI panel config
    #[serde(default)]
    pub ui_panel: Option<UiPanel>,
    /// Required permissions
    #[serde(default)]
    pub permissions: Vec<Permission>,
    /// Plugin-specific configuration schema
    #[serde(default)]
    pub config_schema: serde_json::Value,
    /// Periodic interval for monitor-type plugins (seconds)
    #[serde(default)]
    pub interval_seconds: Option<u64>,
    /// Port the plugin serves on (for daemon-type)
    #[serde(default)]
    pub port: Option<u16>,
}

/// Plugin type determines how the engine manages it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginType {
    /// Periodic polling (gpu-monitor, disk-cleaner)
    Monitor,
    /// Event-driven middleware
    Hook,
    /// Adds new LLM/tool/channel providers
    Provider,
    /// Standalone agent tool
    Tool,
    /// Desktop panel/widget
    UiPanel,
    /// Long-running background service
    Daemon,
}

/// Language of the plugin entry point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Python,
    JavaScript,
    TypeScript,
    Binary,
}

fn default_language() -> Language {
    Language::Python
}

/// A tool definition contributed by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginToolDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// An HTTP route served by the plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRoute {
    pub method: String,
    pub path: String,
    pub description: String,
}

/// UI panel configuration for desktop integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiPanel {
    /// Panel title in the dock/menu
    pub title: String,
    /// Icon name (from Lucide icons)
    pub icon: String,
    /// Width in pixels (default: 400)
    #[serde(default = "default_panel_width")]
    pub width: u32,
    /// Height in pixels (default: 600)
    #[serde(default = "default_panel_height")]
    pub height: u32,
    /// URL path for the panel (served by plugin's API)
    #[serde(default)]
    pub url: Option<String>,
}

fn default_panel_width() -> u32 {
    400
}
fn default_panel_height() -> u32 {
    600
}

/// Permissions a plugin can request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// Read files in its sandbox
    FileRead,
    /// Write files in its sandbox
    FileWrite,
    /// Execute shell commands
    ShellExec,
    /// Network access
    Network,
    /// GPU access
    Gpu,
    /// Access to agent chat
    AgentChat,
    /// Access to system metrics
    SystemMetrics,
    /// Access to other plugins
    PluginInterop,
}

impl PluginManifest {
    /// Load from a JSON file.
    pub fn from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read plugin manifest {}: {}", path.display(), e))?;
        let manifest: Self = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse plugin manifest {}: {}", path.display(), e))?;
        Ok(manifest)
    }

    /// Validate the manifest for required fields and consistency.
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.id.is_empty() {
            return Err(anyhow::anyhow!("Plugin ID is required"));
        }
        if self.name.is_empty() {
            return Err(anyhow::anyhow!("Plugin name is required"));
        }
        if self.version.is_empty() {
            return Err(anyhow::anyhow!("Plugin version is required"));
        }
        // Daemon/monitor types should have an entry point
        if matches!(
            self.plugin_type,
            PluginType::Monitor | PluginType::Daemon | PluginType::Provider
        ) && self.entry.is_none()
        {
            return Err(anyhow::anyhow!(
                "Plugin type {:?} requires an entry point",
                self.plugin_type
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_manifest() {
        let json = r#"{
            "id": "gpu-monitor",
            "name": "GPU Monitor",
            "version": "1.0.0",
            "type": "monitor",
            "description": "GPU stats",
            "author": "nde-os",
            "entry": "monitor.py",
            "language": "python",
            "deps": ["pynvml"],
            "hooks": [],
            "api_routes": [
                {"method": "GET", "path": "/api/plugins/gpu-monitor/stats", "description": "Stats"}
            ]
        }"#;

        let manifest: PluginManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.id, "gpu-monitor");
        assert_eq!(manifest.plugin_type, PluginType::Monitor);
        assert_eq!(manifest.deps, vec!["pynvml"]);
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_validate_missing_entry() {
        let manifest = PluginManifest {
            id: "test".into(),
            name: "Test".into(),
            version: "1.0.0".into(),
            plugin_type: PluginType::Daemon,
            description: "A test".into(),
            author: "test".into(),
            entry: None,
            language: Language::Python,
            deps: vec![],
            hooks: vec![],
            provides_tools: vec![],
            api_routes: vec![],
            ui_panel: None,
            permissions: vec![],
            config_schema: serde_json::json!({}),
            interval_seconds: None,
            port: None,
        };
        assert!(manifest.validate().is_err());
    }
}
