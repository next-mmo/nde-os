//! Runtime loader for the ONNX Runtime shared library.
//!
//! Keeps the installer small: we don't ship `onnxruntime.dll` / `.so` / `.dylib`
//! with the app. On first KFA use we download Microsoft's official CPU build
//! (~15 MB zip / ~20 MB tgz) and cache the extracted dylib next to the KFA
//! model. Subsequent starts point `ort` at the cached path via
//! `ort::init().with_library_path(...)`.

use anyhow::{anyhow, Context, Result};
use std::io::Read;
use std::path::{Path, PathBuf};

const ORT_VERSION: &str = "1.17.3";

/// Resolve the KFA resource directory inside the NDE-OS workspace.
fn kfa_dir(base_dir: &Path) -> PathBuf {
    base_dir.join("kfa")
}

/// Where the dylib lives (or will live) — no existence check, no I/O.
pub fn dylib_path(base_dir: &Path) -> PathBuf {
    kfa_dir(base_dir).join(platform_dylib_name().unwrap_or("onnxruntime.dll"))
}

/// True when the ONNX Runtime dylib is already cached on disk.
pub fn is_dylib_cached(base_dir: &Path) -> bool {
    dylib_path(base_dir).exists()
}

/// Return the absolute path to `onnxruntime.{dll,so,dylib}`, downloading and
/// extracting it on first call. Idempotent; cheap when cached.
pub fn ensure_dylib(base_dir: &Path) -> Result<PathBuf> {
    let kfa_dir = kfa_dir(base_dir);
    std::fs::create_dir_all(&kfa_dir)?;

    let dylib_name = platform_dylib_name()?;
    let dylib_path = kfa_dir.join(dylib_name);
    if dylib_path.exists() {
        return Ok(dylib_path);
    }

    let (archive_url, inner_path) = platform_archive()?;
    tracing::info!(url = %archive_url, "Downloading ONNX Runtime…");

    let client = reqwest::blocking::Client::builder()
        .timeout(None)
        .build()
        .context("Failed to build HTTP client")?;
    let mut response = client
        .get(&archive_url)
        .send()
        .context("Failed to download ONNX Runtime archive")?
        .error_for_status()
        .context("ONNX Runtime download returned non-success status")?;

    let mut archive_bytes = Vec::with_capacity(20 * 1024 * 1024);
    response
        .copy_to(&mut archive_bytes)
        .context("Failed to buffer ONNX Runtime archive")?;

    let tmp_path = dylib_path.with_extension("tmp");
    extract_dylib(&archive_bytes, &inner_path, &tmp_path)?;
    std::fs::rename(&tmp_path, &dylib_path)?;
    tracing::info!("ONNX Runtime cached at {}", dylib_path.display());
    Ok(dylib_path)
}

fn platform_dylib_name() -> Result<&'static str> {
    if cfg!(windows) {
        Ok("onnxruntime.dll")
    } else if cfg!(target_os = "macos") {
        Ok("libonnxruntime.dylib")
    } else if cfg!(target_os = "linux") {
        Ok("libonnxruntime.so")
    } else {
        Err(anyhow!("Unsupported platform for ONNX Runtime auto-download"))
    }
}

/// `(archive_url, path_inside_archive)` for the current platform.
fn platform_archive() -> Result<(String, String)> {
    let base = format!(
        "https://github.com/microsoft/onnxruntime/releases/download/v{ver}",
        ver = ORT_VERSION
    );
    if cfg!(windows) {
        // onnxruntime-win-x64-1.17.3.zip → lib/onnxruntime.dll
        let name = format!("onnxruntime-win-x64-{ORT_VERSION}.zip");
        let inner = format!("onnxruntime-win-x64-{ORT_VERSION}/lib/onnxruntime.dll");
        Ok((format!("{base}/{name}"), inner))
    } else if cfg!(target_os = "macos") {
        let name = format!("onnxruntime-osx-universal2-{ORT_VERSION}.tgz");
        let inner =
            format!("onnxruntime-osx-universal2-{ORT_VERSION}/lib/libonnxruntime.{ORT_VERSION}.dylib");
        Ok((format!("{base}/{name}"), inner))
    } else if cfg!(target_os = "linux") {
        let name = format!("onnxruntime-linux-x64-{ORT_VERSION}.tgz");
        let inner =
            format!("onnxruntime-linux-x64-{ORT_VERSION}/lib/libonnxruntime.so.{ORT_VERSION}");
        Ok((format!("{base}/{name}"), inner))
    } else {
        Err(anyhow!("Unsupported platform"))
    }
}

fn extract_dylib(archive_bytes: &[u8], inner_path: &str, out_path: &std::path::Path) -> Result<()> {
    if cfg!(windows) {
        extract_from_zip(archive_bytes, inner_path, out_path)
    } else {
        extract_from_tgz(archive_bytes, inner_path, out_path)
    }
}

fn extract_from_zip(archive_bytes: &[u8], inner_path: &str, out_path: &std::path::Path) -> Result<()> {
    let cursor = std::io::Cursor::new(archive_bytes);
    let mut zip = zip::ZipArchive::new(cursor).context("Invalid ONNX Runtime zip")?;
    let mut entry = zip
        .by_name(inner_path)
        .with_context(|| format!("Missing '{inner_path}' in ONNX Runtime zip"))?;
    let mut buf = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut buf)?;
    std::fs::write(out_path, buf)?;
    Ok(())
}

fn extract_from_tgz(archive_bytes: &[u8], inner_path: &str, out_path: &std::path::Path) -> Result<()> {
    let gz = flate2::read::GzDecoder::new(archive_bytes);
    let mut tar = tar::Archive::new(gz);
    for entry in tar.entries()? {
        let mut entry = entry?;
        let entry_path = entry.path()?.into_owned();
        if entry_path.to_string_lossy() == inner_path {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            std::fs::write(out_path, buf)?;
            return Ok(());
        }
    }
    Err(anyhow!("Missing '{inner_path}' in ONNX Runtime archive"))
}
