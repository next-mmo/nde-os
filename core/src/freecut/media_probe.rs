//! FFprobe-based media metadata extraction.
//!
//! Uses `ffprobe` subprocess to detect codec, resolution, duration, FPS,
//! and audio properties. Cross-platform: works on Windows, macOS, Linux.

use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde::Deserialize;
use std::path::Path;
use uuid::Uuid;

use super::project::{MediaMetadata, MediaType};

/// Probe a media file and return structured metadata.
///
/// Tries `ffprobe` first for rich codec/duration/resolution data.
/// Falls back to extension-based detection when `ffprobe` is not installed,
/// so media import still works (with limited metadata) on systems without FFmpeg.
pub fn probe_media(file_path: &Path, ffprobe_bin: Option<&str>) -> Result<MediaMetadata> {
    match probe_media_ffprobe(file_path, ffprobe_bin) {
        Ok(meta) => Ok(meta),
        Err(_) => probe_media_fallback(file_path),
    }
}

/// Full probe via `ffprobe` subprocess.
fn probe_media_ffprobe(file_path: &Path, ffprobe_bin: Option<&str>) -> Result<MediaMetadata> {
    let bin = ffprobe_bin.unwrap_or("ffprobe");

    let output = std::process::Command::new(bin)
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(file_path.as_os_str())
        .output()
        .with_context(|| format!("failed to run {bin} — is FFmpeg installed?"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("ffprobe failed: {stderr}");
    }

    let probe: FfprobeOutput =
        serde_json::from_slice(&output.stdout).context("failed to parse ffprobe JSON")?;

    let file_name = file_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let file_size = std::fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);

    // Find video and audio streams.
    let video_stream = probe
        .streams
        .iter()
        .find(|s| s.codec_type.as_deref() == Some("video"));
    let audio_stream = probe
        .streams
        .iter()
        .find(|s| s.codec_type.as_deref() == Some("audio"));

    // Determine media type.
    let media_type = if video_stream.is_some() {
        // Check if it's actually an image (single frame, common image codecs).
        let is_image = video_stream
            .map(|v| {
                let codec = v.codec_name.as_deref().unwrap_or("");
                let image_codecs = ["png", "mjpeg", "bmp", "gif", "webp", "tiff"];
                image_codecs.iter().any(|c| codec.contains(c))
                    || v.nb_frames.as_deref() == Some("1")
            })
            .unwrap_or(false);

        if is_image {
            MediaType::Image
        } else {
            MediaType::Video
        }
    } else if audio_stream.is_some() {
        MediaType::Audio
    } else {
        media_type_from_extension(file_path)
    };

    // Extract video properties.
    let (width, height, fps, codec) = match video_stream {
        Some(v) => {
            let w = v.width;
            let h = v.height;
            let fps_val = parse_fps(v.r_frame_rate.as_deref());
            let codec_name = v.codec_name.clone();
            (w, h, fps_val, codec_name)
        }
        None => (None, None, None, None),
    };

    // Extract audio properties.
    let (audio_codec, sample_rate, channels) = match audio_stream {
        Some(a) => (
            a.codec_name.clone(),
            a.sample_rate.as_deref().and_then(|s| s.parse::<u32>().ok()),
            a.channels,
        ),
        None => (None, None, None),
    };

    // Duration from format or stream.
    let duration_secs = probe
        .format
        .as_ref()
        .and_then(|f| f.duration.as_deref())
        .and_then(|d| d.parse::<f64>().ok())
        .or_else(|| {
            video_stream
                .and_then(|v| v.duration.as_deref())
                .and_then(|d| d.parse::<f64>().ok())
        });

    Ok(MediaMetadata {
        id: Uuid::new_v4().to_string(),
        file_name,
        file_path: file_path.to_string_lossy().to_string(),
        file_size,
        media_type,
        width,
        height,
        duration_secs,
        fps,
        codec,
        audio_codec,
        sample_rate,
        channels,
        thumbnail_path: None,
        imported_at: Utc::now(),
    })
}

/// Lightweight fallback probe when `ffprobe` is not available.
///
/// Determines media type from file extension and reads basic file metadata.
/// For images, attempts to read dimensions via the file header bytes.
/// Does not populate codec, fps, duration, or audio properties.
fn probe_media_fallback(file_path: &Path) -> Result<MediaMetadata> {
    let file_name = file_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let file_size = std::fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);
    let media_type = media_type_from_extension(file_path);

    // Try to read image dimensions from the file header.
    let (width, height) = if media_type == MediaType::Image {
        read_image_dimensions(file_path).unwrap_or((None, None))
    } else {
        (None, None)
    };

    Ok(MediaMetadata {
        id: Uuid::new_v4().to_string(),
        file_name,
        file_path: file_path.to_string_lossy().to_string(),
        file_size,
        media_type,
        width,
        height,
        duration_secs: None,
        fps: None,
        codec: None,
        audio_codec: None,
        sample_rate: None,
        channels: None,
        thumbnail_path: None,
        imported_at: Utc::now(),
    })
}

/// Determine [`MediaType`] from file extension alone.
fn media_type_from_extension(file_path: &Path) -> MediaType {
    let ext = file_path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "tiff" | "svg" => MediaType::Image,
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" => MediaType::Audio,
        _ => MediaType::Video,
    }
}

/// Read image dimensions from the file header (PNG and JPEG only).
///
/// Avoids pulling in heavyweight image crates — just reads the minimal
/// header bytes needed for the two most common web formats.
fn read_image_dimensions(file_path: &Path) -> Result<(Option<u32>, Option<u32>)> {
    use std::io::Read;
    let mut f = std::fs::File::open(file_path)?;
    let mut header = [0u8; 32];
    let n = f.read(&mut header)?;
    if n < 8 {
        return Ok((None, None));
    }

    // PNG: bytes 16..20 = width (BE u32), 20..24 = height (BE u32)
    if header.starts_with(&[0x89, b'P', b'N', b'G']) && n >= 24 {
        let w = u32::from_be_bytes([header[16], header[17], header[18], header[19]]);
        let h = u32::from_be_bytes([header[20], header[21], header[22], header[23]]);
        return Ok((Some(w), Some(h)));
    }

    // JPEG: need to scan for SOF0/SOF2 marker — skip for now, would require
    // reading deeper into the file. Return None dimensions.
    Ok((None, None))
}

/// Generate thumbnail images at evenly-spaced intervals using `ffmpeg`.
///
/// Returns list of output file paths.
pub fn generate_thumbnails(
    file_path: &Path,
    output_dir: &Path,
    count: usize,
    thumb_width: u32,
    ffmpeg_bin: Option<&str>,
) -> Result<Vec<String>> {
    let bin = ffmpeg_bin.unwrap_or("ffmpeg");
    let media_id = file_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "thumb".to_string());

    std::fs::create_dir_all(output_dir)?;

    // First probe to get duration.
    let meta = probe_media(file_path, None)?;
    let duration = meta.duration_secs.unwrap_or(1.0);

    let mut paths = Vec::with_capacity(count);

    for i in 0..count {
        let time = if count > 1 {
            (duration * i as f64) / (count as f64 - 1.0)
        } else {
            0.0
        };

        let out_name = format!("{media_id}_thumb_{i:04}.jpg");
        let out_path = output_dir.join(&out_name);

        let status = std::process::Command::new(bin)
            .args(["-y", "-ss", &format!("{time:.3}"), "-i"])
            .arg(file_path.as_os_str())
            .args([
                "-vframes",
                "1",
                "-vf",
                &format!("scale={thumb_width}:-1"),
                "-q:v",
                "4",
            ])
            .arg(out_path.as_os_str())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .with_context(|| format!("failed to run {bin}"))?;

        if status.success() {
            paths.push(out_path.to_string_lossy().to_string());
        }
    }

    Ok(paths)
}

/// Generate audio waveform peak data.
///
/// Returns normalized peak values (0.0 – 1.0) at the requested sample count.
pub fn generate_waveform(
    file_path: &Path,
    sample_count: usize,
    ffmpeg_bin: Option<&str>,
) -> Result<Vec<f32>> {
    let bin = ffmpeg_bin.unwrap_or("ffmpeg");

    // Extract raw PCM (mono, 8kHz, s16le) to keep it fast.
    let output = std::process::Command::new(bin)
        .args(["-i"])
        .arg(file_path.as_os_str())
        .args([
            "-ac",
            "1",
            "-ar",
            "8000",
            "-f",
            "s16le",
            "-acodec",
            "pcm_s16le",
            "pipe:1",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .with_context(|| format!("failed to run {bin} for waveform extraction"))?;

    if output.stdout.is_empty() {
        return Ok(vec![0.0; sample_count]);
    }

    // Convert bytes to i16 samples.
    let samples: Vec<i16> = output
        .stdout
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    if samples.is_empty() {
        return Ok(vec![0.0; sample_count]);
    }

    // Downsample to requested count by taking peak of each bucket.
    let bucket_size = (samples.len() as f64 / sample_count as f64).max(1.0) as usize;
    let mut peaks = Vec::with_capacity(sample_count);

    for i in 0..sample_count {
        let start = i * bucket_size;
        let end = ((i + 1) * bucket_size).min(samples.len());
        if start >= samples.len() {
            peaks.push(0.0);
            continue;
        }
        let peak = samples[start..end]
            .iter()
            .map(|s| s.unsigned_abs() as f32)
            .fold(0.0f32, f32::max);
        peaks.push(peak / 32768.0);
    }

    Ok(peaks)
}

// ─── FFprobe JSON schema (minimal) ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct FfprobeOutput {
    #[serde(default)]
    streams: Vec<FfprobeStream>,
    #[serde(default)]
    format: Option<FfprobeFormat>,
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    r_frame_rate: Option<String>,
    duration: Option<String>,
    sample_rate: Option<String>,
    channels: Option<u32>,
    nb_frames: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
}

/// Parse FFprobe frame rate string like "30000/1001" or "30/1".
fn parse_fps(rate: Option<&str>) -> Option<f64> {
    let rate = rate?;
    if let Some((num, den)) = rate.split_once('/') {
        let n: f64 = num.parse().ok()?;
        let d: f64 = den.parse().ok()?;
        if d > 0.0 {
            Some(n / d)
        } else {
            None
        }
    } else {
        rate.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_fps_fraction() {
        assert!((parse_fps(Some("30000/1001")).unwrap() - 29.97).abs() < 0.1);
        assert!((parse_fps(Some("30/1")).unwrap() - 30.0).abs() < 0.01);
        assert!(parse_fps(Some("24")).unwrap() - 24.0 < 0.01);
        assert!(parse_fps(None).is_none());
    }
}
