//! Forced alignment session: wav2vec2 CTC ONNX model + text pipeline.
//!
//! Downloads the model once to the OS cache directory, then runs
//! chunked inference + Viterbi alignment.

use anyhow::{anyhow, Context, Result};
use ndarray::{concatenate, Array2, Axis};
use ort::session::builder::GraphOptimizationLevel;
use ort::session::{Session, SessionInputs};
use ort::value::{Tensor, Value};
use serde::{Deserialize, Serialize};

use crate::kfa::alignment_utils::{
    backtrack, get_trellis, intersperse, log_softmax_last_axis, merge_repeats, merge_words,
    time_to_frame,
};
use crate::kfa::text_normalize::{PhonemizedToken, TextPipeline};
use crate::kfa::vocab::{BLANK_ID, SEPARATOR_ID};

const MODEL_URL: &str =
    "https://huggingface.co/seanghay/wav2vec2-base-khmer-phonetisaurus/resolve/main/wav2vec2-km-base-1500.onnx";
const EMISSION_INTERVAL_SECS: f64 = 30.0;
const CONTEXT_RATIO: f64 = 0.1;
pub const SAMPLE_RATE: u32 = 16_000;

fn get_model_path() -> Result<std::path::PathBuf> {
    let cache_dir = dirs::cache_dir().unwrap_or_else(|| std::path::PathBuf::from("./.cache"));
    let kfa_dir = cache_dir.join("nde-kfa");
    std::fs::create_dir_all(&kfa_dir)?;
    let model_path = kfa_dir.join("wav2vec2-km-base-1500.onnx");
    let tmp_path = kfa_dir.join("wav2vec2-km-base-1500.onnx.tmp");

    if !model_path.exists() {
        tracing::info!(url = MODEL_URL, "Downloading KFA ONNX model…");
        let response = reqwest::blocking::get(MODEL_URL)
            .context("Failed to download KFA ONNX model")?;
        let content = response.bytes().context("Failed to read model bytes")?;
        let mut dest = std::fs::File::create(&tmp_path)?;
        std::io::copy(&mut content.as_ref(), &mut dest)?;
        std::fs::rename(&tmp_path, &model_path)?;
        tracing::info!("KFA model saved to {}", model_path.display());
    }
    Ok(model_path)
}

fn ort_err<T: std::fmt::Display>(e: T) -> anyhow::Error {
    anyhow!("{e}")
}

/// A single word-level aligned segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alignment {
    pub text: String,
    pub start: f64,
    pub end: f64,
    pub actual_start: f64,
    pub actual_end: f64,
    pub score: f64,
}

/// Holds the ONNX session and text pipeline. Reuse across requests for performance.
pub struct AlignmentSession {
    session: Session,
    pipeline: TextPipeline,
}

impl AlignmentSession {
    /// Create a new session. Downloads model on first call.
    pub fn new(use_cuda: bool) -> Result<Self> {
        let _ = ort::init().with_name("NDE-KFA").commit();

        let model_path = get_model_path()?;

        let mut builder = Session::builder()
            .map_err(ort_err)?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(ort_err)?;

        if use_cuda {
            use ort::ep::CUDA;
            builder = builder
                .with_execution_providers([CUDA::default().build()])
                .map_err(|e| anyhow!("CUDA EP failed: {e}"))?;
        }

        let session = builder
            .commit_from_file(&model_path)
            .map_err(ort_err)?;

        let pipeline = TextPipeline::new()?;

        Ok(Self { session, pipeline })
    }

    /// Run forced alignment. Returns one `Alignment` per word-like segment.
    ///
    /// `progress` is called with `(chunks_done, total_chunks)` for each 30-s chunk.
    pub fn align(
        &mut self,
        samples: &[f32],
        sample_rate: u32,
        text: &str,
        progress: Option<&dyn Fn(u64, u64)>,
    ) -> Result<Vec<Alignment>> {
        let total_duration = samples.len() as f64 / sample_rate as f64;
        let total_chunks = (total_duration / EMISSION_INTERVAL_SECS).ceil().max(1.0) as u64;

        let mut emissions_arr: Vec<Array2<f32>> = Vec::new();
        let mut i = 0.0_f64;
        let mut processed = 0_u64;

        while i < total_duration {
            let seg_start = i;
            let seg_end = i + EMISSION_INTERVAL_SECS;
            let context = EMISSION_INTERVAL_SECS * CONTEXT_RATIO;
            let input_start = (seg_start - context).max(0.0);
            let input_end = (seg_end + context).min(total_duration);

            let start_sample = (sample_rate as f64 * input_start) as usize;
            let end_sample = (sample_rate as f64 * input_end) as usize;
            let y_chunk: Vec<f32> = samples[start_sample..end_sample].to_vec();
            let chunk_len = y_chunk.len();

            let shape = [1_i64, chunk_len as i64];
            let tensor: Tensor<f32> = Tensor::from_array((shape, y_chunk)).map_err(ort_err)?;
            let input_value: Value = tensor.into_dyn();
            let inputs: SessionInputs<1> =
                SessionInputs::ValueMap(vec![("input".into(), input_value.into())]);
            let outputs = self.session.run(inputs).map_err(ort_err)?;

            let emissions_view = outputs[0].try_extract_array::<f32>().map_err(ort_err)?;
            let shape = emissions_view.shape().to_vec();
            if shape.len() != 3 || shape[0] != 1 {
                return Err(anyhow!("unexpected emissions shape: {:?}", shape));
            }
            let (frames, vocab) = (shape[1], shape[2]);
            let mut emissions = Array2::<f32>::zeros((frames, vocab));
            for (dst, src) in emissions.iter_mut().zip(emissions_view.iter().copied()) {
                *dst = src;
            }

            let emission_start_frame = time_to_frame(seg_start);
            let emission_end_frame = time_to_frame(seg_end);
            let offset = time_to_frame(input_start);
            let slice_start = emission_start_frame.saturating_sub(offset);
            let slice_end = (emission_end_frame.saturating_sub(offset)).min(frames);
            if slice_end > slice_start {
                let sliced = emissions.slice(ndarray::s![slice_start..slice_end, ..]).to_owned();
                emissions_arr.push(sliced);
            }

            i += EMISSION_INTERVAL_SECS;
            processed += 1;
            if let Some(cb) = progress {
                cb(processed, total_chunks);
            }
        }

        if emissions_arr.is_empty() {
            return Err(anyhow!("no emissions produced from audio"));
        }

        let views: Vec<_> = emissions_arr.iter().map(|a| a.view()).collect();
        let emissions: Array2<f32> = concatenate(Axis(0), &views)?;

        let (frames, vocab) = emissions.dim();
        let mut emission = Array2::<f64>::zeros((frames, vocab));
        for ((r, c), v) in emissions.indexed_iter() {
            emission[[r, c]] = *v as f64;
        }
        log_softmax_last_axis(&mut emission);

        // Tokenize + phonemize each line
        let mut text_sequences: Vec<PhonemizedToken> = Vec::new();
        for line in text.split('\n') {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            let segs = self.pipeline.tokenize_phonemize(l)?;
            text_sequences.extend(segs);
        }

        let mut tokens: Vec<Vec<usize>> = Vec::new();
        let mut texts: Vec<String> = Vec::new();
        let mut spans: Vec<usize> = Vec::new();

        for item in &text_sequences {
            match item {
                PhonemizedToken::Unknown { .. } => {
                    if let Some(last) = spans.last_mut() {
                        *last += 1;
                    }
                }
                PhonemizedToken::Known { lattice, token_ids, .. } => {
                    spans.push(0);
                    tokens.push(token_ids.clone());
                    texts.push(lattice.clone());
                }
            }
        }

        if tokens.is_empty() {
            return Ok(Vec::new());
        }

        let joined_text: String = intersperse(&texts, "|".to_string()).join("");
        let joined_tokens: Vec<usize> = {
            let with_sep = intersperse(&tokens, vec![SEPARATOR_ID]);
            with_sep.into_iter().flatten().collect()
        };

        let trellis = get_trellis(emission.view(), &joined_tokens, BLANK_ID);
        let path = backtrack(&trellis, emission.view(), &joined_tokens, BLANK_ID);
        let transcript_chars: Vec<char> = joined_text.chars().collect();
        let segments = merge_repeats(&path, &transcript_chars);
        let word_segments = merge_words(&segments, "|");

        let ratio = samples.len() as f64 / trellis.shape()[0] as f64;
        let mut second_start = 0.0_f64;
        let mut results: Vec<Alignment> = Vec::with_capacity(word_segments.len());

        for (i, word) in word_segments.iter().enumerate() {
            let actual_second_start = ratio * word.start as f64 / sample_rate as f64;
            let mut second_end = ratio * word.end as f64 / sample_rate as f64;
            let actual_second_end = second_end;
            if i + 1 < word_segments.len() {
                let next_start = ratio * word_segments[i + 1].start as f64 / sample_rate as f64;
                if next_start > second_end {
                    second_end = next_start;
                }
            }

            let seq_idx: usize = spans.iter().take(i).sum::<usize>() + i;
            let span_size = spans.get(i).copied().unwrap_or(0);
            let mut text_segment = String::new();
            let end = (seq_idx + span_size + 1).min(text_sequences.len());
            for t in &text_sequences[seq_idx..end] {
                match t {
                    PhonemizedToken::Known { token, .. } => text_segment.push_str(token),
                    PhonemizedToken::Unknown { token } => text_segment.push_str(token),
                }
            }

            results.push(Alignment {
                text: text_segment,
                start: second_start,
                end: second_end,
                actual_start: actual_second_start,
                actual_end: actual_second_end,
                score: word.score,
            });
            second_start = second_end;
        }

        Ok(results)
    }
}
