use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    pub python_version: String,
    pub needs_gpu: bool,
    pub pip_deps: Vec<String>,
    pub launch_cmd: String,
    pub port: u16,
    #[serde(default)]
    pub env: Vec<(String, String)>,
    pub disk_size: String,
    #[serde(default)]
    pub tags: Vec<String>,
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

impl AppManifest {
    pub fn sample_node_fullstack() -> Self {
        Self {
            id: "sample-node-fullstack".into(),
            name: "Node.js Fullstack Counter".into(),
            description: "A fullstack React + Express counter app to demonstrate Node.js support in AI Launcher".into(),
            author: "ai-launcher".into(),
            repo: None,
            python_version: "3".into(),
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
            python_version: "3".into(),
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
            python_version: "3.10".into(),
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
            python_version: "3".into(),
            needs_gpu: false,
            pip_deps: vec![],
            launch_cmd: if cfg!(windows) { "ollama.exe serve".into() } else { "ollama serve".into() },
            port: 11434,
            env: vec![],
            disk_size: "~1.2GB + models".into(),
            tags: vec!["llm".into(), "text-generation".into()],
        }
    }

    /// Get the pip command for this platform
    pub fn pip_cmd() -> &'static str {
        if cfg!(windows) { "pip" } else { "pip3" }
    }

    /// Get the python command for this platform
    pub fn python_cmd() -> &'static str {
        if cfg!(windows) { "python" } else { "python3" }
    }
}
