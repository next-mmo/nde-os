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
            embedding: None,
            vlm: None,
        }
    }
}

impl VikingConfig {
    /// Base URL for the OpenViking HTTP API.
    pub fn base_url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    /// Write the `ov.conf` configuration file for the OpenViking server.
    pub fn write_server_conf(&self, conf_dir: &Path) -> Result<PathBuf> {
        std::fs::create_dir_all(conf_dir)?;
        let conf_path = conf_dir.join("ov.conf");

        let mut conf = serde_json::json!({
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
