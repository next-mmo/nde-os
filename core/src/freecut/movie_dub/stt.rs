//! Speech-to-text module for movie dubbing.
//!
//! Delegates to the NDE-OS VoiceRuntime for Whisper CLI resolution and uses
//! the sandboxed FFmpeg for audio extraction/conversion. Never shells out to
//! raw host OS binaries.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tracing::info;

use super::lang::Lang;
use super::segment::Segment;

/// STT engine backed by VoiceRuntime-managed Whisper.
pub struct SttEngine {
    pub model_size: String,
    pub work_dir: PathBuf,
    /// Path to the sandboxed ffmpeg binary.
    ffmpeg_path: PathBuf,
    /// Path to the Whisper CLI binary (resolved via VoiceRuntime).
    whisper_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct WhisperSegment {
    start: f64,
    end: f64,
    text: String,
}

#[derive(Debug, Deserialize)]
struct WhisperOutput {
    #[allow(dead_code)]
    text: Option<String>,
    segments: Option<Vec<WhisperSegment>>,
    language: Option<String>,
}

impl SttEngine {
    /// Create a new STT engine using NDE-OS sandboxed dependencies.
    ///
    /// `ffmpeg_path` — resolved from `crate::media::ffmpeg::ensure_ffmpeg()`.
    /// `whisper_path` — resolved from `VoiceRuntime::resolve_tool("whisper")`.
    pub fn new(
        model_size: &str,
        work_dir: impl Into<PathBuf>,
        ffmpeg_path: PathBuf,
        whisper_path: Option<PathBuf>,
    ) -> Self {
        Self {
            model_size: model_size.to_string(),
            work_dir: work_dir.into(),
            ffmpeg_path,
            whisper_path,
        }
    }

    /// Transcribe audio file → segments with timestamps.
    pub async fn transcribe(
        &self,
        audio_path: &Path,
        source_lang: Option<Lang>,
    ) -> Result<Vec<Segment>> {
        let whisper = self.whisper_path.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "Whisper CLI not found. Install via Service Hub → Voice Runtime, \
                 or run: pip install openai-whisper"
            )
        })?;

        std::fs::create_dir_all(&self.work_dir)?;

        // Convert to 16kHz mono WAV for Whisper.
        let wav_path = self.work_dir.join("input_16k.wav");
        self.convert_to_16k_wav(audio_path, &wav_path).await?;

        // Run Whisper.
        let output_dir = self.work_dir.join("whisper_out");
        std::fs::create_dir_all(&output_dir)?;

        let mut cmd = tokio::process::Command::new(whisper);
        
        // Whisper requires ffmpeg in the system PATH
        if let Some(ffmpeg_dir) = self.ffmpeg_path.parent() {
            let current_path = std::env::var("PATH").unwrap_or_default();
            cmd.env("PATH", format!("{}:{}", ffmpeg_dir.display(), current_path));
        }

        cmd.arg(&wav_path)
            .arg("--model").arg(&self.model_size)
            .arg("--output_format").arg("json")
            .arg("--output_dir").arg(&output_dir)
            .arg("--word_timestamps").arg("True");

        if let Some(l) = source_lang {
            cmd.arg("--language").arg(l.code());
        }

        info!("Running Whisper (model={})...", self.model_size);
        let output = cmd.output().await.context("Failed to run whisper")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("whisper failed: {stderr}");
        }

        let stem = wav_path.file_stem().unwrap().to_str().unwrap();
        let json_path = output_dir.join(format!("{stem}.json"));
        parse_whisper_json(&json_path, source_lang)
    }

    /// Extract audio track from a video file → WAV (mono, 44.1kHz).
    pub async fn extract_audio(&self, video_path: &Path, output_wav: &Path) -> Result<()> {
        info!("Extracting audio: {}", video_path.display());

        if let Some(parent) = output_wav.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let status = tokio::process::Command::new(&self.ffmpeg_path)
            .args(["-y", "-i"])
            .arg(video_path)
            .args(["-vn", "-ar", "44100", "-ac", "1", "-f", "wav"])
            .arg(output_wav)
            .status()
            .await
            .context("Failed to run sandboxed ffmpeg")?;

        if !status.success() {
            anyhow::bail!("ffmpeg audio extraction failed");
        }
        Ok(())
    }

    /// Convert audio to 16kHz mono WAV for Whisper input.
    async fn convert_to_16k_wav(&self, input: &Path, output: &Path) -> Result<()> {
        let status = tokio::process::Command::new(&self.ffmpeg_path)
            .args(["-y", "-i"])
            .arg(input)
            .args(["-ar", "16000", "-ac", "1", "-f", "wav"])
            .arg(output)
            .status()
            .await
            .context("Failed to run sandboxed ffmpeg for 16kHz conversion")?;

        if !status.success() {
            anyhow::bail!("16kHz WAV conversion failed");
        }
        Ok(())
    }
}

/// Load pre-transcribed segments from a JSON file.
pub fn load_segments_from_json(path: &Path) -> Result<Vec<Segment>> {
    let data = std::fs::read_to_string(path)
        .with_context(|| format!("Cannot read: {}", path.display()))?;

    // Try the Whisper JSON format first.
    if let Ok(whisper) = serde_json::from_str::<WhisperOutput>(&data) {
        if let Some(segs) = whisper.segments {
            let detected_lang = whisper.language.as_deref().and_then(|l| match l {
                "en" | "english" => Some(Lang::En),
                "zh" | "chinese" => Some(Lang::Zh),
                "km" | "khmer" => Some(Lang::Km),
                _ => None,
            });
            return Ok(segs
                .into_iter()
                .enumerate()
                .filter(|(_, s)| !s.text.trim().is_empty())
                .map(|(i, s)| Segment {
                    id: i as u32,
                    start_ms: (s.start * 1000.0) as u64,
                    end_ms: (s.end * 1000.0) as u64,
                    source_text: s.text.trim().to_string(),
                    source_lang: detected_lang.unwrap_or(Lang::En),
                    speaker_id: None,
                })
                .collect());
        }
    }

    // Try raw segment array format.
    #[derive(Deserialize)]
    struct RawSegment {
        start_ms: u64,
        end_ms: u64,
        text: String,
        lang: Option<String>,
        speaker_id: Option<u32>,
    }

    let raw: Vec<RawSegment> = serde_json::from_str(&data)?;
    Ok(raw.into_iter().enumerate().map(|(i, r)| {
        let lang = match r.lang.as_deref() {
            Some("zh" | "chinese") => Lang::Zh,
            Some("km" | "khmer") => Lang::Km,
            _ => Lang::En,
        };
        Segment {
            id: i as u32,
            start_ms: r.start_ms,
            end_ms: r.end_ms,
            source_text: r.text,
            source_lang: lang,
            speaker_id: r.speaker_id,
        }
    }).collect())
}

/// Parse Whisper JSON output into segments.
fn parse_whisper_json(path: &Path, lang: Option<Lang>) -> Result<Vec<Segment>> {
    let data = std::fs::read_to_string(path)
        .with_context(|| format!("Cannot read: {}", path.display()))?;

    let whisper: WhisperOutput = serde_json::from_str(&data)?;

    let detected_lang = whisper.language.as_deref().and_then(|l| match l {
        "en" | "english" => Some(Lang::En),
        "zh" | "chinese" => Some(Lang::Zh),
        "km" | "khmer" => Some(Lang::Km),
        _ => None,
    });
    let source_lang = lang.or(detected_lang).unwrap_or(Lang::En);

    let segments: Vec<Segment> = whisper.segments.unwrap_or_default()
        .into_iter()
        .enumerate()
        .filter(|(_, s)| !s.text.trim().is_empty())
        .map(|(i, s)| Segment {
            id: i as u32,
            start_ms: (s.start * 1000.0) as u64,
            end_ms: (s.end * 1000.0) as u64,
            source_text: s.text.trim().to_string(),
            source_lang,
            speaker_id: None,
        })
        .collect();

    info!("Parsed {} segments from whisper JSON", segments.len());
    Ok(segments)
}
