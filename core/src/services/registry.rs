//! Service registry — detects and installs all NDE-OS service dependencies.

use anyhow::Result;
use std::path::Path;

use super::types::{ServiceGroup, ServiceStatus};
use crate::openviking::VikingProcess;
use crate::voice::runtime::VoiceRuntime;

/// Detect the status of all known NDE-OS services.
pub fn detect_all(base_dir: &Path) -> Vec<ServiceStatus> {
    let voice_rt = VoiceRuntime::new(base_dir);
    let voice_status = voice_rt.detect_status();

    // FFmpeg: check sandboxed install first, then system PATH
    let ffmpeg_bins = crate::media::ffmpeg::find_ffmpeg(base_dir);
    let ffmpeg_installed = ffmpeg_bins.is_some();
    let python_installed = crate::voice::runtime::resolve_python().is_some();
    let uv_installed = crate::voice::runtime::resolve_system_command("uv").is_some()
        || base_dir.join("uv").exists()
        || base_dir.join("uv.exe").exists();

    // OpenViking: detect if the package is installed in the workspace venv
    let viking_config = crate::openviking::config::VikingConfig::from_service_config(base_dir);
    let viking_port = viking_config.port;
    let viking_process = VikingProcess::new(viking_config, base_dir);
    let viking_installed = viking_process.is_installed_sync();

    let vision_rt = crate::freecut::vision::VisionRuntime::new(base_dir);
    let vision_installed = vision_rt.is_installed();

    // LDPlayer: detect if the Android emulator is installed on this system
    let ld_detection = crate::shield::ldplayer::detect_ldplayer();

    vec![
        ServiceStatus {
            id: "voice-runtime".to_string(),
            name: "Voice Runtime".to_string(),
            description: "Edge TTS + Whisper for speech synthesis and transcription".to_string(),
            group: ServiceGroup::Voice,
            installed: voice_status.edge_tts_available && voice_status.whisper_available,
            version: None,
            path: voice_status.runtime_path.clone(),
            used_by: vec!["FreeCut".to_string()],
            optional: true,
            details: if voice_status.edge_tts_available && voice_status.whisper_available {
                Some("Edge TTS + Whisper ready".to_string())
            } else {
                let mut missing = Vec::new();
                if !voice_status.edge_tts_available {
                    missing.push("Edge TTS");
                }
                if !voice_status.whisper_available {
                    missing.push("Whisper");
                }
                Some(format!("Missing: {}", missing.join(", ")))
            },
        },
        ServiceStatus {
            id: "rvc".to_string(),
            name: "RVC Voice Conversion".to_string(),
            description: "Retrieval-based Voice Conversion for voice cloning".to_string(),
            group: ServiceGroup::Voice,
            installed: voice_status.rvc_available,
            version: None,
            path: None,
            used_by: vec!["FreeCut".to_string()],
            optional: true,
            details: if voice_status.rvc_available {
                Some("Python available for RVC CLI".to_string())
            } else {
                Some("Requires Python".to_string())
            },
        },
        ServiceStatus {
            id: "ffmpeg".to_string(),
            name: "FFmpeg".to_string(),
            description: "Video/audio processing, encoding, and transcoding".to_string(),
            group: ServiceGroup::Media,
            installed: ffmpeg_installed,
            version: detect_ffmpeg_version(ffmpeg_bins.as_ref()),
            path: ffmpeg_bins.as_ref()
                .map(|bins| bins.ffmpeg.to_string_lossy().to_string()),
            used_by: vec!["FreeCut".to_string()],
            optional: false,
            details: if ffmpeg_installed {
                Some("ffmpeg + ffprobe ready".to_string())
            } else {
                Some("Click Install to auto-download FFmpeg".to_string())
            },
        },
        ServiceStatus {
            id: "python".to_string(),
            name: "Python".to_string(),
            description: "Python runtime for AI tools and scripts".to_string(),
            group: ServiceGroup::Tooling,
            installed: python_installed,
            version: detect_python_version(),
            path: crate::voice::runtime::resolve_python()
                .map(|p| p.to_string_lossy().to_string()),
            used_by: vec!["Voice Runtime".to_string(), "RVC".to_string()],
            optional: true,
            details: None,
        },
        ServiceStatus {
            id: "uv".to_string(),
            name: "uv Package Manager".to_string(),
            description: "Ultra-fast Python package manager (bundled by NDE-OS)".to_string(),
            group: ServiceGroup::Tooling,
            installed: uv_installed,
            version: None,
            path: None,
            used_by: vec!["Voice Runtime".to_string()],
            optional: false,
            details: if uv_installed {
                Some("Bundled with NDE-OS".to_string())
            } else {
                Some("Will be auto-bootstrapped on first install".to_string())
            },
        },
        ServiceStatus {
            id: "openviking".to_string(),
            name: "OpenViking".to_string(),
            description: "Context database for agent memory, resources & skills (semantic search, virtual FS)".to_string(),
            group: ServiceGroup::Ai,
            installed: viking_installed,
            version: None,
            path: None,
            used_by: vec!["Agent Chat".to_string(), "MCP Tools".to_string()],
            optional: false,
            details: if viking_installed {
                Some(format!("Port {}", viking_port))
            } else {
                Some("Not installed — will auto-install on first boot".to_string())
            },
        },
        ServiceStatus {
            id: "ai-vision-runtime".to_string(),
            name: "AI Vision Runtime".to_string(),
            description: "On-device AI vision models for automatic background removal and tracking (rembg)".to_string(),
            group: ServiceGroup::Ai,
            installed: vision_installed,
            version: None,
            path: None,
            used_by: vec!["FreeCut".to_string()],
            optional: true,
            details: if vision_installed { 
                Some("Ready".to_string()) 
            } else { 
                Some("Requires installation via Service Hub".to_string()) 
            },
        },
        ServiceStatus {
            id: "ldplayer".to_string(),
            name: "LDPlayer".to_string(),
            description: "Android emulator for mobile testing and anti-detect browsing".to_string(),
            group: ServiceGroup::Tooling,
            installed: ld_detection.available,
            version: ld_detection.version_dir.clone(),
            path: ld_detection.ldconsole_path.clone(),
            used_by: vec!["Shield Browser".to_string()],
            optional: true,
            details: if ld_detection.available {
                Some(format!(
                    "Detected: {}",
                    ld_detection.version_dir.as_deref().unwrap_or("LDPlayer")
                ))
            } else {
                Some("Install from ldplayer.net — required for emulator management".to_string())
            },
        },
    ]
}

/// Install a specific service by ID.
pub fn install_service(service_id: &str, base_dir: &Path) -> Result<String> {
    match service_id {
        "voice-runtime" => {
            let rt = VoiceRuntime::new(base_dir);
            let result = rt.install_components(&["core".to_string()])?;
            Ok(result.message)
        }
        "uv" => {
            crate::uv_env::ensure_uv(base_dir)?;
            Ok("uv bootstrapped successfully".to_string())
        }
        "openviking" => {
            let config = crate::openviking::config::VikingConfig::from_service_config(base_dir);
            let vp = VikingProcess::new(config, base_dir);
            let rt = tokio::runtime::Handle::try_current()
                .map(|_| None)
                .unwrap_or_else(|_| Some(tokio::runtime::Runtime::new().expect("tokio runtime")));
            let result = if let Some(ref rt) = rt {
                rt.block_on(vp.ensure_installed())
            } else {
                // We're inside spawn_blocking called from an async runtime
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(vp.ensure_installed())
                })
            };
            result?;
            Ok("OpenViking installed successfully".to_string())
        }
        "ai-vision-runtime" => {
            let rt = crate::freecut::vision::VisionRuntime::new(base_dir);
            rt.install()?;
            Ok("AI Vision models installed successfully".to_string())
        }
        "ffmpeg" => {
            let bins = crate::media::ffmpeg::ensure_ffmpeg(base_dir)?;
            Ok(format!(
                "FFmpeg installed to {}",
                bins.ffmpeg.parent().unwrap_or(base_dir).display()
            ))
        }
        "python" => {
            anyhow::bail!("Python must be installed via your system package manager or python.org")
        }
        "rvc" => {
            anyhow::bail!("RVC requires manual setup: clone the RVC repo and configure model paths in the app")
        }
        "ldplayer" => {
            anyhow::bail!("LDPlayer must be installed manually from https://www.ldplayer.net — download and install, then restart NDE-OS")
        }
        _ => anyhow::bail!("Unknown service: {service_id}"),
    }
}

fn detect_ffmpeg_version(
    bins: Option<&crate::media::ffmpeg::FfmpegBins>,
) -> Option<String> {
    let ffmpeg_path = bins
        .map(|b| b.ffmpeg.to_string_lossy().to_string())
        .unwrap_or_else(|| "ffmpeg".to_string());
    let output = std::process::Command::new(&ffmpeg_path)
        .arg("-version")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    // First line: "ffmpeg version N.N.N ..."
    stdout
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(2))
        .map(|v| v.to_string())
}

fn detect_python_version() -> Option<String> {
    let python = crate::voice::runtime::resolve_python()?;
    let output = std::process::Command::new(python)
        .arg("--version")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    // "Python 3.11.x"
    stdout.trim().strip_prefix("Python ").map(|v| v.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_all_returns_known_services() {
        let statuses = detect_all(std::path::Path::new("/tmp/nde-test-nonexistent"));
        let ids: Vec<&str> = statuses.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&"voice-runtime"));
        assert!(ids.contains(&"ffmpeg"));
        assert!(ids.contains(&"python"));
        assert!(ids.contains(&"uv"));
        assert!(ids.contains(&"rvc"));
        assert!(ids.contains(&"openviking"));
        assert!(ids.contains(&"ai-vision-runtime"));
        assert!(ids.contains(&"ldplayer"));
    }
}
