use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::browser::BrowserEngine;

// ─── Engine Version Info ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineVersion {
    pub engine: BrowserEngine,
    pub version: String,
    pub download_url: String,
    /// SHA256 hash for integrity verification
    pub sha256: Option<String>,
}

/// Platform-specific download URL builder.
/// Wayfern releases: https://github.com/nicksrandall/nickel-chromium/releases
/// Camoufox releases: https://github.com/nicksrandall/nickel-chromium/releases
fn get_platform_suffix() -> &'static str {
    if cfg!(target_os = "windows") {
        if cfg!(target_arch = "x86_64") {
            "win64"
        } else {
            "win32"
        }
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "mac-arm64"
        } else {
            "mac-x64"
        }
    } else {
        "linux-x64"
    }
}

// ─── Engine Manager ────────────────────────────────────────────────

/// Manages downloading, extracting, and versioning of browser engine binaries.
pub struct EngineManager {
    engines_dir: PathBuf,
}

impl EngineManager {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            engines_dir: base_dir.join("shield-engines"),
        }
    }

    pub fn engines_dir(&self) -> &Path {
        &self.engines_dir
    }

    /// Get the installation directory for a specific engine version.
    /// e.g., {engines_dir}/wayfern/133.0.0/
    pub fn engine_install_dir(&self, engine: &BrowserEngine, version: &str) -> PathBuf {
        self.engines_dir.join(engine.as_str()).join(version)
    }

    /// Check if a specific engine version is already downloaded.
    pub fn is_downloaded(&self, engine: &BrowserEngine, version: &str) -> bool {
        let install_dir = self.engine_install_dir(engine, version);
        if !install_dir.exists() {
            return false;
        }

        // Verify the executable actually exists
        super::browser::find_executable(engine, &install_dir).is_ok()
    }

    /// Get the executable path for a downloaded engine version.
    pub fn get_executable(&self, engine: &BrowserEngine, version: &str) -> Result<PathBuf> {
        let install_dir = self.engine_install_dir(engine, version);
        if !install_dir.exists() {
            anyhow::bail!(
                "{} version {} is not downloaded. Install it first.",
                engine.display_name(),
                version
            );
        }

        super::browser::find_executable(engine, &install_dir)
    }

    /// List all downloaded engine versions.
    pub fn list_downloaded(&self) -> Result<Vec<(BrowserEngine, String)>> {
        let mut result = Vec::new();

        for engine in &[BrowserEngine::Wayfern, BrowserEngine::Camoufox] {
            let engine_dir = self.engines_dir.join(engine.as_str());
            if !engine_dir.exists() {
                continue;
            }

            for entry in std::fs::read_dir(&engine_dir)
                .with_context(|| format!("Failed to read {} engines dir", engine.as_str()))?
            {
                let entry = entry?;
                if entry.path().is_dir() {
                    let version = entry.file_name().to_string_lossy().to_string();
                    if self.is_downloaded(engine, &version) {
                        result.push((engine.clone(), version));
                    }
                }
            }
        }

        Ok(result)
    }

    /// Download and extract an engine version with progress reporting.
    /// The `on_progress` callback receives `(bytes_downloaded, total_bytes)`.
    /// `total_bytes` is 0 if the server didn't send a Content-Length header.
    /// Returns the installation directory path.
    pub async fn download_engine<F>(
        &self,
        engine: &BrowserEngine,
        version: &str,
        download_url: &str,
        on_progress: F,
    ) -> Result<PathBuf>
    where
        F: Fn(u64, u64) + Send + 'static,
    {
        let install_dir = self.engine_install_dir(engine, version);

        if self.is_downloaded(engine, version) {
            tracing::info!(
                "{} version {} is already downloaded",
                engine.display_name(),
                version
            );
            return Ok(install_dir);
        }

        tracing::info!(
            "Downloading {} version {} from {}",
            engine.display_name(),
            version,
            download_url
        );

        std::fs::create_dir_all(&install_dir)
            .context("Failed to create engine installation directory")?;

        // Download with streaming progress
        let client = reqwest::Client::new();
        let response = client
            .get(download_url)
            .send()
            .await
            .context("Failed to download engine binary")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Download failed with status {}: {}",
                response.status(),
                download_url
            );
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;
        let mut bytes_buf = Vec::with_capacity(total_size as usize);

        // Stream chunks and report progress
        let mut stream = response;
        while let Some(chunk) = stream.chunk().await.context("Failed to read download chunk")? {
            downloaded += chunk.len() as u64;
            bytes_buf.extend_from_slice(&chunk);
            on_progress(downloaded, total_size);
        }

        // Determine archive type from URL and extract
        let archive_path = install_dir.join("_download_archive");
        std::fs::write(&archive_path, &bytes_buf)
            .context("Failed to write downloaded archive")?;

        // Extract based on file extension in URL
        if download_url.ends_with(".zip") {
            extract_zip(&archive_path, &install_dir)?;
        } else if download_url.ends_with(".tar.gz") || download_url.ends_with(".tgz") {
            extract_tar_gz(&archive_path, &install_dir)?;
        } else if download_url.ends_with(".tar.bz2") {
            extract_tar_bz2(&archive_path, &install_dir)?;
        } else {
            // Try zip first, then tar.gz
            if extract_zip(&archive_path, &install_dir).is_err() {
                extract_tar_gz(&archive_path, &install_dir)?;
            }
        }

        // Cleanup the archive
        let _ = std::fs::remove_file(&archive_path);

        // Set executable permissions on Unix
        #[cfg(unix)]
        {
            if let Ok(exe_path) = super::browser::find_executable(engine, &install_dir) {
                use std::os::unix::fs::PermissionsExt;
                let metadata = std::fs::metadata(&exe_path)?;
                let mut perms = metadata.permissions();
                perms.set_mode(perms.mode() | 0o755);
                std::fs::set_permissions(&exe_path, perms)?;
            }
        }

        tracing::info!(
            "{} version {} installed successfully to {}",
            engine.display_name(),
            version,
            install_dir.display()
        );

        Ok(install_dir)
    }

    /// Remove a downloaded engine version.
    pub fn remove_engine(&self, engine: &BrowserEngine, version: &str) -> Result<()> {
        let install_dir = self.engine_install_dir(engine, version);
        if install_dir.exists() {
            std::fs::remove_dir_all(&install_dir)
                .context("Failed to remove engine installation")?;
        }
        Ok(())
    }

    /// Get the current platform suffix for downloads.
    pub fn platform_suffix() -> &'static str {
        get_platform_suffix()
    }
}

// ─── Archive Extraction ────────────────────────────────────────────

fn extract_zip(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let file = std::fs::File::open(archive_path)
        .context("Failed to open zip archive")?;
    let mut archive = zip::ZipArchive::new(file)
        .context("Failed to read zip archive")?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = dest_dir.join(file.mangled_name());

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = std::fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }

    Ok(())
}

fn extract_tar_gz(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    use std::io::Read;
    let file = std::fs::File::open(archive_path)
        .context("Failed to open tar.gz archive")?;
    let buf_reader = std::io::BufReader::new(file);
    let gz = flate2::read::GzDecoder::new(buf_reader);
    let mut archive = tar::Archive::new(gz);

    archive.unpack(dest_dir)
        .context("Failed to extract tar.gz archive")?;
    Ok(())
}

fn extract_tar_bz2(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let file = std::fs::File::open(archive_path)
        .context("Failed to open tar.bz2 archive")?;
    let buf_reader = std::io::BufReader::new(file);
    let bz2 = bzip2::read::BzDecoder::new(buf_reader);
    let mut archive = tar::Archive::new(bz2);

    archive.unpack(dest_dir)
        .context("Failed to extract tar.bz2 archive")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_engine_install_dir_structure() {
        let tmp = TempDir::new().unwrap();
        let mgr = EngineManager::new(tmp.path());

        let dir = mgr.engine_install_dir(&BrowserEngine::Wayfern, "133.0.0");
        assert!(dir.ends_with("shield-engines/wayfern/133.0.0"));

        let dir = mgr.engine_install_dir(&BrowserEngine::Camoufox, "120.5");
        assert!(dir.ends_with("shield-engines/camoufox/120.5"));
    }

    #[test]
    fn test_not_downloaded_by_default() {
        let tmp = TempDir::new().unwrap();
        let mgr = EngineManager::new(tmp.path());

        assert!(!mgr.is_downloaded(&BrowserEngine::Wayfern, "133.0.0"));
        assert!(!mgr.is_downloaded(&BrowserEngine::Camoufox, "120.5"));
    }

    #[test]
    fn test_list_downloaded_empty() {
        let tmp = TempDir::new().unwrap();
        let mgr = EngineManager::new(tmp.path());

        let result = mgr.list_downloaded().unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_platform_suffix_not_empty() {
        let suffix = EngineManager::platform_suffix();
        assert!(!suffix.is_empty());
    }
}
