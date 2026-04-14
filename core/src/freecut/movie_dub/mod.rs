//! Movie dubbing pipeline for NDE-OS.
//!
//! Full end-to-end: Video → Extract Audio → Whisper STT → Translate (Lingva/LLM) →
//! Vocal Separation (demucs) → TTS (Edge TTS) → WSOLA Time-Stretch → Mix → Remux → MP4
//!
//! All external tools (ffmpeg, whisper, edge-tts, demucs) are resolved through the
//! NDE-OS sandbox infrastructure — never shelled out directly to host OS binaries.

pub mod config;
pub mod lang;
pub mod mix;
pub mod pipeline;
pub mod segment;
pub mod stt;
pub mod sync;
pub mod translate;

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

use config::MovieDubConfig;
use lang::Lang;
use mix::{MixParams, PlacedSegment};
use pipeline::{DubVideoOptions, Phase};
use segment::{Segment, TimedText};
use stt::SttEngine;
use translate::Translator;

/// The main movie dubbing pipeline.
///
/// Resolves all dependencies from NDE-OS sandbox:
/// - FFmpeg from `crate::media::ffmpeg`
/// - Whisper/edge-tts/demucs from `crate::voice::runtime::VoiceRuntime`
pub struct MovieDubPipeline {
    config: MovieDubConfig,
    /// Resolved sandboxed ffmpeg binary path.
    ffmpeg_path: PathBuf,
    /// Resolved edge-tts binary path (via VoiceRuntime).
    edge_tts_path: Option<PathBuf>,
    /// Resolved whisper binary path (via VoiceRuntime).
    whisper_path: Option<PathBuf>,
    /// Resolved demucs binary path (via VoiceRuntime).
    demucs_path: Option<PathBuf>,
    /// Sandbox workspace for temp files.
    workspace: PathBuf,
}

impl MovieDubPipeline {
    /// Create a movie dub pipeline from NDE-OS sandbox infrastructure.
    ///
    /// Resolves all dependencies from the base_dir sandbox:
    /// - FFmpeg: `crate::media::ffmpeg::ensure_ffmpeg(base_dir)`
    /// - Voice tools: `VoiceRuntime::new(base_dir).resolve_tool(...)`
    pub fn new(
        config: MovieDubConfig,
        base_dir: &Path,
        workspace: PathBuf,
    ) -> Result<Self> {
        // Resolve sandboxed FFmpeg.
        let ffmpeg_bins = crate::media::ffmpeg::ensure_ffmpeg(base_dir)
            .context("Failed to ensure FFmpeg via Service Hub")?;

        // Resolve voice tools via VoiceRuntime.
        let voice_rt = crate::voice::runtime::VoiceRuntime::new(base_dir);
        let edge_tts_path = voice_rt.resolve_tool("edge-tts");
        let whisper_path = voice_rt.resolve_tool("whisper");
        let demucs_path = voice_rt.resolve_tool("demucs");

        std::fs::create_dir_all(&workspace)?;

        Ok(Self {
            config,
            ffmpeg_path: ffmpeg_bins.ffmpeg,
            edge_tts_path,
            whisper_path,
            demucs_path,
            workspace,
        })
    }

    /// Full end-to-end: video file → dubbed MP4.
    pub async fn dub_video<F>(&self, opts: &DubVideoOptions, progress: F) -> Result<PathBuf>
    where
        F: Fn(Phase, f32, &str),
    {
        let video = &opts.input_path;
        if !video.exists() {
            anyhow::bail!("Video file not found: {}", video.display());
        }

        let work = &self.workspace;

        // Phase 1: Extract audio.
        progress(Phase::Extract, 0.0, "Extracting audio from video...");
        let audio_wav = work.join("extracted_audio.wav");
        let stt = SttEngine::new(
            &self.config.stt.model_size,
            work.join("stt_work"),
            self.ffmpeg_path.clone(),
            self.whisper_path.clone(),
        );
        stt.extract_audio(video, &audio_wav).await?;
        progress(Phase::Extract, 1.0, "Audio extracted");

        // Phase 2: Transcribe.
        progress(Phase::Transcribe, 0.0, &format!("Transcribing ({})...", opts.source_lang));
        let segments = stt.transcribe(&audio_wav, Some(opts.source_lang)).await?;
        progress(Phase::Transcribe, 1.0, &format!("{} segments found", segments.len()));

        // Save segments.
        let seg_json = serde_json::to_string_pretty(&segments)?;
        std::fs::write(work.join("segments.json"), &seg_json)?;

        // Phase 3: Translate.
        progress(Phase::Translate, 0.0, "Translating → Khmer...");
        let translator = Translator::from_config(&self.config.translation);
        let timed_texts = translator.translate_segments(&segments, Lang::Km).await?;
        progress(Phase::Translate, 1.0, &format!("{} segments translated", timed_texts.len()));

        // Generate subtitles if requested.
        let srt_path = work.join("khmer_subtitles.srt");
        if opts.generate_subtitles || opts.burn_subtitles {
            let srt_entries: Vec<(u64, u64, String)> = timed_texts.iter()
                .map(|t| (t.segment.start_ms, t.segment.end_ms, t.translated_text.clone()))
                .collect();
            mix::generate_srt(&srt_entries, &srt_path)?;
        }

        // Phase 4: Separate background audio.
        progress(Phase::Separate, 0.0, "Separating vocals from background...");
        let bg_path = work.join("separated");
        let bg_audio = match mix::separate_audio(&audio_wav, &bg_path, self.demucs_path.as_deref(), &self.ffmpeg_path).await {
            Ok((_, bg)) => {
                progress(Phase::Separate, 1.0, "Vocal separation complete");
                bg
            }
            Err(e) => {
                warn!("Vocal separation failed: {e}");
                progress(Phase::Separate, 1.0, "Separation failed, using silence");
                let len = mix::wav_duration_ms(&audio_wav)
                    .map(|ms| mix::ms_to_samples(ms, self.config.output.sample_rate))
                    .unwrap_or(44100);
                vec![0.0f32; len]
            }
        };

        // Phase 5: TTS + Sync.
        progress(Phase::Synthesize, 0.0, "Generating Khmer speech...");
        let edge_tts = self.edge_tts_path.as_ref().ok_or_else(|| {
            anyhow::anyhow!("edge-tts not found. Install via Service Hub → Voice Runtime")
        })?;

        let tts_cache = work.join("tts_cache");
        std::fs::create_dir_all(&tts_cache)?;

        let sample_rate = self.config.output.sample_rate;
        let mut placed_segments = Vec::new();
        let total = timed_texts.len();

        for (idx, tt) in timed_texts.iter().enumerate() {
            progress(
                Phase::Synthesize,
                idx as f32 / total as f32,
                &format!("Synthesizing segment {}/{}", idx + 1, total),
            );

            let wav_path = tts_cache.join(format!("seg_{:04}.wav", tt.segment.id));
            if let Err(e) = mix::synthesize_tts(
                &self.ffmpeg_path,
                edge_tts,
                &tt.translated_text,
                &self.config.tts.edge_voice,
                self.config.tts.speed,
                &wav_path,
            ).await {
                tracing::warn!("TTS failed for segment {}: {}, skipping", tt.segment.id, e);
                continue;
            }

            let (samples, sr) = mix::load_wav_samples(&wav_path)?;
            let actual_ms = mix::wav_duration_ms(&wav_path)?;
            let target_ms = tt.segment.duration_ms();

            // Time-stretch if needed.
            let ratio = actual_ms as f32 / target_ms as f32;
            let final_samples = if (ratio - 1.0).abs() > 0.05 {
                sync::stretch_to_duration(&samples, sr, target_ms, &self.config.sync)?
            } else {
                samples
            };

            // Fade edges.
            let mut faded = final_samples;
            sync::apply_fades(&mut faded, 10, sr);

            placed_segments.push(PlacedSegment {
                start_sample: mix::ms_to_samples(tt.segment.start_ms, sr),
                samples: faded,
            });
        }
        progress(Phase::Synthesize, 1.0, &format!("{} segments synthesized", placed_segments.len()));

        // Phase 6: Mix.
        progress(Phase::Mix, 0.0, "Mixing final audio...");
        let mix_params = MixParams {
            bg_volume: 0.3,
            voice_volume: 1.0,
            total_samples: bg_audio.len(),
            sample_rate,
        };
        let final_audio = mix::mix_final(&bg_audio, &placed_segments, &mix_params);

        let dubbed_wav = work.join("_dubbed_temp.wav");
        mix::write_wav(&dubbed_wav, &final_audio, sample_rate)?;
        progress(Phase::Mix, 1.0, "Audio mixed");

        // Phase 7: Remux → MP4.
        progress(Phase::Export, 0.0, "Building final MP4...");
        let output_mp4 = &opts.output_path;

        if opts.dual_audio {
            mix::remux_video_dual_audio(&self.ffmpeg_path, video, &dubbed_wav, output_mp4).await?;
        } else {
            mix::remux_video(&self.ffmpeg_path, video, &dubbed_wav, output_mp4).await?;
        }

        // Optional: burn subtitles.
        if opts.burn_subtitles && srt_path.exists() {
            let burned_path = PathBuf::from(
                output_mp4.to_string_lossy().replace(".mp4", "_subtitled.mp4")
            );
            mix::burn_subtitles(&self.ffmpeg_path, output_mp4, &srt_path, &burned_path).await?;
        }

        // Clean up temp.
        let _ = std::fs::remove_file(&dubbed_wav);

        progress(Phase::Export, 1.0, "Done");
        info!("Movie dub complete: {}", output_mp4.display());

        Ok(output_mp4.clone())
    }

    /// Transcribe video/audio → segments JSON.
    pub async fn transcribe(
        &self,
        input_path: &Path,
        source_lang: Option<Lang>,
    ) -> Result<Vec<Segment>> {
        let work = &self.workspace;
        let stt = SttEngine::new(
            &self.config.stt.model_size,
            work.join("stt_work"),
            self.ffmpeg_path.clone(),
            self.whisper_path.clone(),
        );

        // If video, extract audio first.
        let audio_path = if is_video(input_path) {
            let wav = work.join("extracted_audio.wav");
            stt.extract_audio(input_path, &wav).await?;
            wav
        } else {
            input_path.to_path_buf()
        };

        stt.transcribe(&audio_path, source_lang).await
    }

    /// Translate segments → Khmer.
    pub async fn translate_segments(
        &self,
        segments: &[Segment],
        target_lang: Lang,
    ) -> Result<Vec<TimedText>> {
        let translator = Translator::from_config(&self.config.translation);
        translator.translate_segments(segments, target_lang).await
    }

    /// Dub from pre-translated segments → dubbed WAV (no video).
    pub async fn dub_audio(
        &self,
        timed_texts: &[TimedText],
        bg_audio: Option<Vec<f32>>,
        output_path: &Path,
    ) -> Result<PathBuf> {
        let edge_tts = self.edge_tts_path.as_ref().ok_or_else(|| {
            anyhow::anyhow!("edge-tts not found. Install via Service Hub → Voice Runtime")
        })?;

        let tts_cache = self.workspace.join("tts_cache");
        std::fs::create_dir_all(&tts_cache)?;

        let sample_rate = self.config.output.sample_rate;
        let mut placed_segments = Vec::new();

        for tt in timed_texts {
            let wav_path = tts_cache.join(format!("seg_{:04}.wav", tt.segment.id));
            mix::synthesize_tts(
                &self.ffmpeg_path,
                edge_tts,
                &tt.translated_text,
                &self.config.tts.edge_voice,
                self.config.tts.speed,
                &wav_path,
            ).await?;

            let (samples, sr) = mix::load_wav_samples(&wav_path)?;
            let actual_ms = mix::wav_duration_ms(&wav_path)?;
            let target_ms = tt.segment.duration_ms();

            let ratio = actual_ms as f32 / target_ms as f32;
            let final_samples = if (ratio - 1.0).abs() > 0.05 {
                sync::stretch_to_duration(&samples, sr, target_ms, &self.config.sync)?
            } else {
                samples
            };

            let mut faded = final_samples;
            sync::apply_fades(&mut faded, 10, sr);

            placed_segments.push(PlacedSegment {
                start_sample: mix::ms_to_samples(tt.segment.start_ms, sr),
                samples: faded,
            });
        }

        let bg = bg_audio.unwrap_or_else(|| {
            let max_end = timed_texts.iter()
                .map(|t| mix::ms_to_samples(t.segment.end_ms, sample_rate))
                .max()
                .unwrap_or(0);
            vec![0.0f32; max_end + sample_rate as usize]
        });

        let mix_params = MixParams {
            bg_volume: if bg.iter().any(|s| *s != 0.0) { 0.3 } else { 0.0 },
            voice_volume: 1.0,
            total_samples: bg.len(),
            sample_rate,
        };

        let final_audio = mix::mix_final(&bg, &placed_segments, &mix_params);
        mix::write_wav(output_path, &final_audio, sample_rate)?;

        Ok(output_path.to_path_buf())
    }
}

/// Check if a file is a video based on extension.
fn is_video(path: &Path) -> bool {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(ext.as_str(), "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" | "wmv")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization() {
        let config = MovieDubConfig::default();
        let json = serde_json::to_string_pretty(&config).unwrap();
        let parsed: MovieDubConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.tts.edge_voice, "km-KH-PisethNeural");
        assert_eq!(parsed.output.sample_rate, 44100);
        assert_eq!(parsed.sync.max_stretch_ratio, 1.3);
    }

    #[test]
    fn test_is_video() {
        assert!(is_video(Path::new("movie.mp4")));
        assert!(is_video(Path::new("movie.mkv")));
        assert!(is_video(Path::new("movie.MOV")));
        assert!(!is_video(Path::new("audio.wav")));
        assert!(!is_video(Path::new("audio.mp3")));
    }
}
