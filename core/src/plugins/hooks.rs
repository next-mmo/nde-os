use serde::{Deserialize, Serialize};

/// Event types that plugins can hook into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookType {
    /// Before the agent processes a user message
    OnMessageBefore,
    /// After the agent processes a user message
    OnMessageAfter,
    /// Before a tool is executed
    OnToolCallBefore,
    /// After a tool is executed
    OnToolCallAfter,
    /// When an app is launched
    OnAppLaunch,
    /// When an app is stopped
    OnAppStop,
    /// When an app is installed
    OnAppInstall,
    /// When an app is uninstalled
    OnAppUninstall,
    /// System startup
    OnStartup,
    /// System shutdown
    OnShutdown,
    /// On new conversation created
    OnConversationCreate,
    /// Periodic tick (for monitors)
    OnTick,
}

/// Context passed to hook handlers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookContext {
    /// Which hook is firing
    pub hook_type: HookType,
    /// Relevant data for the hook (varies by type)
    pub data: serde_json::Value,
    /// Timestamp of the event
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl HookContext {
    pub fn new(hook_type: HookType, data: serde_json::Value) -> Self {
        Self {
            hook_type,
            data,
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Result from a hook handler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    /// ID of the plugin that handled this
    pub plugin_id: String,
    /// Whether the hook allows processing to continue
    pub allow_continue: bool,
    /// Modified data (plugins can transform the data)
    pub data: Option<serde_json::Value>,
    /// Any error message
    pub error: Option<String>,
}

impl HookResult {
    pub fn ok(plugin_id: &str) -> Self {
        Self {
            plugin_id: plugin_id.to_string(),
            allow_continue: true,
            data: None,
            error: None,
        }
    }

    pub fn with_data(plugin_id: &str, data: serde_json::Value) -> Self {
        Self {
            plugin_id: plugin_id.to_string(),
            allow_continue: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn block(plugin_id: &str, reason: &str) -> Self {
        Self {
            plugin_id: plugin_id.to_string(),
            allow_continue: false,
            data: None,
            error: Some(reason.to_string()),
        }
    }
}
