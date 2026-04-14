//! Audio mixing, vocal separation, video remux, and subtitle generation.
//!
//! All subprocess calls (ffmpeg, demucs) use NDE-OS sandboxed binaries:
//! - FFmpeg → resolved from `crate::media::ffmpeg::ensure_ffmpeg()`
//! - demucs → resolved from VoiceRuntime

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::info;

/// Mix parameters for combining dubbed audio with background.
pub struct MixParams {
    /// Background music/SFX volume (0.0 - 1.0).
    pub bg_volume: f32,
    /// Dubbed voice volume (0.0 - 1.0).
    pub voice_volume: f32,
    /// Total output duration in samples.
    pub total_samples: usize,
    /// Sample rate.
    pub sample_rate: u32,
}

impl Default for MixParams {
    fn default() -> Self {
        Self {
            bg_volume: 0.3,
            voice_volume: 1.0,
            total_samples: 0,
            sample_rate: 44100,
        }
    }
}

/// A placed audio segment at a specific time position.
pub struct PlacedSegment {
    /// Start position in samples.
    pub start_sample: usize,
    /// Audio data (mono f32).
    pub samples: Vec<f32>,
}

/// Convert milliseconds to sample position.
pub fn ms_to_samples(ms: u64, sample_rate: u32) -> usize {
    (sample_rate as u64 * ms / 1000) as usize
}

/// Separate audio into vocals and accompaniment using demucs (via VoiceRuntime).
///
/// `demucs_path` — the resolved demucs binary from VoiceRuntime.
pub async fn separate_audio(
    input_path: &Path,
    output_dir: &Path,
    demucs_path: Option<&Path>,
    ffmpeg_path: &Path,
) -> Result<(Vec<f32>, Vec<f32>)> {
    let demucs = demucs_path.ok_or_else(|| {
        anyhow::anyhow!(
            "demucs not found. Install via Service Hub or: pip install demucs"
        )
    })?;

    std::fs::create_dir_all(output_dir)?;

    let mut cmd = tokio::process::Command::new(demucs);
    if let Some(ffmpeg_dir) = ffmpeg_path.parent() {
        let current_path = std::env::var("PATH").unwrap_or_default();
        cmd.env("PATH", format!("{}:{}", ffmpeg_dir.display(), current_path));
    }

    let status = cmd
        .args(["--two-stems", "vocals", "-o"])
        .arg(output_dir)
        .arg(input_path)
        .status()
        .await
        .context("Failed to run demucs")?;

    if !status.success() {
        anyhow::bail!("demucs separation failed");
    }

    let stem = input_path.file_stem().unwrap().to_str().unwrap();
    let vocals_path = output_dir.join("htdemucs").join(stem).join("vocals.wav");
    let bg_path = output_dir.join("htdemucs").join(stem).join("no_vocals.wav");

    let vocals = load_mono_wav(&vocals_path)?;
    let bg = load_mono_wav(&bg_path)?;

    info!("Separated: {} vocal samples, {} bg samples", vocals.len(), bg.len());
    Ok((vocals, bg))
}

/// Mix dubbed segments with background audio.
pub fn mix_final(
    background: &[f32],
    segments: &[PlacedSegment],
    params: &MixParams,
) -> Vec<f32> {
    let total = params.total_samples.max(background.len());
    let mut output = vec![0.0f32; total];

    // Layer 1: background.
    for (i, &s) in background.iter().enumerate() {
        if i < output.len() {
            output[i] += s * params.bg_volume;
        }
    }

    // Layer 2: dubbed segments.
    for seg in segments {
        for (i, &s) in seg.samples.iter().enumerate() {
            let pos = seg.start_sample + i;
            if pos < output.len() {
                output[pos] += s * params.voice_volume;
            }
        }
    }

    // Soft clamp to [-1, 1].
    for s in &mut output {
        *s = s.clamp(-1.0, 1.0);
    }

    output
}

/// Write mono f32 samples to WAV file.
pub fn write_wav(path: &Path, samples: &[f32], sample_rate: u32) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(path, spec)
        .context("Failed to create WAV file")?;

    for &s in samples {
        let sample = (s * 32767.0).clamp(-32768.0, 32767.0) as i16;
        writer.write_sample(sample)?;
    }

    writer.finalize()?;
    info!("Wrote WAV: {} ({} samples)", path.display(), samples.len());
    Ok(())
}

/// Load WAV as mono f32 samples.
pub fn load_mono_wav(path: &Path) -> Result<Vec<f32>> {
    let reader = hound::WavReader::open(path)
        .with_context(|| format!("Cannot open {}", path.display()))?;

    let spec = reader.spec();
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => {
            reader.into_samples::<f32>()
                .filter_map(|s| s.ok())
                .collect()
        }
        hound::SampleFormat::Int => {
            let max = (1 << (spec.bits_per_sample - 1)) as f32;
            reader.into_samples::<i32>()
                .filter_map(|s| s.ok())
                .map(|s| s as f32 / max)
                .collect()
        }
    };

    // Downmix stereo to mono if needed.
    if spec.channels == 2 {
        let mono: Vec<f32> = samples.chunks(2)
            .map(|c| (c[0] + c.get(1).copied().unwrap_or(0.0)) * 0.5)
            .collect();
        Ok(mono)
    } else {
        Ok(samples)
    }
}

/// Load WAV samples with sample rate info.
pub fn load_wav_samples(path: &Path) -> Result<(Vec<f32>, u32)> {
    let reader = hound::WavReader::open(path)
        .context("Failed to open WAV")?;

    let spec = reader.spec();
    let sample_rate = spec.sample_rate;

    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => {
            reader.into_samples::<f32>()
                .filter_map(|s| s.ok())
                .collect()
        }
        hound::SampleFormat::Int => {
            let max = (1 << (spec.bits_per_sample - 1)) as f32;
            reader.into_samples::<i32>()
                .filter_map(|s| s.ok())
                .map(|s| s as f32 / max)
                .collect()
        }
    };

    Ok((samples, sample_rate))
}

/// Get WAV duration in milliseconds.
pub fn wav_duration_ms(path: &Path) -> Result<u64> {
    let reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    let num_samples = reader.len() as u64;
    let channels = spec.channels as u64;
    let rate = spec.sample_rate as u64;
    Ok((num_samples / channels) * 1000 / rate)
}

// ── Video Remux (sandboxed ffmpeg) ──

/// Replace audio track in video with dubbed audio → output MP4.
///
/// Uses sandboxed ffmpeg to:
/// 1. Copy video stream (no re-encode)
/// 2. Encode dubbed WAV → AAC
/// 3. Mux into MP4 container
pub async fn remux_video(
    ffmpeg_path: &Path,
    original_video: &Path,
    dubbed_wav: &Path,
    output_mp4: &Path,
) -> Result<()> {
    info!(
        "Remuxing: video={} + audio={} → {}",
        original_video.display(),
        dubbed_wav.display(),
        output_mp4.display()
    );

    if let Some(parent) = output_mp4.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let status = tokio::process::Command::new(ffmpeg_path)
        .args(["-y"])
        .args(["-i"]).arg(original_video)
        .args(["-i"]).arg(dubbed_wav)
        .args(["-map", "0:v:0", "-map", "1:a:0"])
        .args(["-c:v", "copy"])
        .args(["-c:a", "aac", "-b:a", "192k"])
        .args(["-shortest"])
        .arg(output_mp4)
        .status()
        .await
        .context("Failed to run sandboxed ffmpeg for remux")?;

    if !status.success() {
        anyhow::bail!("ffmpeg remux failed");
    }

    info!("Remuxed MP4: {}", output_mp4.display());
    Ok(())
}

/// Remux with dual audio: keep original + add dubbed track.
pub async fn remux_video_dual_audio(
    ffmpeg_path: &Path,
    original_video: &Path,
    dubbed_wav: &Path,
    output_mp4: &Path,
) -> Result<()> {
    info!("Remuxing dual-audio: {}", output_mp4.display());

    if let Some(parent) = output_mp4.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let status = tokio::process::Command::new(ffmpeg_path)
        .args(["-y"])
        .args(["-i"]).arg(original_video)
        .args(["-i"]).arg(dubbed_wav)
        .args(["-map", "0:v:0", "-map", "0:a:0", "-map", "1:a:0"])
        .args(["-c:v", "copy"])
        .args(["-c:a", "aac", "-b:a", "192k"])
        .args(["-metadata:s:a:0", "language=eng", "-metadata:s:a:0", "title=Original"])
        .args(["-metadata:s:a:1", "language=khm", "-metadata:s:a:1", "title=Khmer Dub"])
        .args(["-disposition:a:0", "none", "-disposition:a:1", "default"])
        .args(["-shortest"])
        .arg(output_mp4)
        .status()
        .await
        .context("Failed to run sandboxed ffmpeg for dual-audio remux")?;

    if !status.success() {
        anyhow::bail!("ffmpeg dual-audio remux failed");
    }

    info!("Dual-audio MP4: {}", output_mp4.display());
    Ok(())
}

/// Burn subtitles into a video (requires re-encode).
pub async fn burn_subtitles(
    ffmpeg_path: &Path,
    input_mp4: &Path,
    srt_path: &Path,
    output_mp4: &Path,
) -> Result<()> {
    info!("Burning subtitles: {}", srt_path.display());

    if let Some(parent) = output_mp4.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let filter = format!(
        "subtitles={}:force_style='FontName=Khmer OS,FontSize=24,PrimaryColour=&HFFFFFF&'",
        srt_path.display()
    );

    let status = tokio::process::Command::new(ffmpeg_path)
        .args(["-y"])
        .args(["-i"]).arg(input_mp4)
        .args(["-vf", &filter])
        .args(["-c:a", "copy"])
        .arg(output_mp4)
        .status()
        .await
        .context("Failed to run sandboxed ffmpeg for subtitle burn")?;

    if !status.success() {
        anyhow::bail!("Subtitle burn failed");
    }

    Ok(())
}

/// Generate SRT subtitle file from timed translations.
pub fn generate_srt(
    segments: &[(u64, u64, String)],
    output_path: &Path,
) -> Result<()> {
    use std::io::Write;

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = std::fs::File::create(output_path)?;

    for (i, (start_ms, end_ms, text)) in segments.iter().enumerate() {
        let start = format_srt_time(*start_ms);
        let end = format_srt_time(*end_ms);
        writeln!(file, "{}", i + 1)?;
        writeln!(file, "{} --> {}", start, end)?;
        writeln!(file, "{}", text)?;
        writeln!(file)?;
    }

    info!("Generated SRT: {} entries → {}", segments.len(), output_path.display());
    Ok(())
}

fn format_srt_time(ms: u64) -> String {
    let hours = ms / 3_600_000;
    let minutes = (ms % 3_600_000) / 60_000;
    let seconds = (ms % 60_000) / 1_000;
    let millis = ms % 1_000;
    format!("{:02}:{:02}:{:02},{:03}", hours, minutes, seconds, millis)
}

/// TTS synthesis via NDE-OS VoiceRuntime (Edge TTS).
///
/// Generates a WAV file from Khmer text using the sandboxed edge-tts.
pub async fn synthesize_tts(
    ffmpeg_path: &Path,
    edge_tts_path: &Path,
    text: &str,
    voice: &str,
    speed: f32,
    output_wav: &Path,
) -> Result<PathBuf> {
    if let Some(parent) = output_wav.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mp3_path = output_wav.with_extension("mp3");

    let rate_str = if speed >= 1.0 {
        format!("+{}%", ((speed - 1.0) * 100.0) as i32)
    } else {
        format!("-{}%", ((1.0 - speed) * 100.0) as i32)
    };

    // Step 1: Generate MP3 with edge-tts.
    let output = tokio::process::Command::new(edge_tts_path)
        .arg("--voice").arg(voice)
        .arg("--rate").arg(&rate_str)
        .arg("--text").arg(text)
        .arg("--write-media").arg(&mp3_path)
        .output()
        .await
        .context("Failed to run edge-tts")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("edge-tts failed: {stderr}");
    }

    // Step 2: Convert MP3 → WAV (mono, 44100Hz) with sandboxed ffmpeg.
    let ffmpeg_out = tokio::process::Command::new(ffmpeg_path)
        .args(["-y", "-i"])
        .arg(&mp3_path)
        .args(["-ar", "44100", "-ac", "1", "-f", "wav"])
        .arg(output_wav)
        .output()
        .await
        .context("Failed to run sandboxed ffmpeg for MP3→WAV")?;

    if !ffmpeg_out.status.success() {
        let stderr = String::from_utf8_lossy(&ffmpeg_out.stderr);
        anyhow::bail!("ffmpeg MP3→WAV conversion failed: {stderr}");
    }

    // Clean up MP3.
    let _ = std::fs::remove_file(&mp3_path);

    Ok(output_wav.to_path_buf())
}
