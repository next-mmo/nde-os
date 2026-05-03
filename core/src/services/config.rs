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
        let contents =
            std::fs::read_to_string(&self.path).context("Failed to read service_configs.json")?;
        let data =
            serde_json::from_str(&contents).context("Failed to parse service_configs.json")?;
        Ok(data)
    }

    /// Load config values for a single service.
    pub fn load(&self, service_id: &str) -> Result<HashMap<String, serde_json::Value>> {
        let all = self.load_all()?;
        Ok(all.get(service_id).cloned().unwrap_or_default())
    }

    /// Save config values for a single service (merges into existing file).
    pub fn save(
        &self,
        service_id: &str,
        values: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let mut all = self.load_all().unwrap_or_default();
        all.insert(service_id.to_string(), values.clone());
        let json =
            serde_json::to_string_pretty(&all).context("Failed to serialize service configs")?;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, json).context("Failed to write service_configs.json")?;
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
        "rvc" => rvc_fields(),
        "voice-runtime" => voice_runtime_fields(),
        "ffmpeg" => ffmpeg_fields(),
        "python" => python_fields(),
        "uv" => uv_fields(),
        _ => vec![],
    }
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
                "tiny".into(),
                "base".into(),
                "small".into(),
                "medium".into(),
                "large".into(),
            ],
            required: false,
        },
    ]
}

fn ffmpeg_fields() -> Vec<ConfigField> {
    vec![ConfigField {
        key: "ffmpeg_path".into(),
        label: "FFmpeg Path".into(),
        description: "Override the system FFmpeg binary path (leave empty for auto-detect)".into(),
        field_type: ConfigFieldType::Path,
        default: serde_json::json!(""),
        options: vec![],
        required: false,
    }]
}

fn python_fields() -> Vec<ConfigField> {
    vec![ConfigField {
        key: "python_path".into(),
        label: "Python Path".into(),
        description: "Override the Python binary path (leave empty for auto-detect)".into(),
        field_type: ConfigFieldType::Path,
        default: serde_json::json!(""),
        options: vec![],
        required: false,
    }]
}

fn uv_fields() -> Vec<ConfigField> {
    vec![ConfigField {
        key: "uv_path".into(),
        label: "uv Path".into(),
        description: "Override the uv binary path (leave empty for bundled/auto-detect)".into(),
        field_type: ConfigFieldType::Path,
        default: serde_json::json!(""),
        options: vec![],
        required: false,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let store = ServiceConfigStore::new(dir.path());

        let mut values = HashMap::new();
        values.insert("model_path".into(), serde_json::json!("/tmp/model.pth"));
        values.insert("pitch_shift".into(), serde_json::json!(12));

        store.save("rvc", &values).unwrap();

        let loaded = store.load("rvc").unwrap();
        assert_eq!(loaded["model_path"], "/tmp/model.pth");
        assert_eq!(loaded["pitch_shift"], 12);
    }

    #[test]
    fn test_get_service_config_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let config = get_service_config("rvc", dir.path()).unwrap();
        assert_eq!(config.service_id, "rvc");
        assert!(!config.fields.is_empty());
        assert_eq!(config.values["pitch_shift"], 0);
    }

    #[test]
    fn test_get_service_config_with_overrides() {
        let dir = tempfile::tempdir().unwrap();
        let mut values = HashMap::new();
        values.insert("model_path".into(), serde_json::json!("/tmp/model.pth"));
        values.insert(
            "pitch_shift".into(),
            serde_json::json!(12),
        );
        set_service_config("rvc", values, dir.path()).unwrap();

        let config = get_service_config("rvc", dir.path()).unwrap();
        assert_eq!(config.values["model_path"], "/tmp/model.pth");
        assert_eq!(config.values["pitch_shift"], 12);
        // Non-overridden fields should still have defaults
        assert_eq!(config.values["index_path"], "");
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
        for id in &[
            "rvc",
            "voice-runtime",
            "ffmpeg",
            "python",
            "uv",
        ] {
            let config = get_service_config(id, dir.path()).unwrap();
            assert!(
                !config.fields.is_empty(),
                "service {} should have config fields",
                id
            );
        }
    }
}
