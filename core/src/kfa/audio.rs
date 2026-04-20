//! WAV → mono 16 kHz f32 loader.

use anyhow::{anyhow, Result};
use hound::{SampleFormat, WavReader};
use std::io::Read;

pub const TARGET_SR: u32 = 16_000;

/// Load WAV bytes (in memory), downmix to mono, resample to 16 kHz.
pub fn load_wav_mono_16k_bytes(data: &[u8]) -> Result<(Vec<f32>, u32)> {
    let cursor = std::io::Cursor::new(data);
    let mut reader = WavReader::new(cursor)?;
    let spec = reader.spec();
    let channels = spec.channels as usize;

    let samples: Vec<f32> = match (spec.sample_format, spec.bits_per_sample) {
        (SampleFormat::Float, 32) => reader
            .samples::<f32>()
            .collect::<Result<Vec<_>, _>>()?,
        (SampleFormat::Int, 16) => reader
            .samples::<i16>()
            .map(|s| s.map(|v| v as f32 / i16::MAX as f32))
            .collect::<Result<Vec<_>, _>>()?,
        (SampleFormat::Int, 24) | (SampleFormat::Int, 32) => {
            let max = 2f32.powi(spec.bits_per_sample as i32 - 1);
            reader
                .samples::<i32>()
                .map(|s| s.map(|v| v as f32 / max))
                .collect::<Result<Vec<_>, _>>()?
        }
        (fmt, bits) => {
            return Err(anyhow!(
                "unsupported WAV sample format: {:?}, {} bits",
                fmt,
                bits
            ))
        }
    };

    let mono: Vec<f32> = if channels <= 1 {
        samples
    } else {
        samples
            .chunks(channels)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
            .collect()
    };

    let resampled = if spec.sample_rate == TARGET_SR {
        mono
    } else {
        resample_linear(&mono, spec.sample_rate, TARGET_SR)
    };

    Ok((resampled, TARGET_SR))
}

fn resample_linear(input: &[f32], from_sr: u32, to_sr: u32) -> Vec<f32> {
    if from_sr == to_sr || input.is_empty() {
        return input.to_vec();
    }
    let ratio = to_sr as f64 / from_sr as f64;
    let out_len = ((input.len() as f64) * ratio).round() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 / ratio;
        let idx = src_pos.floor() as usize;
        let frac = (src_pos - idx as f64) as f32;
        let a = input[idx.min(input.len() - 1)];
        let b = input[(idx + 1).min(input.len() - 1)];
        out.push(a + (b - a) * frac);
    }
    out
}
