use serde::{Deserialize, Serialize};

/// App runtime type — determines which environment manager to use.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AppRuntime {
    /// Python app — uses uv for venv + pip deps.
    Python,
    /// Node.js app — uses npm/pnpm/yarn for deps.
    Node,
    /// Custom/binary app — no automatic environment setup.
    Custom,
}

impl Default for AppRuntime {
    fn default() -> Self {
        Self::Python
    }
}

/// Node.js package manager preference (optional override in manifest).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ManifestPackageManager {
    Npm,
    Pnpm,
    Yarn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,

    /// Runtime type. Defaults to "python" for backwards compatibility.
    #[serde(default)]
    pub runtime: AppRuntime,

    /// Python version (only used when runtime == "python").
    #[serde(default = "default_python_version")]
    pub python_version: String,
    /// Node.js version constraint (only used when runtime == "node").
    /// e.g. ">=18", "20", "lts". Currently informational.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_version: Option<String>,
    /// Explicit package manager for Node.js apps.
    /// If omitted, auto-detected from lockfile.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_manager: Option<ManifestPackageManager>,

    pub needs_gpu: bool,
    /// Python pip dependencies (runtime == python).
    pub pip_deps: Vec<String>,
    pub launch_cmd: String,
    pub port: u16,
    #[serde(default)]
    pub env: Vec<(String, String)>,
    pub disk_size: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_python_version() -> String {
    "3".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "state")]
pub enum AppStatus {
    NotInstalled,
    Installing,
    Installed,
    Running { pid: u32, port: u16 },
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledApp {
    pub manifest: AppManifest,
    pub status: AppStatus,
    pub workspace: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_run: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InstallRequest {
    pub manifest: AppManifest,
}

/// Source type for store uploads
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Folder,
    Zip,
    GitUrl,
}

/// Request payload for uploading an app to the store
#[derive(Debug, Deserialize)]
pub struct StoreUploadRequest {
    pub source_type: SourceType,
    /// Local path — required for Folder and Zip sources
    #[serde(default)]
    pub source_path: Option<String>,
    /// Git clone URL — required for GitUrl source
    #[serde(default)]
    pub git_url: Option<String>,
}

/// Structured validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

/// Result of a store upload attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreUploadResult {
    pub accepted: bool,
    pub app_id: Option<String>,
    pub app_name: Option<String>,
    pub validation_errors: Vec<ValidationError>,
    pub install_log: Vec<String>,
}

impl AppManifest {
    pub fn sample_node_fullstack() -> Self {
        Self {
            id: "sample-node".into(),
            name: "Node.js Fullstack Counter".into(),
            description: "A fullstack React + Express counter app to demonstrate Node.js support in AI Launcher".into(),
            author: "ai-launcher".into(),
            repo: None,
            runtime: AppRuntime::Node,
            python_version: "3".into(),
            node_version: Some(">=18".into()),
            package_manager: None, // auto-detect
            needs_gpu: false,
            pip_deps: vec![],
            launch_cmd: "npm start".into(),
            port: 3000,
            env: vec![],
            disk_size: "~100MB".into(),
            tags: vec!["node".into(), "express".into(), "react".into(), "fullstack".into(), "demo".into()],
        }
    }

    pub fn sample_counter() -> Self {
        Self {
            id: "sample-gradio".into(),
            name: "Sample Counter".into(),
            description: "A simple Gradio counter app to test the sandbox launcher".into(),
            author: "ai-launcher".into(),
            repo: None,
            runtime: AppRuntime::Python,
            python_version: "3".into(),
            node_version: None,
            package_manager: None,
            needs_gpu: false,
            pip_deps: vec!["gradio".into()],
            launch_cmd: "python3 app.py".into(),
            port: 7860,
            env: vec![],
            disk_size: "~200MB".into(),
            tags: vec!["demo".into(), "gradio".into(), "counter".into()],
        }
    }

    pub fn stable_diffusion() -> Self {
        Self {
            id: "stable-diffusion-webui".into(),
            name: "Stable Diffusion WebUI".into(),
            description: "AUTOMATIC1111 Stable Diffusion web interface".into(),
            author: "AUTOMATIC1111".into(),
            repo: Some("https://github.com/AUTOMATIC1111/stable-diffusion-webui.git".into()),
            runtime: AppRuntime::Python,
            python_version: "3.10".into(),
            node_version: None,
            package_manager: None,
            needs_gpu: true,
            pip_deps: vec![],
            launch_cmd: "python3 launch.py --listen --port 7860".into(),
            port: 7860,
            env: vec![],
            disk_size: "~12GB".into(),
            tags: vec!["image-generation".into(), "gpu".into()],
        }
    }

    pub fn ollama() -> Self {
        Self {
            id: "ollama".into(),
            name: "Ollama".into(),
            description: "Run large language models locally".into(),
            author: "ollama".into(),
            repo: Some("https://github.com/ollama/ollama.git".into()),
            runtime: AppRuntime::Custom,
            python_version: "3".into(),
            node_version: None,
            package_manager: None,
            needs_gpu: false,
            pip_deps: vec![],
            launch_cmd: if cfg!(windows) {
                "ollama.exe serve".into()
            } else {
                "ollama serve".into()
            },
            port: 11434,
            env: vec![],
            disk_size: "~1.2GB + models".into(),
            tags: vec!["llm".into(), "text-generation".into()],
        }
    }

    /// Get the pip command for this platform
    pub fn pip_cmd() -> &'static str {
        if cfg!(windows) {
            "pip"
        } else {
            "pip3"
        }
    }

    /// Get the python command for this platform
    pub fn python_cmd() -> &'static str {
        if cfg!(windows) {
            "python"
        } else {
            "python3"
        }
    }

    /// Whether this is a Node.js app.
    pub fn is_node(&self) -> bool {
        self.runtime == AppRuntime::Node
    }

    /// Whether this is a Python app.
    pub fn is_python(&self) -> bool {
        self.runtime == AppRuntime::Python
    }
}
