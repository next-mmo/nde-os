/// GGUF LLM Provider — auto-bootstrapping llama.cpp subprocess.
///
/// Downloads pre-built llama-server binary + default model on first use.
/// Lifecycle: bootstrap binary -> download model -> launch server -> OpenAI-compat API.
use super::{
    streaming, ChunkStream, LlmProvider, LlmResponse, Message, StopReason, ToolCall, ToolDef, Usage,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;

/// Cached GPU detection result (runs nvidia-smi once per process).
static GPU_DETECTED: OnceLock<bool> = OnceLock::new();

/// Info about a locally-available GGUF model file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalGgufModel {
    pub filename: String,
    pub path: String,
    pub size_bytes: u64,
    pub size_display: String,
}

/// Result of verifying a GGUF provider can actually run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GgufVerifyResult {
    pub ok: bool,
    pub model_exists: bool,
    pub model_path: String,
    pub server_available: bool,
    pub server_path: Option<String>,
    pub error: Option<String>,
}

/// Default small model for first-run — bundled in core/models/.
const DEFAULT_MODEL_URL: &str =
    "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf";
const DEFAULT_MODEL_FILENAME: &str = "qwen2.5-0.5b-instruct-q4_k_m.gguf";

/// llama.cpp release to auto-download.
const LLAMA_CPP_VERSION: &str = "b4679";
const DEFAULT_PORT: u16 = 8090;

#[derive(Clone)]
pub struct GgufProvider {
    client: reqwest::Client,
    base_url: String,
    download_url: String,
    data_dir: PathBuf,
    model_path: PathBuf,
    model_name: String,
    port: u16,
    server_process: Arc<Mutex<Option<tokio::process::Child>>>,
}

impl GgufProvider {
    pub fn new(
        data_dir: &Path,
        model_id: &str,
        model_path: Option<&str>,
        port: Option<u16>,
    ) -> Self {
        let port = port.unwrap_or(DEFAULT_PORT);

        let mut download_url = DEFAULT_MODEL_URL.to_string();
        let mut default_filename = DEFAULT_MODEL_FILENAME.to_string();

        let recs = GgufModelRecommendation::recommend_models(u64::MAX, None);
        // Try exact ID match, then try matching by URL filename containing the model_id
        let rec_match = recs.iter().find(|r| r.id == model_id).or_else(|| {
            let id_lower = model_id.to_lowercase();
            recs.iter().find(|r| {
                let url_filename = r.url.split('/').last().unwrap_or_default().to_lowercase();
                // Match "Qwen3.5-9B-Q4_K_M" against URL filename "Qwen3.5-9B-Q4_K_M.gguf"
                url_filename.starts_with(&id_lower)
                    || url_filename.trim_end_matches(".gguf") == id_lower
            })
        });

        if let Some(rec) = rec_match {
            download_url = rec.url.clone();
            default_filename = download_url
                .split('/')
                .last()
                .unwrap_or(DEFAULT_MODEL_FILENAME)
                .to_string();
        } else if model_id.starts_with("http") {
            download_url = model_id.to_string();
            default_filename = download_url
                .split('/')
                .last()
                .unwrap_or(DEFAULT_MODEL_FILENAME)
                .to_string();
        } else if model_id.ends_with(".gguf") || model_id.contains("-Q") || model_id.contains("-q")
        {
            // Looks like a GGUF filename or model name pattern — use it directly
            let fname = if model_id.ends_with(".gguf") {
                model_id.to_string()
            } else {
                format!("{}.gguf", model_id)
            };
            default_filename = fname;
        }

        // Resolve model path: explicit path > bundled core/models > data_dir/models
        let model_path = match model_path {
            Some(p) if !p.is_empty() => PathBuf::from(p),
            _ => {
                // Check bundled models directory first (core/models/)
                let bundled = Self::find_bundled_model(&default_filename);
                if let Some(bp) = bundled {
                    bp
                } else {
                    data_dir.join("models").join(&default_filename)
                }
            }
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
            download_url,
            data_dir: data_dir.to_path_buf(),
            model_path,
            model_name,
            port,
            server_process: Arc::new(Mutex::new(None)),
        }
    }

    /// Search for a model file in the bundled core/models/ directory.
    /// This looks relative to the executable path and common project layouts.
    fn find_bundled_model(filename: &str) -> Option<PathBuf> {
        // Try paths relative to the current exe
        let candidates: Vec<PathBuf> = vec![
            // Running from project root (dev mode)
            PathBuf::from("core/models").join(filename),
            // Running from target/debug or target/release
            PathBuf::from("../../core/models").join(filename),
        ];

        // Also try relative to the executable location
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                let extras = vec![
                    exe_dir.join("core/models").join(filename),
                    exe_dir.join("../../core/models").join(filename),
                    exe_dir.join("../../../core/models").join(filename),
                ];
                for p in extras {
                    if p.exists() {
                        if let Ok(canon) = p.canonicalize() {
                            tracing::info!(path = %canon.display(), "Found bundled model");
                            return Some(canon);
                        }
                    }
                }
            }
        }

        for p in candidates {
            if p.exists() {
                if let Ok(canon) = p.canonicalize() {
                    tracing::info!(path = %canon.display(), "Found bundled model");
                    return Some(canon);
                }
            }
        }
        None
    }

    /// List all .gguf model files found in the data models dir AND bundled core/models/.
    pub fn list_local_models(data_dir: &Path) -> Vec<LocalGgufModel> {
        let mut models = Vec::new();
        let mut seen_filenames = std::collections::HashSet::new();

        // Directories to scan: data_dir/models + bundled core/models/
        let mut dirs_to_scan: Vec<PathBuf> = vec![data_dir.join("models")];

        // Add bundled model directories
        let bundled_dirs = vec![
            PathBuf::from("core/models"),
            PathBuf::from("../../core/models"),
        ];
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                dirs_to_scan.push(exe_dir.join("core/models"));
                dirs_to_scan.push(exe_dir.join("../../core/models"));
                dirs_to_scan.push(exe_dir.join("../../../core/models"));
            }
        }
        dirs_to_scan.extend(bundled_dirs);

        for dir in &dirs_to_scan {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("gguf") {
                        let fname = path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        if seen_filenames.contains(&fname) {
                            continue; // deduplicate by filename
                        }
                        if let Ok(meta) = std::fs::metadata(&path) {
                            let size = meta.len();
                            let size_display = if size >= 1_073_741_824 {
                                format!("{:.1} GB", size as f64 / 1_073_741_824.0)
                            } else {
                                format!("{:.0} MB", size as f64 / 1_048_576.0)
                            };
                            // Canonicalize the path for consistency
                            let abs_path = path.canonicalize().unwrap_or(path.clone());
                            seen_filenames.insert(fname.clone());
                            models.push(LocalGgufModel {
                                filename: fname,
                                path: abs_path.to_string_lossy().to_string(),
                                size_bytes: size,
                                size_display,
                            });
                        }
                    }
                }
            }
        }

        // Sort by size ascending
        models.sort_by_key(|m| m.size_bytes);
        models
    }

    /// Verify this provider is usable: model file exists and server binary is available.
    pub async fn verify(&self) -> GgufVerifyResult {
        let model_exists = self.model_path.exists();
        let model_path = self.model_path.to_string_lossy().to_string();

        // Check server binary
        let server_bin = self.server_bin_path();
        let has_local_bin = server_bin.exists();
        let has_system_bin = Self::find_in_path().is_some();
        let server_available = has_local_bin || has_system_bin;
        let server_path = if has_local_bin {
            Some(server_bin.to_string_lossy().to_string())
        } else {
            Self::find_in_path().map(|p| p.to_string_lossy().to_string())
        };

        let mut errors = Vec::new();
        if !model_exists {
            // Check if the download URL looks valid
            if self.download_url.starts_with("http") {
                // Model will be auto-downloaded — not a blocking error but note it
                // Let's do a HEAD request to verify the URL is reachable
                match self
                    .client
                    .head(&self.download_url)
                    .timeout(std::time::Duration::from_secs(10))
                    .send()
                    .await
                {
                    Ok(resp) if resp.status().is_success() || resp.status().is_redirection() => {
                        // URL is valid, model will be downloaded on first use
                    }
                    Ok(resp) => {
                        errors.push(format!(
                            "Model not found locally and download URL returned HTTP {}",
                            resp.status()
                        ));
                    }
                    Err(e) => {
                        errors.push(format!(
                            "Model not found locally and download URL unreachable: {}",
                            e
                        ));
                    }
                }
            } else {
                errors.push(format!("Model file not found: {}", model_path));
            }
        }
        if !server_available {
            errors.push(
                "llama-server binary not found (will auto-download on first use)".to_string(),
            );
        }

        let error = if errors.is_empty() {
            None
        } else {
            Some(errors.join("; "))
        };
        let ok = model_exists || (self.download_url.starts_with("http") && error.is_none());

        GgufVerifyResult {
            ok,
            model_exists,
            model_path,
            server_available,
            server_path,
            error,
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
    /// Also handles GPU upgrade: if a GPU is now available but the current
    /// binary is CPU-only (no ggml-cuda DLL), re-downloads the CUDA build.
    pub async fn ensure_server_binary(&self) -> Result<PathBuf> {
        let bin_path = self.server_bin_path();
        let bin_dir = self.data_dir.join("bin");

        if bin_path.exists() {
            // GPU upgrade check: binary exists but may be CPU-only
            let needs_gpu_upgrade = Self::has_nvidia_gpu() && cfg!(windows) && {
                let cuda_dll = bin_dir.join("ggml-cuda.dll");
                !cuda_dll.exists()
            };
            if needs_gpu_upgrade {
                tracing::info!("GPU detected but CUDA DLLs missing — upgrading to CUDA build...");
                // Remove old CPU-only binaries
                if let Err(e) = std::fs::remove_dir_all(&bin_dir) {
                    tracing::warn!(error = %e, "Failed to remove old bin dir for GPU upgrade");
                    // Fall through — still try to use existing binary
                    return Ok(bin_path);
                }
                // Fall through to download CUDA build
            } else {
                return Ok(bin_path);
            }
        }

        // Also check system PATH first
        if let Some(system_bin) = Self::find_in_path() {
            return Ok(system_bin);
        }

        let bin_dir = self.data_dir.join("bin");
        std::fs::create_dir_all(&bin_dir).context("Failed to create bin directory")?;

        let (url, archive_name) = Self::release_url();
        tracing::info!(url = %url, "Downloading llama-server (first run)...");

        let resp = self
            .client
            .get(&url)
            .send()
            .await
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

        // Also download CUDA runtime DLLs if needed (Windows + NVIDIA GPU)
        if let Some(cudart_url) = Self::cudart_url() {
            tracing::info!(url = %cudart_url, "Downloading CUDA runtime DLLs...");
            match self.client.get(&cudart_url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(cudart_bytes) = resp.bytes().await {
                        if let Err(e) = Self::extract_server(&cudart_bytes, "zip", &bin_dir) {
                            tracing::warn!(error = %e, "Failed to extract cudart — GPU may not work");
                        } else {
                            tracing::info!("CUDA runtime DLLs installed");
                        }
                    }
                }
                Ok(resp) => {
                    tracing::warn!(status = %resp.status(), "Failed to download cudart — GPU may not work");
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to download cudart — GPU may not work");
                }
            }
        }

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

    /// Detect whether an NVIDIA GPU is available (via nvidia-smi).
    /// Result is cached for the lifetime of the process via OnceLock.
    fn has_nvidia_gpu() -> bool {
        *GPU_DETECTED.get_or_init(|| Self::detect_nvidia_gpu())
    }

    /// Actually run nvidia-smi to detect GPU. Called once, result cached.
    fn detect_nvidia_gpu() -> bool {
        // Try nvidia-smi from PATH first (System32 on Windows, /usr/bin on Linux)
        let mut result = std::process::Command::new("nvidia-smi")
            .arg("--query-gpu=name")
            .arg("--format=csv,noheader")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .output();

        // Fallback: try explicit Windows path if PATH lookup failed
        #[cfg(windows)]
        if result.is_err() {
            // nvidia-smi lives in System32 on Windows, but some environments
            // strip System32 from PATH. Also check Program Files.
            for path in [
                r"C:\Windows\System32\nvidia-smi.exe",
                r"C:\Program Files\NVIDIA Corporation\NVSMI\nvidia-smi.exe",
            ] {
                if PathBuf::from(path).exists() {
                    result = std::process::Command::new(path)
                        .arg("--query-gpu=name")
                        .arg("--format=csv,noheader")
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::null())
                        .output();
                    if result.is_ok() {
                        break;
                    }
                }
            }
        }

        match result {
            Ok(out) if out.status.success() => {
                let s = String::from_utf8_lossy(&out.stdout);
                let found = !s.trim().is_empty();
                if found {
                    tracing::info!(gpu = %s.trim(), "NVIDIA GPU detected — using CUDA build");
                } else {
                    tracing::info!("nvidia-smi found but no GPU returned");
                }
                found
            }
            _ => {
                tracing::info!("No NVIDIA GPU detected — using CPU build");
                false
            }
        }
    }

    /// Build the GitHub release URL(s) for the current platform.
    /// Returns `(main_url, archive_type)`.
    /// Use `cudart_url()` to get the companion CUDA runtime archive if needed.
    fn release_url() -> (String, String) {
        let tag = LLAMA_CPP_VERSION;
        let has_gpu = Self::has_nvidia_gpu();

        let (suffix, archive) = if cfg!(target_os = "windows") {
            if cfg!(target_arch = "x86_64") {
                if has_gpu {
                    ("bin-win-cuda-cu12.4-x64.zip", "zip")
                } else {
                    ("bin-win-avx2-x64.zip", "zip")
                }
            } else {
                ("bin-win-msvc-arm64.zip", "zip")
            }
        } else if cfg!(target_os = "macos") {
            // macOS uses Metal (built into the default binary), no CUDA
            if cfg!(target_arch = "x86_64") {
                ("bin-macos-x64.zip", "zip")
            } else {
                ("bin-macos-arm64.zip", "zip")
            }
        } else {
            // Linux
            if cfg!(target_arch = "x86_64") {
                if has_gpu {
                    ("bin-ubuntu-x64.zip", "zip") // Linux CUDA builds have same name
                } else {
                    ("bin-ubuntu-x64.zip", "zip")
                }
            } else {
                ("bin-ubuntu-arm64.zip", "zip")
            }
        };
        let url = format!(
            "https://github.com/ggerganov/llama.cpp/releases/download/{}/llama-{}-{}",
            tag, tag, suffix
        );
        (url, archive.to_string())
    }

    /// URL for the CUDA runtime DLLs (cudart) — needed alongside the CUDA build on Windows.
    fn cudart_url() -> Option<String> {
        if !cfg!(target_os = "windows") || !cfg!(target_arch = "x86_64") {
            return None;
        }
        if !Self::has_nvidia_gpu() {
            return None;
        }
        let tag = LLAMA_CPP_VERSION;
        Some(format!(
            "https://github.com/ggerganov/llama.cpp/releases/download/{}/cudart-llama-bin-win-cu12.4-x64.zip",
            tag
        ))
    }

    /// Extract llama-server and ALL companion files (DLLs etc.) from the downloaded archive.
    fn extract_server(bytes: &[u8], _archive_type: &str, dest: &Path) -> Result<()> {
        // llama.cpp releases are ZIP files on all platforms.
        // We MUST extract all files (DLLs, .so, .dylib) — not just the server binary,
        // because llama-server.exe depends on ggml-base.dll and other shared libraries.
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor).context("Failed to open zip archive")?;

        let mut extracted_count = 0u32;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).context("Failed to read zip entry")?;
            let name = file.name().to_string();

            if file.is_dir() {
                continue;
            }

            // Get just the filename (strip directory prefixes from the archive)
            let file_name = match PathBuf::from(&name).file_name() {
                Some(f) => f.to_os_string(),
                None => continue,
            };

            let out_path = dest.join(&file_name);
            let mut out_file = std::fs::File::create(&out_path)
                .with_context(|| format!("Failed to create file: {}", out_path.display()))?;
            std::io::copy(&mut file, &mut out_file)
                .with_context(|| format!("Failed to write file: {}", out_path.display()))?;
            extracted_count += 1;
        }

        tracing::info!(count = extracted_count, dest = %dest.display(), "Extracted llama.cpp files");
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
            url = %self.download_url,
            path = %self.model_path.display(),
            "Downloading GGUF model (first run)..."
        );

        let resp = self
            .client
            .get(&self.download_url)
            .send()
            .await
            .context("Failed to download GGUF model")?;
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "Model download failed: HTTP {}",
                resp.status()
            ));
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

        // Use larger context for agent workflows; GPU can handle it
        let has_gpu = Self::has_nvidia_gpu();
        let ctx_size = if has_gpu { "16384" } else { "8192" };

        tracing::info!(
            bin = %server_bin.display(),
            model = %self.model_path.display(),
            port = self.port,
            ctx_size,
            gpu = has_gpu,
            "Starting llama-server..."
        );

        let bin_dir = server_bin
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();

        let mut cmd = tokio::process::Command::new(&server_bin);
        cmd.arg("--model")
            .arg(&self.model_path)
            .arg("--port")
            .arg(self.port.to_string())
            .arg("--ctx-size")
            .arg(ctx_size)
            .arg("--n-gpu-layers")
            .arg("99")
            .arg("--host")
            .arg("127.0.0.1")
            // Set working directory to bin/ so Windows can find companion DLLs
            // (ggml-cuda.dll, cudart64_*.dll, etc.)
            .current_dir(&bin_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // Enable flash attention for GPU — significantly faster inference
        if has_gpu {
            cmd.arg("--flash-attn");
        }

        let child = cmd.spawn().context("Failed to spawn llama-server")?;

        *proc = Some(child);

        // Wait for server ready (up to 90s — large models on first GPU load need time)
        for i in 0..180 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            if self.health_check().await {
                tracing::info!(elapsed_ms = (i + 1) * 500, "llama-server ready");
                return Ok(());
            }
        }

        Err(anyhow::anyhow!(
            "llama-server did not become ready within 90 seconds"
        ))
    }

    async fn health_check(&self) -> bool {
        self.client
            .get(format!("{}/health", self.base_url))
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
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
    messages
        .iter()
        .map(|m| match m {
            Message::System { content } => OaiMessage {
                role: "system".into(),
                content: Some(content.clone()),
                tool_calls: None,
                tool_call_id: None,
            },
            Message::User { content } => OaiMessage {
                role: "user".into(),
                content: Some(content.clone()),
                tool_calls: None,
                tool_call_id: None,
            },
            Message::Assistant {
                content,
                tool_calls,
            } => {
                let tc = if tool_calls.is_empty() {
                    None
                } else {
                    Some(
                        tool_calls
                            .iter()
                            .map(|tc| OaiToolCall {
                                id: tc.id.clone(),
                                call_type: "function".into(),
                                function: OaiFunctionCall {
                                    name: tc.name.clone(),
                                    arguments: tc.arguments.to_string(),
                                },
                            })
                            .collect(),
                    )
                };
                OaiMessage {
                    role: "assistant".into(),
                    content: content.clone(),
                    tool_calls: tc,
                    tool_call_id: None,
                }
            }
            Message::Tool {
                tool_call_id,
                content,
            } => OaiMessage {
                role: "tool".into(),
                content: Some(content.clone()),
                tool_calls: None,
                tool_call_id: Some(tool_call_id.clone()),
            },
        })
        .collect()
}

fn to_oai_tools(tools: &[ToolDef]) -> Vec<OaiTool> {
    tools
        .iter()
        .map(|t| OaiTool {
            tool_type: "function".into(),
            function: OaiFunction {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: t.parameters.clone(),
            },
        })
        .collect()
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
            // Skip tools for GGUF models — most small models don't support tool calling
            // and sending tools causes llama-server to crash without --jinja flag
            tools: vec![],
            max_tokens: Some(2048),
        };

        let resp = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to connect to llama-server")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "llama-server API error {}: {}",
                status,
                text
            ));
        }

        let data: ChatResponse = resp
            .json()
            .await
            .context("Failed to parse llama-server response")?;

        let choice = data
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No choices in response"))?;

        let tool_calls: Vec<ToolCall> = choice
            .message
            .tool_calls
            .unwrap_or_default()
            .into_iter()
            .map(|tc| {
                let args = serde_json::from_str(&tc.function.arguments)
                    .unwrap_or(serde_json::Value::Object(Default::default()));
                ToolCall {
                    id: tc.id,
                    name: tc.function.name,
                    arguments: args,
                }
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

    fn name(&self) -> &str {
        "gguf"
    }

    async fn chat_stream(&self, messages: &[Message], tools: &[ToolDef]) -> Result<ChunkStream> {
        let messages = messages.to_vec();
        let tools = tools.to_vec();
        let provider = self.clone();

        let needs_download = !self.model_path.exists();

        let stream = async_stream::stream! {
            if needs_download {
                yield Ok(streaming::StreamChunk::TextDelta {
                    content: "\n*(Downloading GGUF model and starting server in the background... This may take a few minutes depending on your internet connection and the model size, please wait!)*\n\n".to_string()
                });
            } else if !provider.health_check().await {
                yield Ok(streaming::StreamChunk::TextDelta {
                    content: "\n*(Starting local GGUF inference server...)*\n\n".to_string()
                });
            }

            if let Err(e) = provider.ensure_server().await {
                yield Ok(streaming::StreamChunk::Error { message: format!("\n**Error starting GGUF server:** {}", e) });
                return;
            }

            match provider.chat(&messages, &tools).await {
                Ok(resp) => {
                    if let Some(content) = &resp.content {
                        yield Ok(streaming::StreamChunk::TextDelta { content: content.clone() });
                    }
                    yield Ok(streaming::StreamChunk::Done { content: None, usage: resp.usage });
                }
                Err(e) => {
                    yield Ok(streaming::StreamChunk::Error { message: format!("\n**Error during inference:** {}", e) });
                }
            }
        };
        Ok(Box::pin(stream))
    }
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

// -- Recommendation -------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GgufModelRecommendation {
    pub id: String,
    pub name: String,
    pub url: String,
    pub description: String,
    pub size_gb: f64,
    pub recommended_ram_gb: u32,
}

impl GgufModelRecommendation {
    pub fn recommend_models(ram_bytes: u64, _gpu_vram_bytes: Option<u64>) -> Vec<Self> {
        let ram_gb = ram_bytes
            .saturating_mul(10)
            .checked_div(1024 * 1024 * 1024)
            .unwrap_or(0) as f64
            / 10.0;
        let mut models = vec![];

        // 1. Qwen 2.5 0.5B — bundled, instant use, no download needed
        models.push(GgufModelRecommendation {
            id: "qwen2.5-0.5b-instruct".to_string(),
            name: "Qwen 2.5 0.5B Instruct".to_string(),
            url: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf".to_string(),
            description: "Ultra-light bundled model. Instant start, no download. Good for basic tasks.".to_string(),
            size_gb: 0.46,
            recommended_ram_gb: 2,
        });

        // 2. Qwen2.5 1.5B
        models.push(GgufModelRecommendation {
            id: "qwen2.5-1.5b".to_string(),
            name: "Qwen 2.5 1.5B Instruct".to_string(),
            url: "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf".to_string(),
            description: "Smart, capable small model for coding and multilingual text.".to_string(),
            size_gb: 1.12,
            recommended_ram_gb: 4,
        });

        // 3. Llama-3.2 3B
        if ram_gb >= 3.5 {
            models.push(GgufModelRecommendation {
                id: "llama-3.2-3b".to_string(),
                name: "Llama 3.2 3B Instruct".to_string(),
                url: "https://huggingface.co/hugging-quants/Llama-3.2-3B-Instruct-Q4_K_M-GGUF/resolve/main/llama-3.2-3b-instruct-q4_k_m.gguf".to_string(),
                description: "Capable mid-sized model from Meta, good balance of speed and logic.".to_string(),
                size_gb: 2.02,
                recommended_ram_gb: 4,
            });
        }

        // 4. Qwen 3.5 9B — bundled, high quality
        if ram_gb >= 6.0 {
            models.push(GgufModelRecommendation {
                id: "qwen3.5-9b".to_string(),
                name: "Qwen 3.5 9B".to_string(),
                url: "https://huggingface.co/Qwen/Qwen3.5-9B-GGUF/resolve/main/Qwen3.5-9B-Q4_K_M.gguf".to_string(),
                description: "Bundled high-quality 9B model. Excellent reasoning and coding capability.".to_string(),
                size_gb: 5.24,
                recommended_ram_gb: 8,
            });
        }

        // 5. Llama-3 8B
        if ram_gb >= 6.5 {
            models.push(GgufModelRecommendation {
                id: "llama-3-8b".to_string(),
                name: "Llama 3 8B Instruct".to_string(),
                url: "https://huggingface.co/QuantFactory/Meta-Llama-3-8B-Instruct-GGUF/resolve/main/Meta-Llama-3-8B-Instruct.Q4_K_M.gguf".to_string(),
                description: "Heavyweight 8B model. High intelligence for complex chat.".to_string(),
                size_gb: 4.92,
                recommended_ram_gb: 8,
            });
        }

        models
    }
}
