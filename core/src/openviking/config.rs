use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// OpenViking server + client configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VikingConfig {
    /// Port the OpenViking server listens on.
    pub port: u16,
    /// Workspace directory for OpenViking data storage.
    pub workspace: PathBuf,
    /// Embedding model configuration.
    pub embedding: Option<EmbeddingConfig>,
    /// Vision-Language model configuration.
    pub vlm: Option<VlmConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    pub dimension: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlmConfig {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub api_base: Option<String>,
}

impl Default for VikingConfig {
    fn default() -> Self {
        Self {
            port: 1933,
            workspace: PathBuf::from("openviking_workspace"),
            embedding: Some(EmbeddingConfig {
                provider: "litellm".into(),
                model: "qwen2.5-0.5b-instruct".into(),
                api_key: None,
                api_base: Some("http://127.0.0.1:8090/v1".into()),
                dimension: 512,
            }),
            vlm: None,
        }
    }
}

impl VikingConfig {
    /// Build a VikingConfig from persisted service hub settings.
    /// Falls back to `Default` for any missing values.
    pub fn from_service_config(data_dir: &Path) -> Self {
        let cfg = crate::services::config::get_service_config("openviking", data_dir).ok();
        let vals = cfg.as_ref().map(|c| &c.values);

        let defaults = Self::default();

        let port = vals
            .and_then(|v| v.get("port"))
            .and_then(|v| v.as_u64())
            .map(|v| v as u16)
            .unwrap_or(defaults.port);

        let provider = vals
            .and_then(|v| v.get("embedding_provider"))
            .and_then(|v| v.as_str())
            .unwrap_or("litellm")
            .to_string();

        let model = vals
            .and_then(|v| v.get("embedding_model"))
            .and_then(|v| v.as_str())
            .unwrap_or("qwen2.5-0.5b-instruct")
            .to_string();

        let dimension = vals
            .and_then(|v| v.get("embedding_dimension"))
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(512);

        let api_key = vals
            .and_then(|v| v.get("embedding_api_key"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from);

        let api_base = vals
            .and_then(|v| v.get("embedding_api_base"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from)
            .or_else(|| Some("http://127.0.0.1:8090/v1".into()));

        Self {
            port,
            workspace: defaults.workspace,
            embedding: Some(EmbeddingConfig {
                provider,
                model,
                api_key,
                api_base,
                dimension,
            }),
            vlm: None,
        }
    }

    /// Base URL for the OpenViking HTTP API.
    pub fn base_url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    /// Write the `ov.conf` configuration file for the OpenViking server.
    pub fn write_server_conf(&self, conf_dir: &Path) -> Result<PathBuf> {
        std::fs::create_dir_all(conf_dir)?;
        let conf_path = conf_dir.join("ov.conf");

        let mut conf = serde_json::json!({
            "server": {
                "host": "127.0.0.1",
                "port": self.port
            },
            "storage": {
                "workspace": self.workspace.to_string_lossy()
            },
            "log": {
                "level": "INFO",
                "output": "stdout"
            }
        });

        if let Some(emb) = &self.embedding {
            conf["embedding"] = serde_json::json!({
                "dense": {
                    "provider": emb.provider,
                    "model": emb.model,
                    "dimension": emb.dimension,
                    "api_key": emb.api_key,
                    "api_base": emb.api_base,
                },
                "max_concurrent": 10
            });
        }

        if let Some(vlm) = &self.vlm {
            conf["vlm"] = serde_json::json!({
                "provider": vlm.provider,
                "model": vlm.model,
                "api_key": vlm.api_key,
                "api_base": vlm.api_base,
                "max_concurrent": 100
            });
        }

        std::fs::write(&conf_path, serde_json::to_string_pretty(&conf)?)?;
        Ok(conf_path)
    }

    /// Write the `ovcli.conf` client configuration file.
    pub fn write_client_conf(&self, conf_dir: &Path) -> Result<PathBuf> {
        std::fs::create_dir_all(conf_dir)?;
        let conf_path = conf_dir.join("ovcli.conf");

        let conf = serde_json::json!({
            "url": self.base_url(),
            "timeout": 60.0,
            "output": "json"
        });

        std::fs::write(&conf_path, serde_json::to_string_pretty(&conf)?)?;
        Ok(conf_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = VikingConfig::default();
        assert_eq!(config.port, 1933);
        assert_eq!(config.base_url(), "http://localhost:1933");
        assert!(
            config.embedding.is_some(),
            "default must include embedding config"
        );
    }

    #[test]
    fn test_default_conf_has_embedding() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = VikingConfig::default();
        config.workspace = dir.path().join("viking_data");
        let conf_path = config.write_server_conf(dir.path()).unwrap();
        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&conf_path).unwrap()).unwrap();
        assert!(
            content.get("embedding").is_some(),
            "ov.conf must contain embedding section"
        );
        assert_eq!(content["embedding"]["dense"]["provider"], "litellm");
    }

    #[test]
    fn test_write_server_conf() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = VikingConfig::default();
        config.workspace = dir.path().join("viking_data");
        config.embedding = Some(EmbeddingConfig {
            provider: "openai".into(),
            model: "text-embedding-3-large".into(),
            api_key: Some("test-key".into()),
            api_base: Some("https://api.openai.com/v1".into()),
            dimension: 3072,
        });

        let conf_path = config.write_server_conf(dir.path()).unwrap();
        assert!(conf_path.exists());

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&conf_path).unwrap()).unwrap();
        assert_eq!(content["embedding"]["dense"]["provider"], "openai");
    }

    #[test]
    fn test_write_client_conf() {
        let dir = tempfile::tempdir().unwrap();
        let config = VikingConfig::default();
        let conf_path = config.write_client_conf(dir.path()).unwrap();
        assert!(conf_path.exists());

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&conf_path).unwrap()).unwrap();
        assert_eq!(content["url"], "http://localhost:1933");
    }
}
