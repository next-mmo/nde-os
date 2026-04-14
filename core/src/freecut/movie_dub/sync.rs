//! Audio time-stretching for dubbing synchronization.
//!
//! Implements WSOLA (Waveform Similarity Overlap-Add) to stretch or compress
//! audio without changing pitch. This is the critical piece that makes Khmer
//! dubs sound natural even when the translation is longer/shorter than the
//! original dialogue.
//!
//! Safe stretch range: 0.7x – 1.35x (beyond this, artifacts become audible).
//!
//! Pure Rust — no external dependencies.

use anyhow::Result;
use tracing::{debug, info, warn};

use super::config::SyncConfig;

/// WSOLA parameters.
const WINDOW_SIZE: usize = 1024;
const HOP_SIZE: usize = 256;
const SEARCH_RANGE: usize = 128;
const CROSSFADE_LEN: usize = 128;

/// Time-stretch audio samples by a given ratio using WSOLA.
///
/// - ratio > 1.0 → stretch (make longer / slower)
/// - ratio < 1.0 → compress (make shorter / faster)
/// - ratio = 1.0 → no change
pub fn time_stretch(samples: &[f32], ratio: f32) -> Result<Vec<f32>> {
    if samples.is_empty() {
        return Ok(Vec::new());
    }

    // Skip if ratio is close to 1.0.
    if (ratio - 1.0).abs() < 0.02 {
        return Ok(samples.to_vec());
    }

    if ratio < 0.5 || ratio > 2.0 {
        warn!("Stretch ratio {ratio:.2} outside safe range [0.5, 2.0]");
    }

    let input_len = samples.len();
    let output_len = (input_len as f32 * ratio) as usize;

    debug!(
        "WSOLA: {} samples → {} samples ({:.2}x)",
        input_len, output_len, ratio
    );

    let mut output = vec![0.0f32; output_len];
    let mut out_pos: usize = 0;

    let analysis_hop = HOP_SIZE;
    let synthesis_hop = (HOP_SIZE as f32 * ratio) as usize;

    let mut in_pos: usize = 0;

    // Hann window for smooth overlap.
    let window: Vec<f32> = (0..WINDOW_SIZE)
        .map(|i| {
            let t = i as f32 / (WINDOW_SIZE - 1) as f32;
            0.5 * (1.0 - (2.0 * std::f32::consts::PI * t).cos())
        })
        .collect();

    while in_pos + WINDOW_SIZE < input_len && out_pos + WINDOW_SIZE < output_len {
        let best_offset = find_best_overlap(
            samples,
            &output,
            in_pos,
            out_pos,
        );

        let src_start = (in_pos as i64 + best_offset as i64).max(0) as usize;

        if src_start + WINDOW_SIZE <= input_len {
            for i in 0..WINDOW_SIZE {
                if out_pos + i < output_len {
                    output[out_pos + i] += samples[src_start + i] * window[i];
                }
            }
        }

        in_pos += analysis_hop;
        out_pos += synthesis_hop;
    }

    // Normalize to prevent clipping.
    let max_val = output.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    if max_val > 1.0 {
        for s in &mut output {
            *s /= max_val;
        }
    }

    Ok(output)
}

/// Find best overlap offset using normalized cross-correlation.
fn find_best_overlap(
    input: &[f32],
    output: &[f32],
    in_pos: usize,
    out_pos: usize,
) -> i32 {
    let mut best_offset: i32 = 0;
    let mut best_corr: f32 = f32::NEG_INFINITY;

    let search_min = -(SEARCH_RANGE as i32);
    let search_max = SEARCH_RANGE as i32;

    for offset in search_min..=search_max {
        let src_i64 = in_pos as i64 + offset as i64;
        if src_i64 < 0 {
            continue;
        }
        let src = src_i64 as usize;
        if src + CROSSFADE_LEN > input.len() || out_pos + CROSSFADE_LEN > output.len() {
            continue;
        }

        let mut corr: f32 = 0.0;
        let mut energy_a: f32 = 0.0;
        let mut energy_b: f32 = 0.0;

        for i in 0..CROSSFADE_LEN {
            let a = input[src + i];
            let b = if out_pos + i < output.len() { output[out_pos + i] } else { 0.0 };
            corr += a * b;
            energy_a += a * a;
            energy_b += b * b;
        }

        let norm = (energy_a * energy_b).sqrt();
        let normalized = if norm > 1e-8 { corr / norm } else { 0.0 };

        if normalized > best_corr {
            best_corr = normalized;
            best_offset = offset;
        }
    }

    best_offset
}

/// Stretch audio to fit a target duration.
pub fn stretch_to_duration(
    samples: &[f32],
    sample_rate: u32,
    target_ms: u64,
    config: &SyncConfig,
) -> Result<Vec<f32>> {
    let current_ms = (samples.len() as f64 / sample_rate as f64 * 1000.0) as u64;
    let ratio = target_ms as f32 / current_ms as f32;

    info!(
        "Sync: {}ms → {}ms ({:.2}x stretch)",
        current_ms, target_ms, ratio
    );

    if ratio < config.min_stretch_ratio {
        warn!(
            "Stretch ratio {:.2} below min {:.2}, clamping",
            ratio, config.min_stretch_ratio
        );
        return time_stretch(samples, config.min_stretch_ratio);
    }

    if ratio > config.max_stretch_ratio {
        warn!(
            "Stretch ratio {:.2} above max {:.2}, clamping",
            ratio, config.max_stretch_ratio
        );
        return time_stretch(samples, config.max_stretch_ratio);
    }

    time_stretch(samples, ratio)
}

/// Apply fade in/out to avoid clicks at segment boundaries.
pub fn apply_fades(samples: &mut [f32], fade_ms: u64, sample_rate: u32) {
    let fade_samples = (sample_rate as u64 * fade_ms / 1000) as usize;
    let len = samples.len();

    // Fade in.
    for i in 0..fade_samples.min(len) {
        let t = i as f32 / fade_samples as f32;
        samples[i] *= t * t; // quadratic fade for smoother onset
    }

    // Fade out.
    for i in 0..fade_samples.min(len) {
        let idx = len - 1 - i;
        let t = i as f32 / fade_samples as f32;
        samples[idx] *= t * t;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sine(freq: f32, duration_ms: u64, sample_rate: u32) -> Vec<f32> {
        let num_samples = (sample_rate as u64 * duration_ms / 1000) as usize;
        (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * freq * t).sin()
            })
            .collect()
    }

    #[test]
    fn test_stretch_longer() {
        let sine = make_sine(440.0, 1000, 44100);
        let stretched = time_stretch(&sine, 1.5).unwrap();
        let expected_len = (sine.len() as f32 * 1.5) as usize;
        let tolerance = expected_len / 10;
        assert!(
            (stretched.len() as i64 - expected_len as i64).unsigned_abs() < tolerance as u64,
            "expected ~{expected_len}, got {}",
            stretched.len()
        );
    }

    #[test]
    fn test_stretch_shorter() {
        let sine = make_sine(440.0, 2000, 44100);
        let compressed = time_stretch(&sine, 0.75).unwrap();
        let expected_len = (sine.len() as f32 * 0.75) as usize;
        let tolerance = expected_len / 10;
        assert!(
            (compressed.len() as i64 - expected_len as i64).unsigned_abs() < tolerance as u64,
            "expected ~{expected_len}, got {}",
            compressed.len()
        );
    }

    #[test]
    fn test_no_stretch() {
        let sine = make_sine(440.0, 500, 44100);
        let result = time_stretch(&sine, 1.0).unwrap();
        assert_eq!(result.len(), sine.len());
    }
}
