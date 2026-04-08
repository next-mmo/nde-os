use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::shield::browser::BrowserEngine;

// ─── Input Schema ──────────────────────────────────────────────────

/// JSON Schema-compatible property type for actor inputs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PropertyType {
    String,
    Integer,
    Number,
    Boolean,
    Array,
    Object,
}

/// A single input property definition (subset of JSON Schema).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputProperty {
    pub title: String,
    #[serde(rename = "type")]
    pub property_type: PropertyType,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    /// UI editor hint: "requestListSources", "proxy", "textfield", "json", etc.
    #[serde(default)]
    pub editor: Option<String>,
    /// Enum constraints for string types.
    #[serde(default, rename = "enum")]
    pub enum_values: Option<Vec<String>>,
    /// Minimum value for integer/number types.
    #[serde(default)]
    pub minimum: Option<f64>,
    /// Maximum value for integer/number types.
    #[serde(default)]
    pub maximum: Option<f64>,
    /// Pre-filled example value for UI.
    #[serde(default)]
    pub prefill: Option<serde_json::Value>,
}

/// Input schema for an actor — defines accepted parameters.
/// Compatible with Apify's `input_schema.json` format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputSchema {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Always "object" at the top level.
    #[serde(rename = "type", default = "default_object_type")]
    pub schema_type: String,
    #[serde(default = "default_schema_version")]
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    #[serde(default)]
    pub properties: HashMap<String, InputProperty>,
    #[serde(default)]
    pub required: Vec<String>,
}

fn default_object_type() -> String {
    "object".to_string()
}

fn default_schema_version() -> u32 {
    1
}

impl InputSchema {
    /// Validate an input JSON value against this schema.
    /// Returns Ok(()) if valid, Err with details if not.
    pub fn validate(&self, input: &serde_json::Value) -> Result<()> {
        let obj = input
            .as_object()
            .context("Actor input must be a JSON object")?;

        // Check required fields
        for field in &self.required {
            if !obj.contains_key(field) {
                anyhow::bail!("Missing required input field: '{}'", field);
            }
        }

        // Type-check provided fields
        for (key, value) in obj {
            if let Some(prop) = self.properties.get(key) {
                validate_property_type(key, value, &prop.property_type)?;

                // Range checks for numeric types
                if let Some(min) = prop.minimum {
                    if let Some(n) = value.as_f64() {
                        if n < min {
                            anyhow::bail!("Input '{}' value {} is below minimum {}", key, n, min);
                        }
                    }
                }
                if let Some(max) = prop.maximum {
                    if let Some(n) = value.as_f64() {
                        if n > max {
                            anyhow::bail!("Input '{}' value {} is above maximum {}", key, n, max);
                        }
                    }
                }

                // Enum check for strings
                if let Some(ref allowed) = prop.enum_values {
                    if let Some(s) = value.as_str() {
                        if !allowed.iter().any(|a| a == s) {
                            anyhow::bail!(
                                "Input '{}' value '{}' is not in allowed values: {:?}",
                                key,
                                s,
                                allowed
                            );
                        }
                    }
                }
            }
            // Extra fields are allowed (lenient validation).
        }

        Ok(())
    }

    /// Apply default values to input, filling missing fields.
    pub fn apply_defaults(&self, input: &mut serde_json::Value) {
        let obj = match input.as_object_mut() {
            Some(o) => o,
            None => return,
        };

        for (key, prop) in &self.properties {
            if !obj.contains_key(key) {
                if let Some(ref default) = prop.default {
                    obj.insert(key.clone(), default.clone());
                }
            }
        }
    }
}

fn validate_property_type(
    key: &str,
    value: &serde_json::Value,
    expected: &PropertyType,
) -> Result<()> {
    let valid = match expected {
        PropertyType::String => value.is_string(),
        PropertyType::Integer => value.is_i64() || value.is_u64(),
        PropertyType::Number => value.is_number(),
        PropertyType::Boolean => value.is_boolean(),
        PropertyType::Array => value.is_array(),
        PropertyType::Object => value.is_object(),
    };

    if !valid && !value.is_null() {
        anyhow::bail!(
            "Input '{}' expected type {:?}, got {}",
            key,
            expected,
            value_type_name(value)
        );
    }

    Ok(())
}

fn value_type_name(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

// ─── Actor Runtime Config ──────────────────────────────────────────

/// Runtime configuration — what language and entry point the actor uses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "language", rename_all = "lowercase")]
pub enum ActorRuntime {
    Python {
        #[serde(default = "default_python_version")]
        version: String,
        #[serde(default)]
        pip_deps: Vec<String>,
        #[serde(default = "default_python_entry")]
        entry: String,
    },
    Node {
        #[serde(default = "default_node_version")]
        version: String,
        #[serde(default)]
        npm_deps: Vec<String>,
        #[serde(default = "default_node_entry")]
        entry: String,
    },
}

fn default_python_version() -> String {
    "3.11".to_string()
}
fn default_python_entry() -> String {
    "src/main.py".to_string()
}
fn default_node_version() -> String {
    "20".to_string()
}
fn default_node_entry() -> String {
    "src/main.js".to_string()
}

impl ActorRuntime {
    pub fn entry(&self) -> &str {
        match self {
            ActorRuntime::Python { entry, .. } => entry,
            ActorRuntime::Node { entry, .. } => entry,
        }
    }

    pub fn language_name(&self) -> &'static str {
        match self {
            ActorRuntime::Python { .. } => "python",
            ActorRuntime::Node { .. } => "node",
        }
    }

    pub fn deps(&self) -> Vec<String> {
        match self {
            ActorRuntime::Python { pip_deps, .. } => pip_deps.clone(),
            ActorRuntime::Node { npm_deps, .. } => npm_deps.clone(),
        }
    }
}

// ─── Browser Config ────────────────────────────────────────────────

/// Browser configuration for the actor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    /// Browser engine. None = auto-detect based on runtime env.
    #[serde(default)]
    pub engine: Option<BrowserEngine>,
    /// Run headless (no visible window).
    #[serde(default = "default_true")]
    pub headless: bool,
    /// Use the linked Shield profile's proxy settings.
    #[serde(default)]
    pub proxy_from_profile: bool,
    /// Link to a specific Shield profile ID (preserves fingerprint, cookies).
    #[serde(default)]
    pub profile_id: Option<String>,
    /// Maximum concurrent pages.
    #[serde(default = "default_pages")]
    pub pages: u32,
}

fn default_true() -> bool {
    true
}
fn default_pages() -> u32 {
    1
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            engine: None,
            headless: true,
            proxy_from_profile: false,
            profile_id: None,
            pages: 1,
        }
    }
}

// ─── Output Config ─────────────────────────────────────────────────

/// Configures how actor output is stored and formatted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Dataset format for export.
    #[serde(default = "default_output_format")]
    pub format: OutputFormat,
    /// Maximum dataset size in items (0 = unlimited).
    #[serde(default)]
    pub max_items: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Json,
    Csv,
    Jsonl,
}

fn default_output_format() -> OutputFormat {
    OutputFormat::Jsonl
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            format: OutputFormat::Jsonl,
            max_items: 0,
        }
    }
}

// ─── Apify Compatibility ───────────────────────────────────────────

/// Apify-specific configuration for cloud deployment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApifyConfig {
    /// Override actor name on Apify (defaults to manifest id).
    #[serde(default)]
    pub actor_name: Option<String>,
    /// Default memory allocation in MB.
    #[serde(default = "default_apify_memory")]
    pub default_memory_mb: u32,
    /// Docker build tag.
    #[serde(default = "default_build_tag")]
    pub build_tag: String,
}

fn default_apify_memory() -> u32 {
    2048
}
fn default_build_tag() -> String {
    "latest".to_string()
}

impl Default for ApifyConfig {
    fn default() -> Self {
        Self {
            actor_name: None,
            default_memory_mb: 2048,
            build_tag: "latest".to_string(),
        }
    }
}

// ─── Actor Manifest ────────────────────────────────────────────────

/// NDE-OS Actor Manifest (`nde_actor.json`).
///
/// Defines everything about an actor: identity, input schema, runtime,
/// browser config, output format, and Apify compatibility settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub icon: Option<String>,

    /// Input schema — defines accepted parameters.
    pub input_schema: InputSchema,

    /// Runtime configuration.
    pub runtime: ActorRuntime,

    /// Browser / Shield integration settings.
    #[serde(default)]
    pub browser: BrowserConfig,

    /// Output storage configuration.
    #[serde(default)]
    pub output: OutputConfig,

    /// Apify cloud deployment configuration.
    #[serde(default)]
    pub apify: Option<ApifyConfig>,

    /// Epoch timestamp of creation.
    #[serde(default)]
    pub created_at: u64,
}

impl ActorManifest {
    /// Load a manifest from a directory containing `nde_actor.json`.
    pub fn load(actor_dir: &Path) -> Result<Self> {
        let manifest_path = actor_dir.join("nde_actor.json");
        let content = std::fs::read_to_string(&manifest_path).with_context(|| {
            format!("Failed to read actor manifest: {}", manifest_path.display())
        })?;
        let manifest: Self = serde_json::from_str(&content).with_context(|| {
            format!(
                "Failed to parse actor manifest: {}",
                manifest_path.display()
            )
        })?;
        Ok(manifest)
    }

    /// Save the manifest to `nde_actor.json` in the given directory.
    pub fn save(&self, actor_dir: &Path) -> Result<()> {
        let manifest_path = actor_dir.join("nde_actor.json");
        let json =
            serde_json::to_string_pretty(self).context("Failed to serialize actor manifest")?;
        std::fs::write(&manifest_path, json).with_context(|| {
            format!(
                "Failed to write actor manifest: {}",
                manifest_path.display()
            )
        })?;
        Ok(())
    }

    /// Generate Apify-compatible `actor.json` content.
    pub fn to_apify_actor_json(&self) -> serde_json::Value {
        let actor_name = self
            .apify
            .as_ref()
            .and_then(|a| a.actor_name.clone())
            .unwrap_or_else(|| self.id.clone());

        let memory = self
            .apify
            .as_ref()
            .map(|a| a.default_memory_mb)
            .unwrap_or(2048);

        serde_json::json!({
            "actorSpecification": 1,
            "name": actor_name,
            "title": self.name,
            "description": self.description,
            "version": self.version,
            "input": "./input_schema.json",
            "dockerfile": "./Dockerfile",
            "defaultMemoryMbytes": memory,
        })
    }

    /// Generate Apify-compatible `input_schema.json` content.
    pub fn to_apify_input_schema(&self) -> serde_json::Value {
        serde_json::to_value(&self.input_schema).unwrap_or_else(|_| serde_json::json!({}))
    }

    /// Generate a Dockerfile for Apify deployment.
    pub fn to_dockerfile(&self) -> String {
        match &self.runtime {
            ActorRuntime::Python { version, .. } => {
                format!(
                    r#"FROM apify/actor-python:{version}

COPY requirements.txt ./
RUN pip install --no-cache-dir -r requirements.txt

COPY . ./

CMD ["python", "-m", "src.main"]
"#,
                    version = version,
                )
            }
            ActorRuntime::Node { version, .. } => {
                format!(
                    r#"FROM apify/actor-node:{version}

COPY package*.json ./
RUN npm ci --omit=dev --omit=optional

COPY . ./

CMD ["node", "src/main.js"]
"#,
                    version = version,
                )
            }
        }
    }
}

// ─── Actor Manager ─────────────────────────────────────────────────

/// Manages installed actors on disk.
pub struct ActorManager {
    actors_dir: PathBuf,
}

impl ActorManager {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            actors_dir: base_dir.join("actors"),
        }
    }

    pub fn actors_dir(&self) -> &Path {
        &self.actors_dir
    }

    /// List all installed actors by reading their manifests.
    pub fn list_actors(&self) -> Result<Vec<ActorManifest>> {
        if !self.actors_dir.exists() {
            return Ok(Vec::new());
        }

        let mut actors = Vec::new();
        for entry in
            std::fs::read_dir(&self.actors_dir).context("Failed to read actors directory")?
        {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                match ActorManifest::load(&path) {
                    Ok(manifest) => actors.push(manifest),
                    Err(e) => {
                        tracing::warn!("Skipping actor dir {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(actors)
    }

    /// Get a specific actor by ID.
    pub fn get_actor(&self, id: &str) -> Result<ActorManifest> {
        let actor_dir = self.actors_dir.join(id);
        ActorManifest::load(&actor_dir)
    }

    /// Get the directory path for an actor.
    pub fn actor_dir(&self, id: &str) -> PathBuf {
        self.actors_dir.join(id)
    }

    /// Delete an actor and all its data.
    pub fn delete_actor(&self, id: &str) -> Result<()> {
        let actor_dir = self.actors_dir.join(id);
        if !actor_dir.exists() {
            anyhow::bail!("Actor '{}' not found", id);
        }
        std::fs::remove_dir_all(&actor_dir).with_context(|| {
            format!("Failed to delete actor directory: {}", actor_dir.display())
        })?;
        Ok(())
    }

    /// Generate Apify-compatible files in the actor directory.
    pub fn export_apify(&self, id: &str) -> Result<PathBuf> {
        let actor_dir = self.actors_dir.join(id);
        let manifest = ActorManifest::load(&actor_dir)?;

        // Create .actor/ directory
        let apify_dir = actor_dir.join(".actor");
        std::fs::create_dir_all(&apify_dir).context("Failed to create .actor directory")?;

        // Write actor.json
        let actor_json = manifest.to_apify_actor_json();
        let actor_json_str = serde_json::to_string_pretty(&actor_json)?;
        std::fs::write(apify_dir.join("actor.json"), actor_json_str)?;

        // Write input_schema.json
        let input_schema = manifest.to_apify_input_schema();
        let input_schema_str = serde_json::to_string_pretty(&input_schema)?;
        std::fs::write(apify_dir.join("input_schema.json"), input_schema_str)?;

        // Write Dockerfile
        let dockerfile = manifest.to_dockerfile();
        std::fs::write(actor_dir.join("Dockerfile"), dockerfile)?;

        tracing::info!("Exported Apify-compatible files for actor '{}'", id);
        Ok(actor_dir)
    }
}

// ─── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_input_schema() -> InputSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "startUrls".to_string(),
            InputProperty {
                title: "Start URLs".to_string(),
                property_type: PropertyType::Array,
                description: Some("URLs to scrape".to_string()),
                default: None,
                editor: Some("requestListSources".to_string()),
                enum_values: None,
                minimum: None,
                maximum: None,
                prefill: None,
            },
        );
        properties.insert(
            "maxPages".to_string(),
            InputProperty {
                title: "Max Pages".to_string(),
                property_type: PropertyType::Integer,
                description: Some("Maximum pages to scrape".to_string()),
                default: Some(serde_json::json!(10)),
                editor: None,
                enum_values: None,
                minimum: Some(1.0),
                maximum: Some(10000.0),
                prefill: None,
            },
        );

        InputSchema {
            title: "Scraper Input".to_string(),
            description: None,
            schema_type: "object".to_string(),
            schema_version: 1,
            properties,
            required: vec!["startUrls".to_string()],
        }
    }

    #[test]
    fn test_input_validation_passes() {
        let schema = sample_input_schema();
        let input = serde_json::json!({
            "startUrls": [{"url": "https://example.com"}],
            "maxPages": 5,
        });
        assert!(schema.validate(&input).is_ok());
    }

    #[test]
    fn test_input_validation_missing_required() {
        let schema = sample_input_schema();
        let input = serde_json::json!({
            "maxPages": 5,
        });
        let result = schema.validate(&input);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("startUrls"));
    }

    #[test]
    fn test_input_validation_wrong_type() {
        let schema = sample_input_schema();
        let input = serde_json::json!({
            "startUrls": "not an array",
            "maxPages": 5,
        });
        let result = schema.validate(&input);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expected type"));
    }

    #[test]
    fn test_input_validation_below_minimum() {
        let schema = sample_input_schema();
        let input = serde_json::json!({
            "startUrls": [{"url": "https://example.com"}],
            "maxPages": 0,
        });
        let result = schema.validate(&input);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("below minimum"));
    }

    #[test]
    fn test_input_validation_above_maximum() {
        let schema = sample_input_schema();
        let input = serde_json::json!({
            "startUrls": [{"url": "https://example.com"}],
            "maxPages": 99999,
        });
        let result = schema.validate(&input);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("above maximum"));
    }

    #[test]
    fn test_apply_defaults() {
        let schema = sample_input_schema();
        let mut input = serde_json::json!({
            "startUrls": [{"url": "https://example.com"}],
        });
        schema.apply_defaults(&mut input);
        assert_eq!(input["maxPages"], 10);
    }

    #[test]
    fn test_apply_defaults_does_not_overwrite() {
        let schema = sample_input_schema();
        let mut input = serde_json::json!({
            "startUrls": [{"url": "https://example.com"}],
            "maxPages": 42,
        });
        schema.apply_defaults(&mut input);
        assert_eq!(input["maxPages"], 42);
    }

    #[test]
    fn test_input_must_be_object() {
        let schema = sample_input_schema();
        let input = serde_json::json!("not an object");
        assert!(schema.validate(&input).is_err());
    }

    #[test]
    fn test_manifest_serialization_roundtrip() {
        let manifest = ActorManifest {
            id: "test-scraper".to_string(),
            name: "Test Scraper".to_string(),
            version: "1.0.0".to_string(),
            description: "A test scraper".to_string(),
            author: Some("NDE-OS".to_string()),
            tags: vec!["scraping".to_string()],
            icon: Some("🕷️".to_string()),
            input_schema: sample_input_schema(),
            runtime: ActorRuntime::Python {
                version: "3.11".to_string(),
                pip_deps: vec!["playwright".to_string()],
                entry: "src/main.py".to_string(),
            },
            browser: BrowserConfig::default(),
            output: OutputConfig::default(),
            apify: Some(ApifyConfig::default()),
            created_at: 1000,
        };

        let json = serde_json::to_string_pretty(&manifest).unwrap();
        let parsed: ActorManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "test-scraper");
        assert_eq!(parsed.runtime.language_name(), "python");
    }

    #[test]
    fn test_apify_actor_json_generation() {
        let manifest = ActorManifest {
            id: "my-actor".to_string(),
            name: "My Actor".to_string(),
            version: "1.0.0".to_string(),
            description: "Does stuff".to_string(),
            author: None,
            tags: vec![],
            icon: None,
            input_schema: InputSchema {
                title: "Input".to_string(),
                description: None,
                schema_type: "object".to_string(),
                schema_version: 1,
                properties: HashMap::new(),
                required: vec![],
            },
            runtime: ActorRuntime::Python {
                version: "3.11".to_string(),
                pip_deps: vec![],
                entry: "src/main.py".to_string(),
            },
            browser: BrowserConfig::default(),
            output: OutputConfig::default(),
            apify: None,
            created_at: 0,
        };

        let actor_json = manifest.to_apify_actor_json();
        assert_eq!(actor_json["name"], "my-actor");
        assert_eq!(actor_json["actorSpecification"], 1);
        assert_eq!(actor_json["input"], "./input_schema.json");
    }

    #[test]
    fn test_dockerfile_generation_python() {
        let manifest = ActorManifest {
            id: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            description: "test".to_string(),
            author: None,
            tags: vec![],
            icon: None,
            input_schema: InputSchema {
                title: "Input".to_string(),
                description: None,
                schema_type: "object".to_string(),
                schema_version: 1,
                properties: HashMap::new(),
                required: vec![],
            },
            runtime: ActorRuntime::Python {
                version: "3.11".to_string(),
                pip_deps: vec![],
                entry: "src/main.py".to_string(),
            },
            browser: BrowserConfig::default(),
            output: OutputConfig::default(),
            apify: None,
            created_at: 0,
        };

        let dockerfile = manifest.to_dockerfile();
        assert!(dockerfile.contains("FROM apify/actor-python:3.11"));
        assert!(dockerfile.contains("requirements.txt"));
    }

    #[test]
    fn test_enum_validation() {
        let mut properties = HashMap::new();
        properties.insert(
            "browser".to_string(),
            InputProperty {
                title: "Browser".to_string(),
                property_type: PropertyType::String,
                description: None,
                default: None,
                editor: None,
                enum_values: Some(vec!["chromium".to_string(), "firefox".to_string()]),
                minimum: None,
                maximum: None,
                prefill: None,
            },
        );

        let schema = InputSchema {
            title: "Test".to_string(),
            description: None,
            schema_type: "object".to_string(),
            schema_version: 1,
            properties,
            required: vec![],
        };

        let valid = serde_json::json!({"browser": "chromium"});
        assert!(schema.validate(&valid).is_ok());

        let invalid = serde_json::json!({"browser": "safari"});
        assert!(schema.validate(&invalid).is_err());
    }

    #[test]
    fn test_actor_manager_empty() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mgr = ActorManager::new(tmp.path());
        let actors = mgr.list_actors().unwrap();
        assert!(actors.is_empty());
    }
}
