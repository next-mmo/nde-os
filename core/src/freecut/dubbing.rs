//! Local dubbing support for FreeCut.
//!
//! This keeps the pipeline local-first:
//! - local SRT import
//! - local Whisper transcription
//! - optional NDE LLM line polish / translation
//! - Edge TTS synthesis (delegates to `core::voice::tts`)
//! - optional RVC CLI post-processing (delegates to `core::voice::rvc`)
//!
//! The voice runtime is now the shared NDE-OS global service managed by
//! `core::voice::runtime::VoiceRuntime`. This file no longer contains any
//! duplicated helpers for command resolution or synthesis — those live in
//! the `voice` module and are reused by any NDE-OS app.

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

use crate::voice::{
    rvc as voice_rvc,
    runtime::{run_checked_command, VoiceRuntime},
    tts as voice_tts,
    types::{RvcConvertRequest, TtsSynthesizeRequest},
};

// ─── Public result types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DubbingToolReport {
    pub whisper_available: bool,
    pub whisperx_available: bool,
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
    /// Enable speaker diarization via whisperx + pyannote.
    #[serde(default)]
    pub diarize: Option<bool>,
    /// HuggingFace auth token for pyannote speaker models.
    #[serde(default)]
    pub hf_token: Option<String>,
    /// Minimum expected speakers (hint for pyannote).
    #[serde(default)]
    pub min_speakers: Option<u32>,
    /// Maximum expected speakers (hint for pyannote).
    #[serde(default)]
    pub max_speakers: Option<u32>,
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
    #[allow(dead_code)]
    conversation_id: String,
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Detect which dubbing tools are available on this machine.
///
/// Delegates to the shared `VoiceRuntime` for tool detection instead of
/// duplicating `resolve_command` / `list_edge_voices` logic inline.
pub fn detect_local_tools() -> Result<DubbingToolReport> {
    // We don't have the base_dir here so we create a runtime that searches
    // the system PATH and the standard NDE-OS location.
    let base_dir = crate::app_manager::default_base_dir();
    let runtime = VoiceRuntime::new(&base_dir);
    let status = runtime.detect_status();

    let nde_llm_status = detect_nde_llm().ok();
    let mut details = status.details.clone();
    if nde_llm_status.is_none() {
        details.push("NDE LLM gateway not reachable on localhost:8080".to_string());
    }

    Ok(DubbingToolReport {
        whisper_available: status.whisper_available,
        whisperx_available: status.whisperx_available,
        edge_tts_available: status.edge_tts_available,
        nde_llm_available: nde_llm_status.is_some(),
        python_available: status.python_available,
        rvc_available: status.rvc_available,
        edge_voices: status.voices,
        nde_active_model: nde_llm_status,
        details,
    })
}

pub fn import_srt_as_session(file_path: &Path) -> Result<DubbingImportResult> {
    let raw = fs::read_to_string(file_path)
        .with_context(|| format!("failed to read SRT file {}", file_path.display()))?;
    // Strip UTF-8 BOM if present (common in Khmer subtitle exports).
    let content = raw.strip_prefix('\u{FEFF}').unwrap_or(&raw);
    let mut segments = parse_srt(content)?;
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

/// Auto-generate subtitles from a video file.
///
/// 1. Extract audio track to a temporary WAV via FFmpeg.
/// 2. Transcribe the WAV using local Whisper.
/// 3. Parse the generated SRT into `DubbingImportResult`.
///
/// `output_dir` is used for intermediate files (extracted WAV, generated SRT).
pub fn auto_generate_srt_from_video<F>(
    video_path: &Path,
    output_dir: &Path,
    whisper: WhisperSettings,
    translate_to: Option<String>,
    progress: F,
) -> Result<DubbingImportResult>
where
    F: Fn(DubbingProgressUpdate),
{
    if !video_path.exists() {
        bail!("video file not found: {}", video_path.display());
    }
    fs::create_dir_all(output_dir).with_context(|| {
        format!("failed to create output directory {}", output_dir.display())
    })?;

    let base_dir = crate::app_manager::default_base_dir();
    let runtime = VoiceRuntime::new(&base_dir);

    // Step 1: Extract audio from video to WAV (16kHz mono for Whisper).
    progress(DubbingProgressUpdate {
        phase: "extract".to_string(),
        current: 0,
        total: 4,
        message: "Extracting audio from video".to_string(),
    });

    let stem = video_path
        .file_stem()
        .and_then(|v| v.to_str())
        .unwrap_or("audio");
    let wav_path = output_dir.join(format!("{stem}_extracted.wav"));

    // Use FFmpeg to extract audio as 16kHz mono WAV (ideal for Whisper).
    let ffmpeg_bin = crate::media::ffmpeg::find_ffmpeg(&base_dir)
        .map(|bins| bins.ffmpeg)
        .unwrap_or_else(|| PathBuf::from("ffmpeg"));

    let extract_status = Command::new(&ffmpeg_bin)
        .args(["-y", "-i"])
        .arg(video_path)
        .args(["-vn", "-acodec", "pcm_s16le", "-ar", "16000", "-ac", "1"])
        .arg(&wav_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .status()
        .context("failed to run FFmpeg for audio extraction")?;

    if !extract_status.success() {
        bail!("FFmpeg audio extraction failed for {}", video_path.display());
    }
    if !wav_path.exists() {
        bail!("audio extraction produced no output at {}", wav_path.display());
    }

    // Step 2: Transcribe with Whisper.
    progress(DubbingProgressUpdate {
        phase: "transcribe".to_string(),
        current: 1,
        total: 4,
        message: "Transcribing audio with Whisper".to_string(),
    });

    let srt_path = transcribe_with_whisper(&wav_path, output_dir, whisper, &runtime)?;

    // Step 3: Parse the SRT into segments.
    progress(DubbingProgressUpdate {
        phase: "parse".to_string(),
        current: 2,
        total: 4,
        message: "Parsing generated subtitles".to_string(),
    });

    let mut result = import_srt_as_session(&srt_path)?;

    // Step 4: Translate if requested.
    if let Some(target_lang) = translate_to {
        let llm = DubbingLlmConfig {
            enabled: true,
            model: None,
            mode: Some("translate".to_string()),
        };
        let lang_str = if target_lang == "km" || target_lang.eq_ignore_ascii_case("khmer") { "Khmer" } else { &target_lang };
        
        progress(DubbingProgressUpdate {
            phase: "translate".to_string(),
            current: 3,
            total: 4,
            message: format!("Translating subtitles to {}", lang_str),
        });

        apply_nde_llm_to_segments(&mut result.segments, &llm, lang_str, &progress)?;

        // Overwrite the original text with the translated text.
        for seg in &mut result.segments {
            if let Some(translated) = seg.output_text.take() {
                seg.text = translated;
            }
        }
    }

    progress(DubbingProgressUpdate {
        phase: "done".to_string(),
        current: 4,
        total: 4,
        message: format!("Generated {} subtitle segments", result.segments.len()),
    });

    Ok(result)
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
    let base_dir = crate::app_manager::default_base_dir();
    let runtime = VoiceRuntime::new(&base_dir);

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
                    transcribe_with_whisper(Path::new(&source_media_path), &output_dir, whisper, &runtime)?;
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
            synthesize_with_rvc(&final_text, speaker, &out_path, &runtime)?;
        } else {
            synthesize_with_edge_tts(&final_text, speaker, &out_path, &session.target_language, &runtime)?;
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

// ─── Private implementation ───────────────────────────────────────────────────

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
                "Polish this subtitle text for spoken dubbing. Keep it concise and natural. Return only the polished subtitle text.\n\n{}",
                segment.text
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

/// Synthesize a segment using Edge TTS via the shared voice runtime.
fn synthesize_with_edge_tts(
    text: &str,
    speaker: &DubbingSpeaker,
    out_path: &Path,
    target_language: &str,
    runtime: &VoiceRuntime,
) -> Result<()> {
    let voice = if speaker.voice.trim().is_empty() {
        voice_tts::default_voice_for_language(target_language)
    } else {
        speaker.voice.clone()
    };

    let req = TtsSynthesizeRequest {
        text: text.to_string(),
        voice: voice.clone(),
        output_path: Some(out_path.to_string_lossy().to_string()),
        rate: speaker.rate.clone(),
        pitch: speaker.pitch.clone(),
        volume: speaker.volume.clone(),
    };

    voice_tts::synthesize(&req, runtime).map(|_| ())
}

/// Synthesize a segment using RVC (TTS then voice conversion) via the shared voice runtime.
fn synthesize_with_rvc(
    text: &str,
    speaker: &DubbingSpeaker,
    out_path: &Path,
    runtime: &VoiceRuntime,
) -> Result<()> {
    let rvc_cfg = speaker
        .rvc
        .as_ref()
        .ok_or_else(|| anyhow!("RVC requested but configuration is missing"))?;

    let model_path = rvc_cfg
        .model_path
        .clone()
        .ok_or_else(|| anyhow!("RVC model path is required when RVC is enabled"))?;
    let index_path = rvc_cfg
        .index_path
        .clone()
        .ok_or_else(|| anyhow!("RVC index path is required when RVC is enabled"))?;

    // Step 1: synthesize via TTS into a temp file
    let tts_voice = if speaker.voice.trim().is_empty() {
        "en-US-AriaNeural".to_string()
    } else {
        speaker.voice.clone()
    };
    let temp_tts_path = out_path.with_extension("edge.mp3");
    let tts_req = TtsSynthesizeRequest {
        text: text.to_string(),
        voice: tts_voice,
        output_path: Some(temp_tts_path.to_string_lossy().to_string()),
        rate: speaker.rate.clone(),
        pitch: speaker.pitch.clone(),
        volume: speaker.volume.clone(),
    };
    voice_tts::synthesize(&tts_req, runtime)?;

    // Step 2: run RVC conversion on the TTS output
    let req = RvcConvertRequest {
        input_audio: temp_tts_path.to_string_lossy().to_string(),
        model_path,
        index_path,
        output_path: Some(out_path.to_string_lossy().to_string()),
        pitch_shift: rvc_cfg.pitch_shift,
        python_path: rvc_cfg.python_path.clone(),
        cli_path: rvc_cfg.cli_path.clone(),
        output_format: "MP3".to_string(),
    };

    voice_rvc::convert(&req, runtime).map(|_| ())
}

/// Transcribe source media using the Whisper CLI from the shared voice runtime.
fn transcribe_with_whisper(
    source_media_path: &Path,
    output_dir: &Path,
    whisper: WhisperSettings,
    runtime: &VoiceRuntime,
) -> Result<PathBuf> {
    // Check if diarization is requested and whisperx is available.
    let want_diarize = whisper.diarize.unwrap_or(false);
    if want_diarize {
        if let Some(result) = try_transcribe_with_diarization(source_media_path, output_dir, &whisper, runtime) {
            return result;
        }
        // Fall through to plain whisper if diarization script fails to start.
    }

    let whisper_cli = runtime
        .resolve_tool("whisper")
        .ok_or_else(|| anyhow!("Whisper CLI not found; install the voice runtime via Service Hub or switch to SRT import"))?;

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

    // Find the generated SRT file
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

/// Attempt transcription with speaker diarization via the diarize.py script.
///
/// Returns `None` if the diarization script cannot be located (caller should
/// fall back to plain Whisper). Returns `Some(Result)` if it was attempted.
fn try_transcribe_with_diarization(
    source_media_path: &Path,
    output_dir: &Path,
    whisper: &WhisperSettings,
    runtime: &VoiceRuntime,
) -> Option<Result<PathBuf>> {
    // Resolve the diarize.py script — shipped alongside the core crate.
    let script_path = locate_diarize_script();
    if !script_path.exists() {
        tracing::warn!("diarize.py not found at {}, falling back to plain Whisper", script_path.display());
        return None;
    }

    // We MUST use the sandboxed venv python — whisperx is installed there, not
    // on system PATH. Falling back to a system interpreter causes
    // `ImportError: whisperx` even after a successful Service Hub install.
    let python = match runtime.venv_python() {
        Some(p) => p,
        None => {
            tracing::warn!(
                "Sandboxed voice-runtime venv not found at {}. Install the Voice Runtime (or WhisperX) via Service Hub first; falling back to plain Whisper.",
                runtime.bin_dir().display()
            );
            return None;
        }
    };

    let output_json = output_dir.join("diarized_output.json");
    let model = whisper.model.as_deref().unwrap_or("base");

    let mut command = Command::new(&python);
    command.arg(&script_path);
    command.arg(source_media_path);
    command.arg(&output_json);
    command.arg("--model").arg(model);

    if let Some(ref token) = whisper.hf_token {
        if !token.trim().is_empty() {
            command.arg("--hf_token").arg(token);
        }
    }
    if let Some(ref lang) = whisper.language {
        if !lang.trim().is_empty() && lang != "auto" {
            command.arg("--language").arg(lang);
        }
    }
    if let Some(min_spk) = whisper.min_speakers {
        command.arg("--min_speakers").arg(min_spk.to_string());
    }
    if let Some(max_spk) = whisper.max_speakers {
        command.arg("--max_speakers").arg(max_spk.to_string());
    }

    // Prepend the runtime venv bin to PATH so whisperx imports work.
    let runtime_path = runtime.prepend_path();
    command.env("PATH", &runtime_path);

    tracing::info!("Running diarize.py for speaker detection...");
    Some(run_diarization_and_build_srt(command, &output_json, output_dir, source_media_path))
}

/// Execute the diarization command and convert its JSON output to an SRT file
/// that includes speaker labels (e.g. `[SPEAKER_00]: text`).
fn run_diarization_and_build_srt(
    mut command: Command,
    json_path: &Path,
    output_dir: &Path,
    source_media_path: &Path,
) -> Result<PathBuf> {
    let output = command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .context("failed to execute diarize.py")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Diarization failed: {stderr}");
    }

    // Parse the JSON output.
    let data = fs::read_to_string(json_path)
        .with_context(|| format!("failed to read diarization output: {}", json_path.display()))?;

    #[derive(Deserialize)]
    struct DiarizeOutput {
        #[allow(dead_code)]
        language: Option<String>,
        #[allow(dead_code)]
        speakers: Option<Vec<String>>,
        segments: Vec<DiarizeSegment>,
    }
    #[derive(Deserialize)]
    struct DiarizeSegment {
        start: f64,
        end: f64,
        text: String,
        speaker: Option<String>,
    }

    let parsed: DiarizeOutput = serde_json::from_str(&data)
        .context("failed to parse diarization JSON")?;

    // Build SRT with speaker labels embedded.
    let stem = source_media_path
        .file_stem()
        .and_then(|v| v.to_str())
        .unwrap_or("output");
    let srt_path = output_dir.join(format!("{stem}.srt"));

    let mut srt_content = String::new();
    for (i, seg) in parsed.segments.iter().enumerate() {
        let start_ts = format_srt_timestamp(seg.start);
        let end_ts = format_srt_timestamp(seg.end);
        let text = match &seg.speaker {
            Some(spk) if !spk.is_empty() => format!("[{}]: {}", spk, seg.text.trim()),
            _ => seg.text.trim().to_string(),
        };
        srt_content.push_str(&format!("{}\n{} --> {}\n{}\n\n", i + 1, start_ts, end_ts, text));
    }

    fs::write(&srt_path, &srt_content)
        .with_context(|| format!("failed to write diarized SRT: {}", srt_path.display()))?;

    tracing::info!("Diarized SRT written: {} ({} segments)", srt_path.display(), parsed.segments.len());
    Ok(srt_path)
}

/// Format seconds as SRT timestamp: HH:MM:SS,mmm
fn format_srt_timestamp(secs: f64) -> String {
    let total_ms = (secs * 1000.0) as u64;
    let h = total_ms / 3_600_000;
    let m = (total_ms % 3_600_000) / 60_000;
    let s = (total_ms % 60_000) / 1_000;
    let ms = total_ms % 1_000;
    format!("{:02}:{:02}:{:02},{:03}", h, m, s, ms)
}

/// Locate the diarize.py script relative to the binary or source tree.
fn locate_diarize_script() -> PathBuf {
    // In development: relative to the workspace root.
    let candidates = [
        // Running from repo root or cargo test
        PathBuf::from("core/scripts/diarize.py"),
        // Running from core/ crate directory
        PathBuf::from("scripts/diarize.py"),
        // Walk up from CWD (Tauri dev runs from desktop/src-tauri/)
        PathBuf::from("../../core/scripts/diarize.py"),
    ];
    for candidate in &candidates {
        if candidate.exists() {
            return candidate.clone();
        }
    }
    // Try absolute using CARGO_MANIFEST_DIR (set at compile time for core crate)
    let manifest_dir = option_env!("CARGO_MANIFEST_DIR").unwrap_or(".");
    PathBuf::from(manifest_dir).join("scripts").join("diarize.py")
}

// ─── SRT parsing ─────────────────────────────────────────────────────────────

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

// ─── Speaker management ───────────────────────────────────────────────────────

fn build_speakers_from_segments(segments: &mut [DubbingSegment]) -> Vec<DubbingSpeaker> {
    /// A curated palette of distinct Edge TTS voices for auto-assignment.
    /// Includes diverse genders and styles to make multi-speaker dubs distinguishable.
    const VOICE_PALETTE: &[&str] = &[
        "en-US-AriaNeural",
        "en-US-GuyNeural",
        "en-US-JennyNeural",
        "en-US-DavisNeural",
        "en-US-AmberNeural",
        "en-US-AndrewNeural",
        "en-US-EmmaNeural",
        "en-US-BrianNeural",
    ];

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
            voice: VOICE_PALETTE[0].to_string(),
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
            voice: VOICE_PALETTE[idx % VOICE_PALETTE.len()].to_string(),
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
            voice: voice_tts::default_voice_for_language(&session.target_language),
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

// ─── Text helpers ─────────────────────────────────────────────────────────────

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

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_srt_basic() {
        let srt = "1\n00:00:01,000 --> 00:00:04,000\nHello world\n\n2\n00:00:05,000 --> 00:00:08,000\nSecond line";
        let segments = parse_srt(srt).expect("should parse");
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].text, "Hello world");
        assert_eq!(segments[1].text, "Second line");
    }

    #[test]
    fn parse_srt_with_speaker_labels() {
        let srt = "1\n00:00:01,000 --> 00:00:04,000\n[Alice]: Hello there\n\n2\n00:00:05,000 --> 00:00:08,000\nNarrator: Once upon a time";
        let segments = parse_srt(srt).expect("should parse");
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].speaker_id.as_deref(), Some("alice"));
        assert_eq!(segments[0].text, "Hello there");
        assert_eq!(segments[1].speaker_id.as_deref(), Some("narrator"));
    }

    #[test]
    fn slugify_handles_special_chars() {
        assert_eq!(slugify("Hello World!"), "hello-world");
        assert_eq!(slugify("  foo--bar  "), "foo-bar");
    }

    #[test]
    fn build_speakers_defaults_to_narrator() {
        let mut segments = vec![DubbingSegment {
            id: "seg-0001".to_string(),
            start_secs: 0.0,
            end_secs: 1.0,
            text: "hi".to_string(),
            output_text: None,
            speaker_id: None,
            audio_path: None,
            status: None,
        }];
        let speakers = build_speakers_from_segments(&mut segments);
        assert_eq!(speakers.len(), 1);
        assert_eq!(speakers[0].label, "Narrator");
    }

    #[test]
    fn merge_speakers_preserves_existing() {
        let existing = vec![DubbingSpeaker {
            id: "alice".to_string(),
            label: "Alice".to_string(),
            voice: "en-US-AriaNeural".to_string(),
            rate: None,
            pitch: None,
            volume: None,
            rvc: None,
        }];
        let imported = vec![
            DubbingSpeaker {
                id: "alice".to_string(),
                label: "Imported Alice".to_string(),
                voice: "en-US-JennyNeural".to_string(),
                rate: None,
                pitch: None,
                volume: None,
                rvc: None,
            },
            DubbingSpeaker {
                id: "bob".to_string(),
                label: "Bob".to_string(),
                voice: "en-US-GuyNeural".to_string(),
                rate: None,
                pitch: None,
                volume: None,
                rvc: None,
            },
        ];
        let merged = merge_speakers(&existing, &imported);
        assert_eq!(merged.len(), 2);
        // Existing alice should be preserved, not overwritten
        let alice = merged.iter().find(|s| s.id == "alice").unwrap();
        assert_eq!(alice.voice, "en-US-AriaNeural");
    }
}
