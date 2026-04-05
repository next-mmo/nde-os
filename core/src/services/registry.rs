//! Service registry — detects and installs all NDE-OS service dependencies.

use anyhow::Result;
use std::path::Path;

use super::types::{ServiceGroup, ServiceStatus};
use crate::voice::runtime::VoiceRuntime;

/// Detect the status of all known NDE-OS services.
pub fn detect_all(base_dir: &Path) -> Vec<ServiceStatus> {
    let voice_rt = VoiceRuntime::new(base_dir);
    let voice_status = voice_rt.detect_status();

    let ffmpeg_installed = crate::voice::runtime::resolve_system_command("ffmpeg").is_some();
    let ffprobe_installed = crate::voice::runtime::resolve_system_command("ffprobe").is_some();
    let python_installed = crate::voice::runtime::resolve_python().is_some();
    let uv_installed = crate::voice::runtime::resolve_system_command("uv").is_some()
        || base_dir.join("uv").exists()
        || base_dir.join("uv.exe").exists();

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
            installed: ffmpeg_installed && ffprobe_installed,
            version: detect_ffmpeg_version(),
            path: crate::voice::runtime::resolve_system_command("ffmpeg")
                .map(|p| p.to_string_lossy().to_string()),
            used_by: vec!["FreeCut".to_string()],
            optional: false,
            details: if ffmpeg_installed && ffprobe_installed {
                Some("ffmpeg + ffprobe ready".to_string())
            } else {
                let mut missing = Vec::new();
                if !ffmpeg_installed {
                    missing.push("ffmpeg");
                }
                if !ffprobe_installed {
                    missing.push("ffprobe");
                }
                Some(format!("Missing: {}", missing.join(", ")))
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
        "ffmpeg" => {
            anyhow::bail!("FFmpeg must be installed via your system package manager (brew install ffmpeg / apt install ffmpeg / winget install ffmpeg)")
        }
        "python" => {
            anyhow::bail!("Python must be installed via your system package manager or python.org")
        }
        "rvc" => {
            anyhow::bail!("RVC requires manual setup: clone the RVC repo and configure model paths in the app")
        }
        _ => anyhow::bail!("Unknown service: {service_id}"),
    }
}

fn detect_ffmpeg_version() -> Option<String> {
    let output = std::process::Command::new("ffmpeg")
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
    stdout
        .trim()
        .strip_prefix("Python ")
        .map(|v| v.to_string())
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
    }
}
