//! Local dubbing support for FreeCut.
//!
//! This keeps the pipeline local-first:
//! - local SRT import
//! - local Whisper transcription
//! - optional NDE LLM line polish / translation
//! - Edge TTS synthesis
//! - optional RVC CLI post-processing

use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::project::{
    DubbingIngestMode, DubbingLlmConfig, DubbingSegment, DubbingSession, DubbingSpeaker,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DubbingToolReport {
    pub whisper_available: bool,
    pub edge_tts_available: bool,
    pub nde_llm_available: bool,
    pub python_available: bool,
    pub rvc_available: bool,
    #[serde(default)]
    pub edge_voices: Vec<String>,
    #[serde(default)]
    pub nde_active_model: Option<String>,
    #[serde(default)]
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DubbingImportResult {
    pub imported_srt_path: String,
    pub segments: Vec<DubbingSegment>,
    pub speakers: Vec<DubbingSpeaker>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WhisperSettings {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub task: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DubbingProgressUpdate {
    pub phase: String,
    pub current: usize,
    pub total: usize,
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct NdeEnvelope<T> {
    success: bool,
    message: String,
    data: T,
}

#[derive(Debug, Deserialize)]
struct NdeAgentChatData {
    response: String,
    conversation_id: String,
}

pub fn detect_local_tools() -> Result<DubbingToolReport> {
    let whisper_available = resolve_command("whisper").is_some();
    let edge_tts_available = resolve_command("edge-tts").is_some();
    let nde_llm_status = detect_nde_llm().ok();
    let python_available = resolve_python().is_some();

    let mut details = Vec::new();
    if !whisper_available {
        details.push("Whisper CLI not found on PATH".to_string());
    }
    if !edge_tts_available {
        details.push("edge-tts CLI not found on PATH".to_string());
    }
    if nde_llm_status.is_none() {
        details.push("NDE LLM gateway not reachable on localhost:8080".to_string());
    }
    if !python_available {
        details.push("Python not found on PATH (needed for RVC CLI)".to_string());
    }

    Ok(DubbingToolReport {
        whisper_available,
        edge_tts_available,
        nde_llm_available: nde_llm_status.is_some(),
        python_available,
        rvc_available: python_available,
        edge_voices: if edge_tts_available {
            list_edge_voices().unwrap_or_default()
        } else {
            Vec::new()
        },
        nde_active_model: nde_llm_status,
        details,
    })
}

pub fn import_srt_as_session(file_path: &Path) -> Result<DubbingImportResult> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("failed to read SRT file {}", file_path.display()))?;
    let mut segments = parse_srt(&content)?;
    if segments.is_empty() {
        bail!("no subtitle segments found in {}", file_path.display());
    }
    let speakers = build_speakers_from_segments(&mut segments);

    Ok(DubbingImportResult {
        imported_srt_path: file_path.to_string_lossy().to_string(),
        segments,
        speakers,
    })
}

pub fn generate_dubbing_assets<F>(
    project_id: &str,
    render_root: &Path,
    mut session: DubbingSession,
    whisper: WhisperSettings,
    progress: F,
) -> Result<DubbingSession>
where
    F: Fn(DubbingProgressUpdate),
{
    let output_dir = render_root.join(project_id).join("dubbing");
    fs::create_dir_all(&output_dir).with_context(|| {
        format!(
            "failed to create dubbing directory {}",
            output_dir.display()
        )
    })?;

    progress(DubbingProgressUpdate {
        phase: "prepare".to_string(),
        current: 0,
        total: 1,
        message: "Preparing dubbing session".to_string(),
    });

    if session.segments.is_empty() {
        match session.ingest_mode {
            DubbingIngestMode::Srt => {
                let srt_path = session
                    .imported_srt_path
                    .clone()
                    .ok_or_else(|| anyhow!("SRT ingest selected but no SRT file was provided"))?;
                let imported = import_srt_as_session(Path::new(&srt_path))?;
                session.imported_srt_path = Some(imported.imported_srt_path);
                session.segments = imported.segments;
                session.speakers = merge_speakers(&session.speakers, &imported.speakers);
            }
            DubbingIngestMode::Whisper => {
                let source_media_path = session.source_media_path.clone().ok_or_else(|| {
                    anyhow!("Whisper ingest selected but no source media path was provided")
                })?;
                progress(DubbingProgressUpdate {
                    phase: "transcribe".to_string(),
                    current: 0,
                    total: 1,
                    message: "Running local Whisper transcription".to_string(),
                });
                let srt_path =
                    transcribe_with_whisper(Path::new(&source_media_path), &output_dir, whisper)?;
                let imported = import_srt_as_session(&srt_path)?;
                session.imported_srt_path = Some(imported.imported_srt_path);
                session.segments = imported.segments;
                session.speakers = merge_speakers(&session.speakers, &imported.speakers);
            }
        }
    }

    ensure_default_speakers(&mut session);

    if session.segments.is_empty() {
        bail!("dubbing session does not contain any segments");
    }

    progress(DubbingProgressUpdate {
        phase: "prepare".to_string(),
        current: 1,
        total: 1,
        message: format!("Prepared {} subtitle segments", session.segments.len()),
    });

    if let Some(llm) = session.llm.clone() {
        if llm.enabled {
            progress(DubbingProgressUpdate {
                phase: "llm".to_string(),
                current: 0,
                total: session.segments.len(),
                message: "Polishing lines with the active NDE LLM".to_string(),
            });
            apply_nde_llm_to_segments(
                &mut session.segments,
                &llm,
                &session.target_language,
                &progress,
            )?;
        }
    }

    let speaker_map: HashMap<String, DubbingSpeaker> = session
        .speakers
        .iter()
        .cloned()
        .map(|speaker| (speaker.id.clone(), speaker))
        .collect();

    let total = session.segments.len();
    for (idx, segment) in session.segments.iter_mut().enumerate() {
        let speaker_id = segment
            .speaker_id
            .clone()
            .unwrap_or_else(|| session.speakers[0].id.clone());
        let speaker = speaker_map.get(&speaker_id).ok_or_else(|| {
            anyhow!(
                "speaker mapping missing for segment {} and speaker {}",
                segment.id,
                speaker_id
            )
        })?;
        let final_text = segment
            .output_text
            .clone()
            .unwrap_or_else(|| segment.text.clone());
        let safe_name = slugify(&format!("{}-{}", idx + 1, speaker.label));
        let out_path = output_dir.join(format!("{safe_name}.mp3"));

        progress(DubbingProgressUpdate {
            phase: "synthesis".to_string(),
            current: idx,
            total,
            message: format!("Generating dub audio for {}", segment.id),
        });

        if speaker
            .rvc
            .as_ref()
            .map(|config| config.enabled)
            .unwrap_or(false)
        {
            synthesize_with_rvc(&final_text, speaker, &out_path)?;
        } else {
            synthesize_with_edge_tts(&final_text, speaker, &out_path, &session.target_language)?;
        }

        segment.audio_path = Some(out_path.to_string_lossy().to_string());
        segment.output_text = Some(final_text);
        segment.status = Some("ready".to_string());
    }

    session.output_dir = Some(output_dir.to_string_lossy().to_string());
    session.updated_at = Some(Utc::now());
    session.last_generated_at = Some(Utc::now());

    progress(DubbingProgressUpdate {
        phase: "synthesis".to_string(),
        current: total,
        total,
        message: "Dubbing assets ready".to_string(),
    });

    Ok(session)
}

fn apply_nde_llm_to_segments<F>(
    segments: &mut [DubbingSegment],
    llm: &DubbingLlmConfig,
    target_language: &str,
    progress: &F,
) -> Result<()>
where
    F: Fn(DubbingProgressUpdate),
{
    let mode = llm.mode.clone().unwrap_or_else(|| "translate".to_string());
    let client = Client::new();

    let total = segments.len();
    for (idx, segment) in segments.iter_mut().enumerate() {
        let prompt = if mode.eq_ignore_ascii_case("polish") {
            format!(
                "Polish this subtitle for natural dubbing in {}. Keep it short and return only the final subtitle text.\n\n{}",
                target_language, segment.text
            )
        } else {
            format!(
                "Translate this subtitle into {} for spoken dubbing. Keep it concise and natural. Return only the translated subtitle text.\n\n{}",
                target_language, segment.text
            )
        };

        let response = client
            .post("http://127.0.0.1:8080/api/agent/chat")
            .json(&serde_json::json!({
                "message": prompt,
                "conversation_id": serde_json::Value::Null,
            }))
            .send()
            .context("failed to contact the local NDE LLM service")?
            .error_for_status()
            .context("local NDE LLM returned an error")?;
        let body: NdeEnvelope<NdeAgentChatData> =
            response.json().context("invalid NDE LLM response body")?;
        if !body.success {
            return Err(anyhow!("NDE LLM request failed: {}", body.message));
        }
        let content = Some(body.data.response.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow!("empty response received from the NDE LLM"))?;
        segment.output_text = Some(content);
        progress(DubbingProgressUpdate {
            phase: "llm".to_string(),
            current: idx + 1,
            total,
            message: format!("Polished line {}", idx + 1),
        });
    }

    Ok(())
}

fn synthesize_with_edge_tts(
    text: &str,
    speaker: &DubbingSpeaker,
    out_path: &Path,
    target_language: &str,
) -> Result<()> {
    let edge_tts = resolve_command("edge-tts")
        .ok_or_else(|| anyhow!("edge-tts CLI not found; install it or disable synthesis"))?;
    let voice = if speaker.voice.trim().is_empty() {
        default_voice_for_language(target_language)
    } else {
        speaker.voice.clone()
    };

    let mut command = Command::new(edge_tts);
    command.arg("--voice").arg(voice);
    command.arg("--text").arg(text);
    command.arg("--write-media").arg(out_path);

    if let Some(rate) = speaker
        .rate
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        command.arg(format!("--rate={rate}"));
    }
    if let Some(pitch) = speaker
        .pitch
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        command.arg(format!("--pitch={pitch}"));
    }
    if let Some(volume) = speaker
        .volume
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        command.arg(format!("--volume={volume}"));
    }

    run_checked_command(command, "edge-tts synthesis")
}

fn synthesize_with_rvc(text: &str, speaker: &DubbingSpeaker, out_path: &Path) -> Result<()> {
    let rvc = speaker
        .rvc
        .as_ref()
        .ok_or_else(|| anyhow!("RVC requested but configuration is missing"))?;
    let python = rvc
        .python_path
        .clone()
        .map(PathBuf::from)
        .or_else(resolve_python)
        .ok_or_else(|| anyhow!("Python not found for RVC CLI invocation"))?;
    let cli_path = rvc
        .cli_path
        .clone()
        .ok_or_else(|| anyhow!("RVC CLI path is required when RVC is enabled"))?;
    let model_path = rvc
        .model_path
        .clone()
        .ok_or_else(|| anyhow!("RVC model path is required when RVC is enabled"))?;
    let index_path = rvc
        .index_path
        .clone()
        .ok_or_else(|| anyhow!("RVC index path is required when RVC is enabled"))?;

    let temp_tts_path = out_path.with_extension("edge.mp3");
    let mut command = Command::new(python);
    command.arg(cli_path);
    command.arg("tts_infer");
    command.arg("--tts_text").arg(text);
    command
        .arg("--tts_voice")
        .arg(if speaker.voice.trim().is_empty() {
            "en-US-AriaNeural"
        } else {
            speaker.voice.as_str()
        });
    command.arg("--output_tts_path").arg(&temp_tts_path);
    command.arg("--output_rvc_path").arg(out_path);
    command.arg("--pth_path").arg(model_path);
    command.arg("--index_path").arg(index_path);
    command.arg("--export_format").arg("MP3");
    if let Some(pitch_shift) = rvc.pitch_shift {
        command.arg("--pitch").arg(pitch_shift.to_string());
    }

    run_checked_command(command, "RVC synthesis")
}

fn transcribe_with_whisper(
    source_media_path: &Path,
    output_dir: &Path,
    whisper: WhisperSettings,
) -> Result<PathBuf> {
    let whisper_cli = resolve_command("whisper")
        .ok_or_else(|| anyhow!("Whisper CLI not found; install it or switch to SRT import"))?;
    let model = whisper.model.unwrap_or_else(|| "base".to_string());
    let task = whisper.task.unwrap_or_else(|| "transcribe".to_string());

    let mut command = Command::new(whisper_cli);
    command.arg(source_media_path);
    command.arg("--model").arg(model);
    command.arg("--task").arg(task);
    command.arg("--output_dir").arg(output_dir);
    command.arg("--output_format").arg("srt");
    if let Some(language) = whisper
        .language
        .as_deref()
        .filter(|value| !value.trim().is_empty() && *value != "auto")
    {
        command.arg("--language").arg(language);
    }

    run_checked_command(command, "Whisper transcription")?;

    if let Some(stem) = source_media_path
        .file_stem()
        .and_then(|value| value.to_str())
    {
        let expected = output_dir.join(format!("{stem}.srt"));
        if expected.exists() {
            return Ok(expected);
        }
    }

    let mut fallback = None;
    for entry in fs::read_dir(output_dir)
        .with_context(|| format!("failed to inspect {}", output_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.eq_ignore_ascii_case("srt"))
            .unwrap_or(false)
        {
            fallback = Some(path);
            break;
        }
    }

    fallback.ok_or_else(|| anyhow!("Whisper completed but no SRT output was found"))
}

fn parse_srt(content: &str) -> Result<Vec<DubbingSegment>> {
    let normalized = content.replace("\r\n", "\n");
    let mut segments = Vec::new();

    for block in normalized.split("\n\n") {
        let lines: Vec<&str> = block
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        if lines.is_empty() {
            continue;
        }

        let timing_index = if lines[0].contains("-->") { 0 } else { 1 };
        if lines.len() <= timing_index {
            continue;
        }

        let timing_line = lines[timing_index];
        let (start_raw, end_raw) = timing_line
            .split_once("-->")
            .ok_or_else(|| anyhow!("invalid SRT timing line: {timing_line}"))?;
        let start_secs = parse_timestamp(start_raw.trim())?;
        let end_secs = parse_timestamp(end_raw.trim())?;
        let text = lines[timing_index + 1..].join(" ").trim().to_string();
        if text.is_empty() {
            continue;
        }

        let (speaker_label, final_text) = extract_speaker_label(&text);
        segments.push(DubbingSegment {
            id: format!("seg-{:04}", segments.len() + 1),
            start_secs,
            end_secs,
            text: final_text,
            output_text: None,
            speaker_id: speaker_label.map(|label| slugify(&label)),
            audio_path: None,
            status: Some("pending".to_string()),
        });
    }

    Ok(segments)
}

fn parse_timestamp(raw: &str) -> Result<f64> {
    let cleaned = raw.replace(',', ".");
    let parts: Vec<&str> = cleaned.split(':').collect();
    if parts.len() != 3 {
        bail!("invalid subtitle timestamp: {raw}");
    }
    let hours: f64 = parts[0].parse().context("invalid hours value")?;
    let minutes: f64 = parts[1].parse().context("invalid minutes value")?;
    let seconds: f64 = parts[2].parse().context("invalid seconds value")?;
    Ok((hours * 3600.0) + (minutes * 60.0) + seconds)
}

fn build_speakers_from_segments(segments: &mut [DubbingSegment]) -> Vec<DubbingSpeaker> {
    let mut labels = BTreeSet::new();
    for segment in segments.iter_mut() {
        if let Some(existing) = segment.speaker_id.clone() {
            labels.insert(existing);
        }
    }

    if labels.is_empty() {
        let default_id = "speaker-narrator".to_string();
        for segment in segments.iter_mut() {
            segment.speaker_id = Some(default_id.clone());
        }
        return vec![DubbingSpeaker {
            id: default_id,
            label: "Narrator".to_string(),
            voice: "en-US-AriaNeural".to_string(),
            rate: Some("+0%".to_string()),
            pitch: None,
            volume: None,
            rvc: None,
        }];
    }

    labels
        .into_iter()
        .enumerate()
        .map(|(idx, speaker_id)| DubbingSpeaker {
            id: speaker_id.clone(),
            label: prettify_speaker_label(&speaker_id, idx + 1),
            voice: "en-US-AriaNeural".to_string(),
            rate: Some("+0%".to_string()),
            pitch: None,
            volume: None,
            rvc: None,
        })
        .collect()
}

fn merge_speakers(existing: &[DubbingSpeaker], imported: &[DubbingSpeaker]) -> Vec<DubbingSpeaker> {
    let mut by_id: HashMap<String, DubbingSpeaker> = existing
        .iter()
        .cloned()
        .map(|speaker| (speaker.id.clone(), speaker))
        .collect();
    for speaker in imported {
        by_id
            .entry(speaker.id.clone())
            .or_insert_with(|| speaker.clone());
    }
    let mut speakers: Vec<DubbingSpeaker> = by_id.into_values().collect();
    speakers.sort_by(|left, right| left.label.cmp(&right.label));
    speakers
}

fn ensure_default_speakers(session: &mut DubbingSession) {
    if session.speakers.is_empty() {
        session.speakers = build_speakers_from_segments(&mut session.segments);
    }

    if session.speakers.is_empty() {
        session.speakers.push(DubbingSpeaker {
            id: "speaker-narrator".to_string(),
            label: "Narrator".to_string(),
            voice: default_voice_for_language(&session.target_language),
            rate: Some("+0%".to_string()),
            pitch: None,
            volume: None,
            rvc: None,
        });
    }

    let default_id = session.speakers[0].id.clone();
    for segment in &mut session.segments {
        if segment.speaker_id.is_none() {
            segment.speaker_id = Some(default_id.clone());
        }
    }
}

fn extract_speaker_label(text: &str) -> (Option<String>, String) {
    let trimmed = text.trim();
    if let Some(stripped) = trimmed.strip_prefix('[') {
        if let Some(close_index) = stripped.find(']') {
            let label = stripped[..close_index].trim();
            let rest = stripped[close_index + 1..]
                .trim_start()
                .trim_start_matches(':')
                .trim();
            if !label.is_empty() && !rest.is_empty() {
                return (Some(label.to_string()), rest.to_string());
            }
        }
    }

    if let Some((label, rest)) = trimmed.split_once(':') {
        let looks_like_speaker = label.len() <= 32
            && label
                .chars()
                .all(|ch| ch.is_alphanumeric() || ch == ' ' || ch == '-' || ch == '_');
        if looks_like_speaker && !rest.trim().is_empty() {
            return (Some(label.trim().to_string()), rest.trim().to_string());
        }
    }

    (None, trimmed.to_string())
}

fn list_edge_voices() -> Result<Vec<String>> {
    let edge_tts =
        resolve_command("edge-tts").ok_or_else(|| anyhow!("edge-tts CLI not found on PATH"))?;
    let output = Command::new(edge_tts)
        .arg("--list-voices")
        .output()
        .context("failed to list edge-tts voices")?;
    if !output.status.success() {
        return Ok(Vec::new());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut voices = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('-') {
            continue;
        }
        if let Some(name) = trimmed.split_whitespace().next() {
            if name.contains('-') && name.ends_with("Neural") {
                voices.push(name.to_string());
            }
        }
        if voices.len() >= 60 {
            break;
        }
    }
    voices.sort();
    voices.dedup();
    Ok(voices)
}

fn detect_nde_llm() -> Result<String> {
    let client = Client::new();
    let response = client
        .get("http://127.0.0.1:8080/api/agent/config")
        .send()
        .context("failed to reach NDE agent config endpoint")?
        .error_for_status()
        .context("NDE agent config endpoint returned an error")?;
    let body: NdeEnvelope<serde_json::Value> = response
        .json()
        .context("invalid NDE agent config response")?;
    if !body.success {
        bail!("NDE agent config request failed: {}", body.message);
    }

    let model = body
        .data
        .get("model")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if model.is_empty() {
        bail!("NDE agent config does not have an active model");
    }
    Ok(model)
}

fn resolve_command(program: &str) -> Option<PathBuf> {
    let output = if cfg!(windows) {
        Command::new("cmd")
            .args(["/C", "where", program])
            .output()
            .ok()?
    } else {
        Command::new("sh")
            .args(["-c", &format!("command -v {program}")])
            .output()
            .ok()?
    };
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(PathBuf::from)
}

fn resolve_python() -> Option<PathBuf> {
    resolve_command("python").or_else(|| resolve_command("python3"))
}

fn run_checked_command(mut command: Command, label: &str) -> Result<()> {
    let output = command
        .output()
        .with_context(|| format!("failed to execute {label}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    bail!("{label} failed: {stderr}");
}

fn default_voice_for_language(language: &str) -> String {
    match language.to_ascii_lowercase().as_str() {
        "ja" | "jp" => "ja-JP-NanamiNeural".to_string(),
        "ko" => "ko-KR-SunHiNeural".to_string(),
        "zh" | "zh-cn" => "zh-CN-XiaoxiaoNeural".to_string(),
        "es" => "es-ES-ElviraNeural".to_string(),
        "fr" => "fr-FR-DeniseNeural".to_string(),
        "de" => "de-DE-KatjaNeural".to_string(),
        _ => "en-US-AriaNeural".to_string(),
    }
}

fn slugify(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut previous_dash = false;
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash {
            out.push('-');
            previous_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn prettify_speaker_label(speaker_id: &str, index: usize) -> String {
    let label = speaker_id
        .replace('-', " ")
        .replace('_', " ")
        .trim()
        .to_string();
    if label.is_empty() {
        format!("Speaker {index}")
    } else {
        label
            .split_whitespace()
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_srt_and_detects_speakers() {
        let srt = "1\n00:00:01,000 --> 00:00:03,000\n[hero] We have to move.\n\n2\n00:00:03,500 --> 00:00:05,000\nvillain: Too late.\n";
        let mut segments = parse_srt(srt).expect("parse srt");
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].speaker_id.as_deref(), Some("hero"));
        assert_eq!(segments[0].text, "We have to move.");
        let speakers = build_speakers_from_segments(&mut segments);
        assert_eq!(speakers.len(), 2);
    }

    #[test]
    fn builds_default_narrator_when_no_speaker_labels() {
        let srt = "1\n00:00:01,000 --> 00:00:03,000\nHello there.\n";
        let mut segments = parse_srt(srt).expect("parse srt");
        let speakers = build_speakers_from_segments(&mut segments);
        assert_eq!(speakers[0].label, "Narrator");
        assert_eq!(segments[0].speaker_id.as_deref(), Some("speaker-narrator"));
    }

    #[test]
    fn parses_subtitle_timestamps() {
        assert_eq!(parse_timestamp("00:01:05,500").expect("timestamp"), 65.5);
    }
}
