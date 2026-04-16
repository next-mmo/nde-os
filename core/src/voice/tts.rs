//! Edge TTS synthesis — wraps the `edge-tts` CLI as a global NDE-OS voice service.

use anyhow::{anyhow, Result};
use std::process::Command;

use super::runtime::{run_checked_command, VoiceRuntime};
use super::types::{TtsSynthesizeRequest, TtsSynthesizeResult};

/// Synthesize speech from text using Edge TTS.
pub fn synthesize(
    request: &TtsSynthesizeRequest,
    runtime: &VoiceRuntime,
) -> Result<TtsSynthesizeResult> {
    runtime.with_runtime_path(|| {
        let edge_tts = runtime
            .resolve_tool("edge-tts")
            .ok_or_else(|| anyhow!("edge-tts CLI not found; install via Service Hub"))?;

        let voice = if request.voice.trim().is_empty() {
            default_voice_for_language("en")
        } else {
            request.voice.clone()
        };

        let out_path = match &request.output_path {
            Some(p) => std::path::PathBuf::from(p),
            None => {
                let tmp =
                    std::env::temp_dir().join(format!("nde-tts-{}.mp3", uuid::Uuid::new_v4()));
                tmp
            }
        };

        // Ensure parent dir exists
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let mut command = Command::new(edge_tts);
        command.arg("--voice").arg(&voice);
        command.arg("--text").arg(&request.text);
        command.arg("--write-media").arg(&out_path);

        if let Some(rate) = request.rate.as_deref().filter(|v| !v.trim().is_empty()) {
            command.arg(format!("--rate={rate}"));
        }
        if let Some(pitch) = request.pitch.as_deref().filter(|v| !v.trim().is_empty()) {
            command.arg(format!("--pitch={pitch}"));
        }
        if let Some(volume) = request.volume.as_deref().filter(|v| !v.trim().is_empty()) {
            command.arg(format!("--volume={volume}"));
        }

        run_checked_command(command, "edge-tts synthesis")?;

        Ok(TtsSynthesizeResult {
            audio_path: out_path.to_string_lossy().to_string(),
            voice,
        })
    })
}

/// List available Edge TTS voices via the runtime.
pub fn list_voices(runtime: &VoiceRuntime) -> Vec<String> {
    let status = runtime.detect_status();
    status.voices
}

/// Default voice for a given language code.
pub fn default_voice_for_language(language: &str) -> String {
    match language.to_ascii_lowercase().as_str() {
        "ja" | "jp" => "ja-JP-NanamiNeural".to_string(),
        "ko" => "ko-KR-SunHiNeural".to_string(),
        "zh" | "zh-cn" => "zh-CN-XiaoxiaoNeural".to_string(),
        "es" => "es-ES-ElviraNeural".to_string(),
        "fr" => "fr-FR-DeniseNeural".to_string(),
        "de" => "de-DE-KatjaNeural".to_string(),
        "th" => "th-TH-PremwadeeNeural".to_string(),
        _ => "en-US-AriaNeural".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_voice_english() {
        assert_eq!(default_voice_for_language("en"), "en-US-AriaNeural");
    }

    #[test]
    fn default_voice_japanese() {
        assert_eq!(default_voice_for_language("ja"), "ja-JP-NanamiNeural");
    }

    #[test]
    fn default_voice_korean() {
        assert_eq!(default_voice_for_language("ko"), "ko-KR-SunHiNeural");
    }

    #[test]
    fn default_voice_unknown_falls_back_to_english() {
        assert_eq!(default_voice_for_language("xx"), "en-US-AriaNeural");
    }
}
