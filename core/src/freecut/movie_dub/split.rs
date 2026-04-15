//! Video splitting for the FreeCut dubbing workstation.
//!
//! Splits a video into N equal parts, runs Whisper STT + Lingva translation
//! on each part, and produces per-part SRT files ready for user editing.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

use super::config::MovieDubConfig;
use super::lang::Lang;
use super::stt::SttEngine;
use super::translate::Translator;
use super::mix;

/// Status of a single split part.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PartStatus {
    Pending,
    SrtReady,
    Dubbed,
    Error,
}

/// A single split part with its associated files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitPart {
    pub index: usize,
    pub video_path: PathBuf,
    pub start_secs: f64,
    pub end_secs: f64,
    pub duration_secs: f64,
    pub orig_srt_path: PathBuf,
    pub translated_srt_path: PathBuf,
    pub dubbed_path: Option<PathBuf>,
    pub status: PartStatus,
    pub error: Option<String>,
}

/// A complete split job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitJob {
    pub id: String,
    pub input_path: PathBuf,
    pub workspace: PathBuf,
    pub segment_duration_secs: u64,
    pub total_duration_secs: f64,
    pub total_parts: usize,
    pub parts: Vec<SplitPart>,
    pub created_at: String,
}

/// Request to split a video.
#[derive(Debug, Clone, Deserialize)]
pub struct SplitRequest {
    pub input_path: String,
    pub segment_duration_secs: u64,
    pub target_lang: Option<String>,
}

impl SplitJob {
    /// Persist job state to disk.
    pub fn save(&self) -> Result<()> {
        let path = self.workspace.join("job.json");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    /// Load job state from disk.
    pub fn load(workspace: &Path) -> Result<Self> {
        let path = workspace.join("job.json");
        let json = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read job.json from {}", workspace.display()))?;
        let job: SplitJob = serde_json::from_str(&json)?;
        Ok(job)
    }
}

/// Get video duration in seconds using ffprobe.
pub async fn get_video_duration(ffmpeg_path: &Path, video_path: &Path) -> Result<f64> {
    let ffprobe = ffmpeg_path.parent()
        .unwrap_or(Path::new(""))
        .join("ffprobe");

    let output = tokio::process::Command::new(&ffprobe)
        .args([
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(video_path)
        .output()
        .await
        .context("Failed to run ffprobe")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let duration: f64 = stdout.trim().parse()
        .context("Failed to parse video duration")?;

    Ok(duration)
}

/// Split a video into N parts using FFmpeg stream copy (no re-encoding).
async fn split_video_part(
    ffmpeg_path: &Path,
    input: &Path,
    output: &Path,
    start_secs: f64,
    duration_secs: f64,
) -> Result<()> {
    let status = tokio::process::Command::new(ffmpeg_path)
        .args([
            "-y",
            "-ss", &format!("{:.3}", start_secs),
            "-t", &format!("{:.3}", duration_secs),
            "-i",
        ])
        .arg(input)
        .args(["-c", "copy"])
        .arg(output)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .context("Failed to run ffmpeg split")?;

    if !status.success() {
        anyhow::bail!("FFmpeg split failed with exit code: {:?}", status.code());
    }
    Ok(())
}

/// Execute the full split workflow:
/// 1. FFprobe → get total duration
/// 2. Split into N parts via FFmpeg
/// 3. Whisper STT each part → original SRT
/// 4. Lingva translate each part → translated SRT
pub async fn execute_split(
    config: &MovieDubConfig,
    ffmpeg_path: &Path,
    whisper_path: &Path,
    data_dir: &Path,
    input_path: &Path,
    segment_duration_secs: u64,
    target_lang: Lang,
    progress: impl Fn(usize, usize, &str),
) -> Result<SplitJob> {
    // 1. Get total duration.
    let total_duration = get_video_duration(ffmpeg_path, input_path).await?;
    let total_parts = (total_duration / segment_duration_secs as f64).ceil() as usize;

    info!(
        "Splitting {}s video into {} parts of {}s each",
        total_duration, total_parts, segment_duration_secs
    );

    // 2. Create job workspace.
    let job_id = uuid::Uuid::new_v4().to_string();
    let workspace = data_dir
        .join("movie_dub_workspace")
        .join("jobs")
        .join(&job_id);
    std::fs::create_dir_all(&workspace)?;

    let mut parts = Vec::with_capacity(total_parts);

    for i in 0..total_parts {
        let start = i as f64 * segment_duration_secs as f64;
        let remaining = total_duration - start;
        let dur = (segment_duration_secs as f64).min(remaining);

        let part_dir = workspace.join(format!("part_{:03}", i + 1));
        std::fs::create_dir_all(&part_dir)?;

        let video_path = part_dir.join(format!("part_{:03}.mp4", i + 1));
        let orig_srt = part_dir.join(format!("part_{:03}_orig.srt", i + 1));
        let translated_srt = part_dir.join(format!("part_{:03}_km.srt", i + 1));

        parts.push(SplitPart {
            index: i,
            video_path,
            start_secs: start,
            end_secs: start + dur,
            duration_secs: dur,
            orig_srt_path: orig_srt,
            translated_srt_path: translated_srt,
            dubbed_path: None,
            status: PartStatus::Pending,
            error: None,
        });
    }

    // 3. Split video files.
    for part in &parts {
        progress(part.index, total_parts, &format!("Splitting part {}/{}...", part.index + 1, total_parts));
        split_video_part(
            ffmpeg_path,
            input_path,
            &part.video_path,
            part.start_secs,
            part.duration_secs,
        ).await?;
    }

    // 4. STT + Translate each part.
    let translator = Translator::from_config(&config.translation);

    for part in parts.iter_mut() {
        let part_dir = part.video_path.parent().unwrap();
        let stt_work = part_dir.join("stt_work");
        std::fs::create_dir_all(&stt_work)?;

        // Extract audio.
        progress(part.index, total_parts, &format!("Transcribing part {}/{}...", part.index + 1, total_parts));
        let audio_wav = part_dir.join("extracted_audio.wav");
        let stt = SttEngine::new(
            &config.stt.model_size,
            stt_work,
            ffmpeg_path.to_path_buf(),
            Some(whisper_path.to_path_buf()),
        );
        stt.extract_audio(&part.video_path, &audio_wav).await?;

        // Transcribe.
        match stt.transcribe(&audio_wav, Some(Lang::En)).await {
            Ok(segments) => {
                // Generate original SRT.
                let orig_entries: Vec<(u64, u64, String)> = segments.iter()
                    .map(|s| (s.start_ms, s.end_ms, s.source_text.clone()))
                    .collect();
                mix::generate_srt(&orig_entries, &part.orig_srt_path)?;

                // Translate.
                progress(part.index, total_parts, &format!("Translating part {}/{}...", part.index + 1, total_parts));
                match translator.translate_segments(&segments, target_lang).await {
                    Ok(timed_texts) => {
                        let km_entries: Vec<(u64, u64, String)> = timed_texts.iter()
                            .map(|t| (t.segment.start_ms, t.segment.end_ms, t.translated_text.clone()))
                            .collect();
                        mix::generate_srt(&km_entries, &part.translated_srt_path)?;
                        part.status = PartStatus::SrtReady;
                    }
                    Err(e) => {
                        warn!("Translation failed for part {}: {}", part.index + 1, e);
                        part.status = PartStatus::Error;
                        part.error = Some(format!("Translation failed: {}", e));
                    }
                }
            }
            Err(e) => {
                warn!("STT failed for part {}: {}", part.index + 1, e);
                part.status = PartStatus::Error;
                part.error = Some(format!("Transcription failed: {}", e));
            }
        }
    }

    let job = SplitJob {
        id: job_id,
        input_path: input_path.to_path_buf(),
        workspace,
        segment_duration_secs,
        total_duration_secs: total_duration,
        total_parts,
        parts,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    job.save()?;
    Ok(job)
}
