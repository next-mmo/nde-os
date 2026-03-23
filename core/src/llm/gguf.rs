/// GGUF LLM Provider — auto-bootstrapping llama.cpp subprocess.
///
/// Downloads pre-built llama-server binary + default model on first use.
/// Lifecycle: bootstrap binary -> download model -> launch server -> OpenAI-compat API.

use super::{LlmProvider, LlmResponse, Message, StopReason, ToolCall, ToolDef, Usage};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Default small model for first-run.
const DEFAULT_MODEL_URL: &str =
    "https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf";
const DEFAULT_MODEL_FILENAME: &str = "tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf";

/// llama.cpp release to auto-download.
const LLAMA_CPP_VERSION: &str = "b4679";
const DEFAULT_PORT: u16 = 8090;

pub struct GgufProvider {
    client: reqwest::Client,
    base_url: String,
    data_dir: PathBuf,
    model_path: PathBuf,
    model_name: String,
    port: u16,
    server_process: Arc<Mutex<Option<tokio::process::Child>>>,
}

impl GgufProvider {
    pub fn new(data_dir: &Path, model_path: Option<&str>, port: Option<u16>) -> Self {
        let port = port.unwrap_or(DEFAULT_PORT);
        let model_path = match model_path {
            Some(p) => PathBuf::from(p),
            None => data_dir.join("models").join(DEFAULT_MODEL_FILENAME),
        };
        let model_name = model_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "gguf-model".to_string());

        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            base_url: format!("http://127.0.0.1:{}", port),
            data_dir: data_dir.to_path_buf(),
            model_path,
            model_name,
            port,
            server_process: Arc::new(Mutex::new(None)),
        }
    }

    /// Path to the llama-server binary inside our data dir.
    fn server_bin_path(&self) -> PathBuf {
        let bin_dir = self.data_dir.join("bin");
        if cfg!(windows) {
            bin_dir.join("llama-server.exe")
        } else {
            bin_dir.join("llama-server")
        }
    }

    /// Auto-download prebuilt llama-server if not present.
    pub async fn ensure_server_binary(&self) -> Result<PathBuf> {
        let bin_path = self.server_bin_path();
        if bin_path.exists() {
            return Ok(bin_path);
        }

        // Also check system PATH first
        if let Some(system_bin) = Self::find_in_path() {
            return Ok(system_bin);
        }

        let bin_dir = self.data_dir.join("bin");
        std::fs::create_dir_all(&bin_dir).context("Failed to create bin directory")?;

        let (url, archive_name) = Self::release_url();
        tracing::info!(url = %url, "Downloading llama-server (first run)...");

        let resp = self.client.get(&url).send().await
            .context("Failed to download llama-server")?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to download llama-server: HTTP {} — you can manually install:\n\
                 • Windows: Download from https://github.com/ggerganov/llama.cpp/releases\n\
                 • macOS: brew install llama.cpp\n\
                 • Linux: apt install llama-cpp",
                resp.status()
            ));
        }

        let bytes = resp.bytes().await.context("Failed to read archive")?;
        Self::extract_server(&bytes, &archive_name, &bin_dir)?;

        if bin_path.exists() {
            // Make executable on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&bin_path, std::fs::Permissions::from_mode(0o755))?;
            }
            tracing::info!(path = %bin_path.display(), "llama-server installed");
            Ok(bin_path)
        } else {
            Err(anyhow::anyhow!(
                "llama-server binary not found after extraction at {}",
                bin_path.display()
            ))
        }
    }

    /// Find llama-server in system PATH.
    fn find_in_path() -> Option<PathBuf> {
        let cmd = if cfg!(windows) { "where" } else { "which" };
        std::process::Command::new(cmd)
            .arg("llama-server")
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    let s = String::from_utf8_lossy(&o.stdout);
                    let first = s.lines().next()?.trim();
                    if !first.is_empty() {
                        Some(PathBuf::from(first))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
    }

    /// Build the GitHub release URL for the current platform.
    fn release_url() -> (String, String) {
        let tag = LLAMA_CPP_VERSION;
        let (suffix, archive) = if cfg!(target_os = "windows") {
            if cfg!(target_arch = "x86_64") {
                ("win-avx2-x64.zip", "zip")
            } else {
                ("win-arm64.zip", "zip")
            }
        } else if cfg!(target_os = "macos") {
            ("macos-arm64.zip", "zip")
        } else {
            // Linux
            if cfg!(target_arch = "x86_64") {
                ("ubuntu-x64.zip", "zip")
            } else {
                ("ubuntu-arm64.zip", "zip")
            }
        };
        let url = format!(
            "https://github.com/ggerganov/llama.cpp/releases/download/{}/llama-{}-{}", 
            tag, tag, suffix
        );
        (url, archive.to_string())
    }

    /// Extract llama-server from the downloaded archive.
    fn extract_server(bytes: &[u8], _archive_type: &str, dest: &Path) -> Result<()> {
        // llama.cpp releases are ZIP files on all platforms
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor)
            .context("Failed to open zip archive")?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).context("Failed to read zip entry")?;
            let name = file.name().to_string();

            // We only care about llama-server binary
            let is_server = if cfg!(windows) {
                name.contains("llama-server") && name.ends_with(".exe")
            } else {
                name.contains("llama-server") && !name.ends_with(".dll") && !name.ends_with(".so")
            };

            if !is_server {
                continue;
            }

            let out_name = if cfg!(windows) {
                "llama-server.exe"
            } else {
                "llama-server"
            };
            let out_path = dest.join(out_name);
            let mut out_file = std::fs::File::create(&out_path)
                .context("Failed to create llama-server file")?;
            std::io::copy(&mut file, &mut out_file)
                .context("Failed to write llama-server")?;

            tracing::info!(path = %out_path.display(), "Extracted llama-server");
            return Ok(());
        }

        // If no exact match, extract everything and hope for the best
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).context("zip entry")?;
            let name = file.name().to_string();
            if let Some(file_name) = PathBuf::from(&name).file_name() {
                let out_path = dest.join(file_name);
                if !file.is_dir() {
                    let mut out_file = std::fs::File::create(&out_path)?;
                    std::io::copy(&mut file, &mut out_file)?;
                }
            }
        }

        Ok(())
    }

    /// Ensure the model file is downloaded.
    pub async fn ensure_model(&self) -> Result<()> {
        if self.model_path.exists() {
            return Ok(());
        }
        if let Some(parent) = self.model_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create models directory")?;
        }

        tracing::info!(
            url = DEFAULT_MODEL_URL,
            path = %self.model_path.display(),
            "Downloading GGUF model (~670MB, first run)..."
        );

        let resp = self.client.get(DEFAULT_MODEL_URL).send().await
            .context("Failed to download GGUF model")?;
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Model download failed: HTTP {}", resp.status()));
        }

        let bytes = resp.bytes().await.context("Failed to read model data")?;
        std::fs::write(&self.model_path, &bytes).context("Failed to save model")?;

        tracing::info!(size_mb = bytes.len() / 1_048_576, "GGUF model downloaded");
        Ok(())
    }

    /// Launch the llama-server process if not already running.
    pub async fn ensure_server(&self) -> Result<()> {
        if self.health_check().await {
            return Ok(());
        }

        let mut proc = self.server_process.lock().await;
        if proc.is_some() {
            return Ok(());
        }

        let server_bin = self.ensure_server_binary().await?;
        self.ensure_model().await?;

        tracing::info!(
            bin = %server_bin.display(),
            model = %self.model_path.display(),
            port = self.port,
            "Starting llama-server..."
        );

        let child = tokio::process::Command::new(&server_bin)
            .arg("--model").arg(&self.model_path)
            .arg("--port").arg(self.port.to_string())
            .arg("--ctx-size").arg("4096")
            .arg("--n-gpu-layers").arg("99")
            .arg("--host").arg("127.0.0.1")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn llama-server")?;

        *proc = Some(child);

        // Wait for server ready (up to 30s)
        for i in 0..60 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            if self.health_check().await {
                tracing::info!(elapsed_ms = (i + 1) * 500, "llama-server ready");
                return Ok(());
            }
        }

        Err(anyhow::anyhow!("llama-server did not become ready within 30 seconds"))
    }

    async fn health_check(&self) -> bool {
        self.client
            .get(format!("{}/health", self.base_url))
            .timeout(std::time::Duration::from_secs(2))
            .send().await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    pub async fn stop(&self) {
        let mut proc = self.server_process.lock().await;
        if let Some(ref mut child) = *proc {
            let _ = child.kill().await;
            tracing::info!("llama-server stopped");
        }
        *proc = None;
    }
}

// -- OpenAI-compatible API types (same as llama.cpp server) -------------------

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<OaiMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<OaiTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Serialize, Deserialize)]
struct OaiMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OaiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct OaiTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OaiFunction,
}

#[derive(Serialize, Deserialize)]
struct OaiFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone)]
struct OaiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OaiFunctionCall,
}

#[derive(Serialize, Deserialize, Clone)]
struct OaiFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<OaiUsage>,
}

#[derive(Deserialize)]
struct Choice {
    message: OaiMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OaiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

// -- Conversions --------------------------------------------------------------

fn to_oai_messages(messages: &[Message]) -> Vec<OaiMessage> {
    messages.iter().map(|m| match m {
        Message::System { content } => OaiMessage {
            role: "system".into(), content: Some(content.clone()),
            tool_calls: None, tool_call_id: None,
        },
        Message::User { content } => OaiMessage {
            role: "user".into(), content: Some(content.clone()),
            tool_calls: None, tool_call_id: None,
        },
        Message::Assistant { content, tool_calls } => {
            let tc = if tool_calls.is_empty() { None } else {
                Some(tool_calls.iter().map(|tc| OaiToolCall {
                    id: tc.id.clone(),
                    call_type: "function".into(),
                    function: OaiFunctionCall {
                        name: tc.name.clone(),
                        arguments: tc.arguments.to_string(),
                    },
                }).collect())
            };
            OaiMessage {
                role: "assistant".into(), content: content.clone(),
                tool_calls: tc, tool_call_id: None,
            }
        }
        Message::Tool { tool_call_id, content } => OaiMessage {
            role: "tool".into(), content: Some(content.clone()),
            tool_calls: None, tool_call_id: Some(tool_call_id.clone()),
        },
    }).collect()
}

fn to_oai_tools(tools: &[ToolDef]) -> Vec<OaiTool> {
    tools.iter().map(|t| OaiTool {
        tool_type: "function".into(),
        function: OaiFunction {
            name: t.name.clone(),
            description: t.description.clone(),
            parameters: t.parameters.clone(),
        },
    }).collect()
}

// -- LlmProvider impl ---------------------------------------------------------

#[async_trait]
impl LlmProvider for GgufProvider {
    async fn chat(&self, messages: &[Message], tools: &[ToolDef]) -> Result<LlmResponse> {
        // Auto-bootstrap: download binary + model + start server
        self.ensure_server().await?;

        let body = ChatRequest {
            model: self.model_name.clone(),
            messages: to_oai_messages(messages),
            tools: to_oai_tools(tools),
            max_tokens: Some(2048),
        };

        let resp = self.client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Content-Type", "application/json")
            .json(&body)
            .send().await
            .context("Failed to connect to llama-server")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("llama-server API error {}: {}", status, text));
        }

        let data: ChatResponse = resp.json().await
            .context("Failed to parse llama-server response")?;

        let choice = data.choices.into_iter().next()
            .ok_or_else(|| anyhow::anyhow!("No choices in response"))?;

        let tool_calls: Vec<ToolCall> = choice.message.tool_calls
            .unwrap_or_default()
            .into_iter()
            .map(|tc| {
                let args = serde_json::from_str(&tc.function.arguments)
                    .unwrap_or(serde_json::Value::Object(Default::default()));
                ToolCall { id: tc.id, name: tc.function.name, arguments: args }
            })
            .collect();

        let stop_reason = match choice.finish_reason.as_deref() {
            Some("tool_calls") => StopReason::ToolUse,
            Some("length") => StopReason::MaxTokens,
            _ if !tool_calls.is_empty() => StopReason::ToolUse,
            _ => StopReason::EndTurn,
        };

        Ok(LlmResponse {
            content: choice.message.content,
            tool_calls,
            stop_reason,
            usage: data.usage.map(|u| Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
            }),
        })
    }

    fn name(&self) -> &str { "gguf" }
}

impl Drop for GgufProvider {
    fn drop(&mut self) {
        let proc = self.server_process.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            if let Ok(rt) = rt {
                rt.block_on(async {
                    let mut guard = proc.lock().await;
                    if let Some(ref mut child) = *guard {
                        let _ = child.kill().await;
                    }
                });
            }
        });
    }
}
