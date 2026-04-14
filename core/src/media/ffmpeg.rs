//! Auto-bootstrap FFmpeg + FFprobe into the NDE-OS sandbox.
//!
//! Shared core utility — any NDE-OS module or app can call [`ensure_ffmpeg`]
//! or [`find_ffmpeg`] without depending on the FreeCut subsystem.
//!
//! Resolution order: check `base_dir/.ffmpeg/` (bundled) → system PATH →
//! auto-download static binaries into `base_dir/.ffmpeg/`.
//!
//! Download sources:
//!   - **Windows**: BtbN/FFmpeg-Builds (GPL static, x86_64)
//!   - **macOS**:   evermeet.cx (static universal binaries)
//!   - **Linux**:   BtbN/FFmpeg-Builds (GPL static, x86_64 & arm64)

use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

// ─── Public API ────────────────────────────────────────────────────────────────

/// Resolved paths to both FFmpeg and FFprobe binaries.
#[derive(Debug, Clone)]
pub struct FfmpegBins {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
}

/// Ensure FFmpeg + FFprobe are available. Returns paths to the binaries.
///
/// Resolution order:
/// 1. Bundled inside `base_dir/.ffmpeg/`
/// 2. System `PATH`
/// 3. Auto-download static binaries into `base_dir/.ffmpeg/`
pub fn ensure_ffmpeg(base_dir: &Path) -> Result<FfmpegBins> {
    let ff_dir = ffmpeg_home(base_dir);

    // 1. Check bundled
    let bundled_ffmpeg = ff_dir.join(ffmpeg_binary_name());
    let bundled_ffprobe = ff_dir.join(ffprobe_binary_name());
    if bundled_ffmpeg.exists() && bundled_ffprobe.exists() {
        return Ok(FfmpegBins {
            ffmpeg: bundled_ffmpeg,
            ffprobe: bundled_ffprobe,
        });
    }

    // 2. Check system PATH
    if let (Some(sys_ffmpeg), Some(sys_ffprobe)) = (find_system_bin("ffmpeg"), find_system_bin("ffprobe")) {
        return Ok(FfmpegBins {
            ffmpeg: sys_ffmpeg,
            ffprobe: sys_ffprobe,
        });
    }

    // 3. Download
    println!("  [ffmpeg] FFmpeg not found, downloading static binaries...");
    std::fs::create_dir_all(&ff_dir)?;
    download_ffmpeg(&ff_dir)?;

    // Verify
    let ffmpeg = ff_dir.join(ffmpeg_binary_name());
    let ffprobe = ff_dir.join(ffprobe_binary_name());
    if !ffmpeg.exists() || !ffprobe.exists() {
        return Err(anyhow!(
            "FFmpeg download completed but binaries not found at {}. \
             Install manually: brew install ffmpeg (macOS) or download from https://ffmpeg.org/download.html",
            ff_dir.display()
        ));
    }

    Ok(FfmpegBins { ffmpeg, ffprobe })
}

/// Check if ffmpeg is already available (bundled or system) without downloading.
pub fn find_ffmpeg(base_dir: &Path) -> Option<FfmpegBins> {
    let ff_dir = ffmpeg_home(base_dir);
    let bundled_ffmpeg = ff_dir.join(ffmpeg_binary_name());
    let bundled_ffprobe = ff_dir.join(ffprobe_binary_name());
    if bundled_ffmpeg.exists() && bundled_ffprobe.exists() {
        return Some(FfmpegBins {
            ffmpeg: bundled_ffmpeg,
            ffprobe: bundled_ffprobe,
        });
    }
    if let (Some(sys_ffmpeg), Some(sys_ffprobe)) = (find_system_bin("ffmpeg"), find_system_bin("ffprobe")) {
        return Some(FfmpegBins {
            ffmpeg: sys_ffmpeg,
            ffprobe: sys_ffprobe,
        });
    }
    None
}

// ─── Internals ─────────────────────────────────────────────────────────────────

fn ffmpeg_home(base_dir: &Path) -> PathBuf {
    base_dir.join(".ffmpeg")
}

fn ffmpeg_binary_name() -> &'static str {
    if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" }
}

fn ffprobe_binary_name() -> &'static str {
    if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" }
}

/// Search system PATH for a binary.
fn find_system_bin(name: &str) -> Option<PathBuf> {
    let cmd = if cfg!(windows) { "where" } else { "which" };
    Command::new(cmd).arg(name).output().ok().and_then(|o| {
        if o.status.success() {
            let path = String::from_utf8_lossy(&o.stdout)
                .trim()
                .lines()
                .next()?
                .to_string();
            if !path.is_empty() {
                Some(PathBuf::from(path))
            } else {
                None
            }
        } else {
            None
        }
    })
}

/// Download FFmpeg static binaries into `dest_dir`.
fn download_ffmpeg(dest_dir: &Path) -> Result<()> {
    if cfg!(target_os = "macos") {
        download_ffmpeg_macos(dest_dir)
    } else if cfg!(windows) {
        download_ffmpeg_windows(dest_dir)
    } else {
        download_ffmpeg_linux(dest_dir)
    }
}

// ─── macOS ─────────────────────────────────────────────────────────────────────

/// Download FFmpeg + FFprobe for macOS from evermeet.cx (static builds).
fn download_ffmpeg_macos(dest_dir: &Path) -> Result<()> {
    let ffmpeg_url = "https://evermeet.cx/ffmpeg/getrelease/zip";
    let ffprobe_url = "https://evermeet.cx/ffmpeg/getrelease/ffprobe/zip";

    // Helper: download a zip, extract the binary
    let download_and_extract = |url: &str, binary_name: &str| -> Result<()> {
        let zip_path = dest_dir.join(format!("{binary_name}.zip"));

        println!("  [ffmpeg] Downloading {binary_name} from evermeet.cx...");
        let status = Command::new("curl")
            .args(["-fSL", "--retry", "3", "--retry-delay", "2", "-o"])
            .arg(zip_path.as_os_str())
            .arg(url)
            .status()
            .context("Failed to run curl")?;

        if !status.success() {
            return Err(anyhow!("curl failed to download {binary_name}"));
        }

        // Unzip
        let status = Command::new("unzip")
            .args(["-o", "-q"])
            .arg(zip_path.as_os_str())
            .arg("-d")
            .arg(dest_dir.as_os_str())
            .status()
            .context("Failed to run unzip")?;

        if !status.success() {
            return Err(anyhow!("unzip failed for {binary_name}"));
        }

        // Make executable
        let bin = dest_dir.join(binary_name);
        if bin.exists() {
            let _ = Command::new("chmod").arg("+x").arg(bin.as_os_str()).status();
        }

        // Clean up zip
        let _ = std::fs::remove_file(&zip_path);
        Ok(())
    };

    download_and_extract(ffmpeg_url, "ffmpeg")?;
    download_and_extract(ffprobe_url, "ffprobe")?;
    println!("  [ffmpeg] FFmpeg installed to {}", dest_dir.display());
    Ok(())
}

// ─── Windows ───────────────────────────────────────────────────────────────────

/// Download FFmpeg for Windows from BtbN GitHub releases.
fn download_ffmpeg_windows(dest_dir: &Path) -> Result<()> {
    let url = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip";
    let zip_path = dest_dir.join("ffmpeg-win64.zip");
    let extract_dir = dest_dir.join("ffmpeg-extract");

    println!("  [ffmpeg] Downloading FFmpeg from BtbN/FFmpeg-Builds...");

    let status = Command::new("powershell")
        .args([
            "-NoProfile", "-Command",
            &format!(
                "Invoke-WebRequest -Uri '{}' -OutFile '{}' -UseBasicParsing",
                url, zip_path.to_string_lossy()
            ),
        ])
        .status()
        .context("Failed to run PowerShell download")?;

    if !status.success() {
        return Err(anyhow!("Failed to download FFmpeg for Windows"));
    }

    // Extract
    let status = Command::new("powershell")
        .args([
            "-NoProfile", "-Command",
            &format!(
                "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                zip_path.to_string_lossy(), extract_dir.to_string_lossy()
            ),
        ])
        .status()
        .context("Failed to extract FFmpeg zip")?;

    if !status.success() {
        return Err(anyhow!("Failed to extract FFmpeg zip"));
    }

    // Find and move binaries — BtbN zips have nested directories
    move_binaries_from_extract(&extract_dir, dest_dir, &["ffmpeg.exe", "ffprobe.exe"])?;

    // Clean up
    let _ = std::fs::remove_file(&zip_path);
    let _ = std::fs::remove_dir_all(&extract_dir);

    println!("  [ffmpeg] FFmpeg installed to {}", dest_dir.display());
    Ok(())
}

// ─── Linux ─────────────────────────────────────────────────────────────────────

/// Download FFmpeg for Linux from BtbN GitHub releases.
fn download_ffmpeg_linux(dest_dir: &Path) -> Result<()> {
    let arch_slug = if cfg!(target_arch = "aarch64") {
        "linuxarm64"
    } else {
        "linux64"
    };
    let url = format!(
        "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-{arch_slug}-gpl.tar.xz"
    );
    let tar_path = dest_dir.join("ffmpeg-linux.tar.xz");
    let extract_dir = dest_dir.join("ffmpeg-extract");

    println!("  [ffmpeg] Downloading FFmpeg from BtbN/FFmpeg-Builds...");

    let status = Command::new("curl")
        .args(["-fSL", "--retry", "3", "--retry-delay", "2", "-o"])
        .arg(tar_path.as_os_str())
        .arg(&url)
        .status()
        .context("Failed to run curl")?;

    if !status.success() {
        return Err(anyhow!("curl failed to download FFmpeg for Linux"));
    }

    std::fs::create_dir_all(&extract_dir)?;

    let status = Command::new("tar")
        .args(["-xf"])
        .arg(tar_path.as_os_str())
        .arg("-C")
        .arg(extract_dir.as_os_str())
        .status()
        .context("Failed to extract FFmpeg tar")?;

    if !status.success() {
        return Err(anyhow!("tar extraction failed"));
    }

    move_binaries_from_extract(&extract_dir, dest_dir, &["ffmpeg", "ffprobe"])?;

    // Clean up
    let _ = std::fs::remove_file(&tar_path);
    let _ = std::fs::remove_dir_all(&extract_dir);

    println!("  [ffmpeg] FFmpeg installed to {}", dest_dir.display());
    Ok(())
}

/// Recursively find named binaries inside `extract_dir` and move them to `dest_dir`.
fn move_binaries_from_extract(
    extract_dir: &Path,
    dest_dir: &Path,
    binary_names: &[&str],
) -> Result<()> {
    for name in binary_names {
        if let Some(found) = find_file_recursive(extract_dir, name) {
            let target = dest_dir.join(name);
            std::fs::copy(&found, &target).with_context(|| {
                format!("Failed to copy {} to {}", found.display(), target.display())
            })?;
            // Make executable on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755));
            }
        } else {
            return Err(anyhow!(
                "Could not find {name} in extracted archive at {}",
                extract_dir.display()
            ));
        }
    }
    Ok(())
}

/// Walk a directory tree to find a file by name.
fn find_file_recursive(dir: &Path, name: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file_recursive(&path, name) {
                return Some(found);
            }
        } else if path.file_name().and_then(|n| n.to_str()) == Some(name) {
            return Some(path);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffmpeg_home_is_inside_base_dir() {
        let home = ffmpeg_home(Path::new("/test/base"));
        assert_eq!(home, PathBuf::from("/test/base/.ffmpeg"));
    }

    #[test]
    fn binary_names_are_platform_correct() {
        let ffmpeg = ffmpeg_binary_name();
        let ffprobe = ffprobe_binary_name();
        if cfg!(windows) {
            assert!(ffmpeg.ends_with(".exe"));
            assert!(ffprobe.ends_with(".exe"));
        } else {
            assert!(!ffmpeg.contains('.'));
            assert!(!ffprobe.contains('.'));
        }
    }
}
