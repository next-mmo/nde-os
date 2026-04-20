//! CTC greedy decoding → Khmer text transcript.
//!
//! Uses the same wav2vec2 ONNX model as forced alignment but decodes
//! the emission matrix greedily (argmax per frame, collapse repeats,
//! remove blanks) to produce a raw Khmer-phoneme string, then maps
//! phonemes back to approximate Khmer words via the lexicon.

use anyhow::{anyhow, Result};
use ndarray::{concatenate, Array2, Axis};
use ort::session::builder::GraphOptimizationLevel;
use ort::session::{Session, SessionInputs};
use ort::value::{Tensor, Value};

use crate::kfa::alignment_utils::time_to_frame;
use crate::kfa::vocab::BLANK_ID;

const EMISSION_INTERVAL_SECS: f64 = 30.0;
const CONTEXT_RATIO: f64 = 0.1;

fn ort_err<T: std::fmt::Display>(e: T) -> anyhow::Error {
    anyhow!("{e}")
}

/// Run CTC greedy decoding on audio, returning a raw token-id transcript.
pub fn ctc_greedy_decode(
    samples: &[f32],
    sample_rate: u32,
    session: &mut Session,
) -> Result<Vec<usize>> {
    let total_duration = samples.len() as f64 / sample_rate as f64;
    let mut emissions_arr: Vec<Array2<f32>> = Vec::new();
    let mut i = 0.0_f64;

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
        let outputs = session.run(inputs).map_err(ort_err)?;

        let view = outputs[0].try_extract_array::<f32>().map_err(ort_err)?;
        let shape = view.shape().to_vec();
        if shape.len() != 3 || shape[0] != 1 {
            return Err(anyhow!("unexpected emissions shape: {:?}", shape));
        }
        let (frames, vocab) = (shape[1], shape[2]);
        let mut emissions = Array2::<f32>::zeros((frames, vocab));
        for (dst, src) in emissions.iter_mut().zip(view.iter().copied()) {
            *dst = src;
        }

        let emission_start_frame = time_to_frame(seg_start);
        let emission_end_frame = time_to_frame(seg_end);
        let offset = time_to_frame(input_start);
        let slice_start = emission_start_frame.saturating_sub(offset);
        let slice_end = (emission_end_frame.saturating_sub(offset)).min(frames);
        if slice_end > slice_start {
            emissions_arr.push(emissions.slice(ndarray::s![slice_start..slice_end, ..]).to_owned());
        }

        i += EMISSION_INTERVAL_SECS;
    }

    if emissions_arr.is_empty() {
        return Err(anyhow!("no emissions produced from audio"));
    }

    let views: Vec<_> = emissions_arr.iter().map(|a| a.view()).collect();
    let emissions: Array2<f32> = concatenate(Axis(0), &views)?;

    // Greedy CTC: argmax per frame
    let mut prev_token: Option<usize> = None;
    let mut token_ids: Vec<usize> = Vec::new();
    for row in emissions.rows() {
        let best = row
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(BLANK_ID);

        // Collapse repeats and strip blanks
        if best != BLANK_ID {
            if prev_token != Some(best) {
                token_ids.push(best);
            }
        }
        prev_token = Some(best);
    }

    Ok(token_ids)
}

/// Build a simple reverse vocab map: token_id → symbol string.
pub fn decode_tokens_to_text(token_ids: &[usize]) -> String {
    use crate::kfa::vocab::VOCABS;
    // Invert the vocab map
    let mut id_to_sym: Vec<&str> = vec![""; 64];
    for (sym, &id) in VOCABS.iter() {
        if id < id_to_sym.len() {
            id_to_sym[id] = sym;
        }
    }

    // Join phoneme symbols; use space as word separator at "|"
    let mut result = String::new();
    for &id in token_ids {
        let sym = if id < id_to_sym.len() { id_to_sym[id] } else { "" };
        if sym == "|" {
            result.push(' ');
        } else if !sym.is_empty() && sym != "[UNK]" && sym != "[PAD]" {
            result.push_str(sym);
        }
    }
    result.trim().to_string()
}

/// High-level: decode audio bytes → phoneme string transcript.
pub fn transcribe(
    samples: &[f32],
    sample_rate: u32,
    session: &mut Session,
) -> Result<String> {
    let ids = ctc_greedy_decode(samples, sample_rate, session)?;
    Ok(decode_tokens_to_text(&ids))
}

/// Create a new ONNX session (shared with AlignmentSession::new internals).
pub fn create_session(model_path: impl AsRef<std::path::Path>) -> Result<Session> {
    let session = Session::builder()
        .map_err(ort_err)?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(ort_err)?
        .commit_from_file(model_path.as_ref())
        .map_err(ort_err)?;
    Ok(session)
}
