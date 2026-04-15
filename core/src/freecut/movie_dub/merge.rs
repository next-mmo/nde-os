//! Video merging for the FreeCut dubbing workstation.
//!
//! Concatenates all dubbed parts back into a single final video
//! using FFmpeg's concat demuxer (stream copy, no re-encoding).

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::info;

/// Merge multiple video files into one using FFmpeg concat demuxer.
///
/// This uses stream copy (no re-encoding) for maximum speed.
pub async fn merge_parts(
    ffmpeg_path: &Path,
    parts: &[PathBuf],
    output: &Path,
) -> Result<PathBuf> {
    if parts.is_empty() {
        anyhow::bail!("No parts to merge");
    }

    if parts.len() == 1 {
        // Single part — just copy it.
        std::fs::copy(&parts[0], output)?;
        return Ok(output.to_path_buf());
    }

    // Create concat list file.
    let concat_dir = output.parent().unwrap_or(Path::new(""));
    let concat_file = concat_dir.join("_concat_list.txt");
    let mut content = String::new();
    for part in parts {
        // FFmpeg concat requires forward slashes and escaped single quotes.
        let path_str = part.to_string_lossy().replace('\'', "'\\''");
        content.push_str(&format!("file '{}'\n", path_str));
    }
    std::fs::write(&concat_file, &content)?;

    info!("Merging {} parts into {}", parts.len(), output.display());

    let status = tokio::process::Command::new(ffmpeg_path)
        .args([
            "-y",
            "-f", "concat",
            "-safe", "0",
            "-i",
        ])
        .arg(&concat_file)
        .args(["-c", "copy"])
        .arg(output)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .context("Failed to run ffmpeg concat")?;

    // Clean up concat list.
    let _ = std::fs::remove_file(&concat_file);

    if !status.success() {
        anyhow::bail!("FFmpeg concat failed with exit code: {:?}", status.code());
    }

    info!("Merge complete: {}", output.display());
    Ok(output.to_path_buf())
}

/// Merge all SRT files from split parts into a single SRT with adjusted timestamps.
pub fn merge_srts(
    srt_paths: &[(PathBuf, f64)], // (srt_path, offset_secs)
    output: &Path,
) -> Result<()> {
    let mut all_entries: Vec<(u64, u64, String)> = Vec::new();

    for (srt_path, offset_secs) in srt_paths {
        if !srt_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(srt_path)?;
        let offset_ms = (*offset_secs * 1000.0) as u64;

        let mut lines = content.lines().peekable();
        while lines.peek().is_some() {
            // Skip blanks.
            while lines.peek().map_or(false, |l| l.trim().is_empty()) {
                lines.next();
            }
            // Sequence number.
            match lines.next() {
                Some(l) if l.trim().parse::<u32>().is_ok() => {}
                _ => continue,
            }
            // Timestamp line.
            let ts_line = match lines.next() {
                Some(l) => l.trim().to_string(),
                None => break,
            };
            let (start_ms, end_ms) = parse_srt_ts(&ts_line)?;
            // Text lines.
            let mut text_parts = Vec::new();
            while lines.peek().map_or(false, |l| !l.trim().is_empty()) {
                text_parts.push(lines.next().unwrap().trim().to_string());
            }
            let text = text_parts.join(" ");
            if !text.is_empty() {
                all_entries.push((start_ms + offset_ms, end_ms + offset_ms, text));
            }
        }
    }

    // Write merged SRT.
    super::mix::generate_srt(&all_entries, output)?;
    info!("Merged {} SRT entries into {}", all_entries.len(), output.display());
    Ok(())
}

fn parse_srt_ts(line: &str) -> Result<(u64, u64)> {
    let parts: Vec<&str> = line.split("-->").collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid SRT timestamp: {}", line);
    }
    let start = parse_time(parts[0].trim())?;
    let end = parse_time(parts[1].trim())?;
    Ok((start, end))
}

fn parse_time(s: &str) -> Result<u64> {
    let s = s.replace(',', ".");
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        anyhow::bail!("Invalid time: {}", s);
    }
    let h: u64 = parts[0].parse()?;
    let m: u64 = parts[1].parse()?;
    let sec_parts: Vec<&str> = parts[2].split('.').collect();
    let sec: u64 = sec_parts[0].parse()?;
    let ms: u64 = if sec_parts.len() > 1 { sec_parts[1].parse()? } else { 0 };
    Ok(h * 3_600_000 + m * 60_000 + sec * 1000 + ms)
}
