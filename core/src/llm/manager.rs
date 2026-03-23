use super::streaming::ModelInfo;
use super::LlmProvider;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for an LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub provider_type: String, // "ollama", "openai", "anthropic", "groq", "together", etc.
    pub model: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub api_key_env: Option<String>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

fn default_max_tokens() -> u32 {
    4096
}

/// Registry of configured LLM providers with hot-swap support.
pub struct LlmManager {
    providers: HashMap<String, Box<dyn LlmProvider>>,
    configs: Vec<ProviderConfig>,
    active: String,
}

impl LlmManager {
    /// Create a new manager with no providers.
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            configs: Vec::new(),
            active: String::new(),
        }
    }

    /// Build a manager from a list of provider configs.
    pub fn from_configs(configs: Vec<ProviderConfig>) -> Result<Self> {
        let mut mgr = Self::new();
        for config in configs {
            mgr.add_from_config(config)?;
        }
        Ok(mgr)
    }

    /// Add a provider from config — resolves env vars for API keys.
    pub fn add_from_config(&mut self, config: ProviderConfig) -> Result<()> {
        let api_key = config
            .api_key
            .clone()
            .or_else(|| {
                config
                    .api_key_env
                    .as_ref()
                    .and_then(|env_name| std::env::var(env_name).ok())
            });

        let provider = super::create_provider(
            &config.provider_type,
            &config.model,
            config.base_url.as_deref(),
            api_key.as_deref(),
        )?;

        let name = config.name.clone();
        if self.active.is_empty() {
            self.active = name.clone();
        }
        self.providers.insert(name, provider);
        self.configs.push(config);
        Ok(())
    }

    /// Add a pre-built provider.
    pub fn add_provider(&mut self, name: &str, provider: Box<dyn LlmProvider>) {
        if self.active.is_empty() {
            self.active = name.to_string();
        }
        self.providers.insert(name.to_string(), provider);
    }

    /// Get the active provider.
    pub fn active_provider(&self) -> Result<&dyn LlmProvider> {
        self.providers
            .get(&self.active)
            .map(|p| p.as_ref())
            .ok_or_else(|| anyhow!("No active LLM provider '{}' found", self.active))
    }

    /// Switch active provider by name.
    pub fn switch(&mut self, name: &str) -> Result<()> {
        if !self.providers.contains_key(name) {
            return Err(anyhow!(
                "Provider '{}' not registered. Available: {:?}",
                name,
                self.provider_names()
            ));
        }
        self.active = name.to_string();
        tracing::info!(provider = name, "Switched active LLM provider");
        Ok(())
    }

    /// Get the name of the active provider.
    pub fn active_name(&self) -> &str {
        &self.active
    }

    /// List all registered provider names.
    pub fn provider_names(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    /// List all configs.
    pub fn configs(&self) -> &[ProviderConfig] {
        &self.configs
    }

    /// Get a provider by name (immutable).
    pub fn get(&self, name: &str) -> Option<&dyn LlmProvider> {
        self.providers.get(name).map(|p| p.as_ref())
    }

    /// Remove a provider by name.
    pub fn remove(&mut self, name: &str) -> bool {
        let removed = self.providers.remove(name).is_some();
        if removed && self.active == name {
            self.active = self
                .providers
                .keys()
                .next()
                .cloned()
                .unwrap_or_default();
        }
        self.configs.retain(|c| c.name != name);
        removed
    }

    /// Get status summary of all providers.
    pub fn status(&self) -> Vec<ProviderStatus> {
        self.providers
            .iter()
            .map(|(name, provider)| ProviderStatus {
                name: name.clone(),
                provider_type: provider.name().to_string(),
                is_active: name == &self.active,
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderStatus {
    pub name: String,
    pub provider_type: String,
    pub is_active: bool,
}

impl Default for LlmManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_add_and_switch() {
        let mut mgr = LlmManager::new();

        // Add ollama (default, no key needed)
        let config = ProviderConfig {
            name: "local".into(),
            provider_type: "ollama".into(),
            model: "llama3.2".into(),
            base_url: None,
            api_key: None,
            api_key_env: None,
            max_tokens: 4096,
        };
        mgr.add_from_config(config).unwrap();

        assert_eq!(mgr.active_name(), "local");
        assert_eq!(mgr.provider_names().len(), 1);

        // Add another
        let config2 = ProviderConfig {
            name: "cloud".into(),
            provider_type: "ollama".into(),
            model: "mistral".into(),
            base_url: None,
            api_key: None,
            api_key_env: None,
            max_tokens: 4096,
        };
        mgr.add_from_config(config2).unwrap();

        // Switch
        mgr.switch("cloud").unwrap();
        assert_eq!(mgr.active_name(), "cloud");

        // Bad switch
        assert!(mgr.switch("nonexistent").is_err());
    }

    #[test]
    fn test_manager_remove() {
        let mut mgr = LlmManager::new();
        mgr.add_from_config(ProviderConfig {
            name: "a".into(),
            provider_type: "ollama".into(),
            model: "m".into(),
            base_url: None,
            api_key: None,
            api_key_env: None,
            max_tokens: 4096,
        })
        .unwrap();
        mgr.add_from_config(ProviderConfig {
            name: "b".into(),
            provider_type: "ollama".into(),
            model: "m".into(),
            base_url: None,
            api_key: None,
            api_key_env: None,
            max_tokens: 4096,
        })
        .unwrap();

        assert_eq!(mgr.active_name(), "a");
        mgr.remove("a");
        // Active should auto-switch
        assert_eq!(mgr.provider_names().len(), 1);
        assert_eq!(mgr.active_name(), "b");
    }
}
