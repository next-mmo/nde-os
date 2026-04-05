//! Per-service configuration — schema definitions, persistence, and defaults.
//!
//! Each service declares its configurable fields (provider, model, port, paths, etc.).
//! Values are persisted to `{data_dir}/service_configs.json` and loaded on startup.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The type of a config field — drives UI rendering.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ConfigFieldType {
    Text,
    Number,
    Password,
    Select,
    Toggle,
    Path,
}

/// Schema for a single configurable field on a service.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigField {
    pub key: String,
    pub label: String,
    pub description: String,
    pub field_type: ConfigFieldType,
    pub default: serde_json::Value,
    /// Available options for `Select` fields.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    /// Whether the field is required for the service to function.
    #[serde(default)]
    pub required: bool,
}

/// Full config schema + current values for a single service.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceConfig {
    pub service_id: String,
    pub fields: Vec<ConfigField>,
    pub values: HashMap<String, serde_json::Value>,
}

/// Manages reading/writing per-service configs to disk.
pub struct ServiceConfigStore {
    path: PathBuf,
}

impl ServiceConfigStore {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            path: data_dir.join("service_configs.json"),
        }
    }

    /// Load all persisted configs. Returns empty map if file doesn't exist.
    pub fn load_all(&self) -> Result<HashMap<String, HashMap<String, serde_json::Value>>> {
        if !self.path.exists() {
            return Ok(HashMap::new());
        }
        let contents = std::fs::read_to_string(&self.path)
            .context("Failed to read service_configs.json")?;
        let data = serde_json::from_str(&contents)
            .context("Failed to parse service_configs.json")?;
        Ok(data)
    }

    /// Load config values for a single service.
    pub fn load(&self, service_id: &str) -> Result<HashMap<String, serde_json::Value>> {
        let all = self.load_all()?;
        Ok(all.get(service_id).cloned().unwrap_or_default())
    }

    /// Save config values for a single service (merges into existing file).
    pub fn save(&self, service_id: &str, values: &HashMap<String, serde_json::Value>) -> Result<()> {
        let mut all = self.load_all().unwrap_or_default();
        all.insert(service_id.to_string(), values.clone());
        let json = serde_json::to_string_pretty(&all)
            .context("Failed to serialize service configs")?;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, json)
            .context("Failed to write service_configs.json")?;
        Ok(())
    }
}

/// Return the config schema for a given service, populated with persisted (or default) values.
pub fn get_service_config(service_id: &str, data_dir: &Path) -> Result<ServiceConfig> {
    let store = ServiceConfigStore::new(data_dir);
    let saved = store.load(service_id)?;
    let fields = config_fields_for(service_id);

    let mut values = HashMap::new();
    for field in &fields {
        let val = saved
            .get(&field.key)
            .cloned()
            .unwrap_or_else(|| field.default.clone());
        values.insert(field.key.clone(), val);
    }

    Ok(ServiceConfig {
        service_id: service_id.to_string(),
        fields,
        values,
    })
}

/// Save user-supplied config values for a service.
pub fn set_service_config(
    service_id: &str,
    values: HashMap<String, serde_json::Value>,
    data_dir: &Path,
) -> Result<()> {
    let store = ServiceConfigStore::new(data_dir);
    store.save(service_id, &values)
}

// ─── Per-Service Field Definitions ───────────────────────────────────────────

fn config_fields_for(service_id: &str) -> Vec<ConfigField> {
    match service_id {
        "openviking" => openviking_fields(),
        "rvc" => rvc_fields(),
        "voice-runtime" => voice_runtime_fields(),
        "ffmpeg" => ffmpeg_fields(),
        "python" => python_fields(),
        "uv" => uv_fields(),
        _ => vec![],
    }
}

fn openviking_fields() -> Vec<ConfigField> {
    vec![
        ConfigField {
            key: "port".into(),
            label: "Port".into(),
            description: "Port the OpenViking server listens on".into(),
            field_type: ConfigFieldType::Number,
            default: serde_json::json!(1933),
            options: vec![],
            required: true,
        },
        ConfigField {
            key: "embedding_provider".into(),
            label: "Embedding Provider".into(),
            description: "Which embedding API to use for dense vectors".into(),
            field_type: ConfigFieldType::Select,
            default: serde_json::json!("litellm"),
            options: vec![
                "openai".into(), "azure".into(), "ollama".into(), "jina".into(),
                "gemini".into(), "voyage".into(), "cohere".into(), "litellm".into(),
                "volcengine".into(), "vikingdb".into(), "minimax".into(),
            ],
            required: true,
        },
        ConfigField {
            key: "embedding_model".into(),
            label: "Embedding Model".into(),
            description: "Model name for embeddings (e.g. text-embedding-3-large, nomic-embed-text)".into(),
            field_type: ConfigFieldType::Text,
            default: serde_json::json!("qwen2.5-0.5b-instruct"),
            options: vec![],
            required: true,
        },
        ConfigField {
            key: "embedding_dimension".into(),
            label: "Embedding Dimension".into(),
            description: "Vector dimension of the embedding model".into(),
            field_type: ConfigFieldType::Number,
            default: serde_json::json!(512),
            options: vec![],
            required: true,
        },
        ConfigField {
            key: "embedding_api_key".into(),
            label: "Embedding API Key".into(),
            description: "API key for the embedding provider (leave empty for local providers)".into(),
            field_type: ConfigFieldType::Password,
            default: serde_json::json!(""),
            options: vec![],
            required: false,
        },
        ConfigField {
            key: "embedding_api_base".into(),
            label: "Embedding API Base URL".into(),
            description: "Base URL for the embedding API (e.g. http://127.0.0.1:8090/v1)".into(),
            field_type: ConfigFieldType::Text,
            default: serde_json::json!("http://127.0.0.1:8090/v1"),
            options: vec![],
            required: false,
        },
    ]
}

fn rvc_fields() -> Vec<ConfigField> {
    vec![
        ConfigField {
            key: "model_path".into(),
            label: "Model Path".into(),
            description: "Path to the RVC voice model (.pth file)".into(),
            field_type: ConfigFieldType::Path,
            default: serde_json::json!(""),
            options: vec![],
            required: false,
        },
        ConfigField {
            key: "index_path".into(),
            label: "Index Path".into(),
            description: "Path to the .index file for the voice model".into(),
            field_type: ConfigFieldType::Path,
            default: serde_json::json!(""),
            options: vec![],
            required: false,
        },
        ConfigField {
            key: "pitch_shift".into(),
            label: "Pitch Shift".into(),
            description: "Semitones to shift pitch (0 = no change, +12 = one octave up)".into(),
            field_type: ConfigFieldType::Number,
            default: serde_json::json!(0),
            options: vec![],
            required: false,
        },
    ]
}

fn voice_runtime_fields() -> Vec<ConfigField> {
    vec![
        ConfigField {
            key: "tts_voice".into(),
            label: "TTS Voice".into(),
            description: "Edge TTS voice identifier (e.g. en-US-AriaNeural)".into(),
            field_type: ConfigFieldType::Text,
            default: serde_json::json!("en-US-AriaNeural"),
            options: vec![],
            required: false,
        },
        ConfigField {
            key: "whisper_model".into(),
            label: "Whisper Model".into(),
            description: "Whisper model size for speech recognition".into(),
            field_type: ConfigFieldType::Select,
            default: serde_json::json!("base"),
            options: vec![
                "tiny".into(), "base".into(), "small".into(),
                "medium".into(), "large".into(),
            ],
            required: false,
        },
    ]
}

fn ffmpeg_fields() -> Vec<ConfigField> {
    vec![
        ConfigField {
            key: "ffmpeg_path".into(),
            label: "FFmpeg Path".into(),
            description: "Override the system FFmpeg binary path (leave empty for auto-detect)".into(),
            field_type: ConfigFieldType::Path,
            default: serde_json::json!(""),
            options: vec![],
            required: false,
        },
    ]
}

fn python_fields() -> Vec<ConfigField> {
    vec![
        ConfigField {
            key: "python_path".into(),
            label: "Python Path".into(),
            description: "Override the Python binary path (leave empty for auto-detect)".into(),
            field_type: ConfigFieldType::Path,
            default: serde_json::json!(""),
            options: vec![],
            required: false,
        },
    ]
}

fn uv_fields() -> Vec<ConfigField> {
    vec![
        ConfigField {
            key: "uv_path".into(),
            label: "uv Path".into(),
            description: "Override the uv binary path (leave empty for bundled/auto-detect)".into(),
            field_type: ConfigFieldType::Path,
            default: serde_json::json!(""),
            options: vec![],
            required: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let store = ServiceConfigStore::new(dir.path());

        let mut values = HashMap::new();
        values.insert("port".into(), serde_json::json!(1933));
        values.insert("embedding_provider".into(), serde_json::json!("openai"));

        store.save("openviking", &values).unwrap();

        let loaded = store.load("openviking").unwrap();
        assert_eq!(loaded["port"], 1933);
        assert_eq!(loaded["embedding_provider"], "openai");
    }

    #[test]
    fn test_get_service_config_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let config = get_service_config("openviking", dir.path()).unwrap();
        assert_eq!(config.service_id, "openviking");
        assert!(!config.fields.is_empty());
        assert_eq!(config.values["embedding_provider"], "litellm");
    }

    #[test]
    fn test_get_service_config_with_overrides() {
        let dir = tempfile::tempdir().unwrap();
        let mut values = HashMap::new();
        values.insert("embedding_provider".into(), serde_json::json!("openai"));
        values.insert("embedding_model".into(), serde_json::json!("text-embedding-3-large"));
        set_service_config("openviking", values, dir.path()).unwrap();

        let config = get_service_config("openviking", dir.path()).unwrap();
        assert_eq!(config.values["embedding_provider"], "openai");
        assert_eq!(config.values["embedding_model"], "text-embedding-3-large");
        // Non-overridden fields should still have defaults
        assert_eq!(config.values["port"], 1933);
    }

    #[test]
    fn test_unknown_service_returns_empty_fields() {
        let dir = tempfile::tempdir().unwrap();
        let config = get_service_config("nonexistent", dir.path()).unwrap();
        assert!(config.fields.is_empty());
        assert!(config.values.is_empty());
    }

    #[test]
    fn test_all_services_have_fields() {
        let dir = tempfile::tempdir().unwrap();
        for id in &["openviking", "rvc", "voice-runtime", "ffmpeg", "python", "uv"] {
            let config = get_service_config(id, dir.path()).unwrap();
            assert!(!config.fields.is_empty(), "service {} should have config fields", id);
        }
    }
}
