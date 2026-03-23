use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

/// Agent configuration parsed from TOML.
#[derive(Debug, Clone, Deserialize)]
pub struct AgentConfig {
    #[serde(default = "default_name")]
    pub name: String,

    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,

    #[serde(default)]
    pub system_prompt: String,

    // ── Model ────────────────────────────────────────────────────────────
    #[serde(default = "default_provider")]
    pub model_provider: String,

    #[serde(default = "default_model")]
    pub model_name: String,

    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub api_key_env: Option<String>,

    // ── Tools ────────────────────────────────────────────────────────────
    #[serde(default = "default_tools")]
    pub enabled_tools: Vec<String>,

    // ── Sandbox ──────────────────────────────────────────────────────────
    #[serde(default = "default_workspace")]
    pub workspace: String,
}

fn default_name() -> String { "assistant".into() }
fn default_max_iterations() -> usize { 25 }
fn default_provider() -> String { "ollama".into() }
fn default_model() -> String { "llama3.2".into() }
fn default_workspace() -> String { "./workspace".into() }
fn default_tools() -> Vec<String> {
    vec![
        // Filesystem
        "file_read".into(), "file_write".into(), "file_delete".into(),
        "file_list".into(), "file_search".into(), "file_patch".into(),
        // Shell
        "shell_exec".into(),
        // Code
        "code_search".into(), "code_edit".into(), "code_symbols".into(),
        // Memory
        "memory_store".into(), "memory_recall".into(),
        "conversation_save".into(), "conversation_search".into(),
        // Knowledge
        "knowledge_store".into(), "knowledge_query".into(),
        // System
        "app_list".into(), "app_install".into(), "app_launch".into(), "app_stop".into(),
        "system_info".into(), "http_fetch".into(), "skill_list".into(),
    ]
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: default_name(),
            max_iterations: default_max_iterations(),
            system_prompt: String::new(),
            model_provider: default_provider(),
            model_name: default_model(),
            base_url: None,
            api_key: None,
            api_key_env: None,
            enabled_tools: default_tools(),
            workspace: default_workspace(),
        }
    }
}

impl AgentConfig {
    /// Load from a TOML file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config: {}", path.as_ref().display()))?;
        Self::from_str(&content)
    }

    /// Parse from a TOML string.
    pub fn from_str(toml_str: &str) -> Result<Self> {
        let raw: RawConfig = toml::from_str(toml_str)
            .context("Failed to parse TOML config")?;
        Ok(raw.into_agent_config())
    }
}

/// Raw config structure matching the TOML layout.
#[derive(Deserialize)]
struct RawConfig {
    #[serde(default)]
    agent: RawAgent,
    #[serde(default)]
    model: RawModel,
    #[serde(default)]
    tools: RawTools,
    #[serde(default)]
    sandbox: RawSandbox,
}

#[derive(Deserialize, Default)]
struct RawAgent {
    name: Option<String>,
    max_iterations: Option<usize>,
    system_prompt: Option<String>,
}

#[derive(Deserialize, Default)]
struct RawModel {
    provider: Option<String>,
    model: Option<String>,
    base_url: Option<String>,
    api_key_env: Option<String>,
}

#[derive(Deserialize, Default)]
struct RawTools {
    #[serde(default)]
    enabled: Vec<String>,
}

#[derive(Deserialize, Default)]
struct RawSandbox {
    workspace: Option<String>,
}

impl RawConfig {
    fn into_agent_config(self) -> AgentConfig {
        // Resolve API key from env var if specified
        let api_key = self.model.api_key_env.as_ref()
            .and_then(|env_name| std::env::var(env_name).ok());

        AgentConfig {
            name: self.agent.name.unwrap_or_else(default_name),
            max_iterations: self.agent.max_iterations.unwrap_or_else(default_max_iterations),
            system_prompt: self.agent.system_prompt.unwrap_or_default(),
            model_provider: self.model.provider.unwrap_or_else(default_provider),
            model_name: self.model.model.unwrap_or_else(default_model),
            base_url: self.model.base_url,
            api_key,
            api_key_env: self.model.api_key_env,
            enabled_tools: if self.tools.enabled.is_empty() { default_tools() } else { self.tools.enabled },
            workspace: self.sandbox.workspace.unwrap_or_else(default_workspace),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AgentConfig::default();
        assert_eq!(config.max_iterations, 25);
        assert_eq!(config.model_provider, "ollama");
        assert_eq!(config.enabled_tools.len(), 23);
    }

    #[test]
    fn test_parse_toml() {
        let toml = r#"
[agent]
name = "test"
max_iterations = 10

[model]
provider = "openai"
model = "gpt-4o"
api_key_env = "OPENAI_API_KEY"

[tools]
enabled = ["file_read", "shell_exec"]

[sandbox]
workspace = "./test_workspace"
"#;
        let config = AgentConfig::from_str(toml).unwrap();
        assert_eq!(config.name, "test");
        assert_eq!(config.max_iterations, 10);
        assert_eq!(config.model_provider, "openai");
        assert_eq!(config.enabled_tools.len(), 2);
    }
}
