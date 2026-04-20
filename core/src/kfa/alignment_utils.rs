//! CTC forced-alignment utilities.
//! Ported from Python `kfa/utils.py` using ndarray in place of numpy.

use ndarray::{Array2, ArrayView2};

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub token_index: usize,
    pub time_index: usize,
    pub score: f64,
}

#[derive(Debug, Clone)]
pub struct Segment {
    pub label: String,
    pub start: usize,
    pub end: usize,
    pub score: f64,
}

impl Segment {
    pub fn length(&self) -> usize {
        self.end - self.start
    }
}

/// Inserts `item` between every pair of elements in `lst`, plus surrounding padding.
/// Result has length `2 * lst.len() + 1`.
pub fn intersperse<T: Clone>(lst: &[T], item: T) -> Vec<T> {
    let mut out = Vec::with_capacity(lst.len() * 2 + 1);
    out.push(item.clone());
    for x in lst {
        out.push(x.clone());
        out.push(item.clone());
    }
    out
}

pub fn time_to_frame(time: f64) -> usize {
    let frames_per_sec = 1000.0 / 20.0;
    (time * frames_per_sec) as usize
}

/// Build the CTC trellis from log-softmax emissions and a token sequence.
pub fn get_trellis(emission: ArrayView2<f64>, tokens: &[usize], blank_id: usize) -> Array2<f64> {
    let num_frame = emission.shape()[0];
    let num_tokens = tokens.len();
    let mut trellis = Array2::<f64>::zeros((num_frame, num_tokens));

    let mut running = 0.0_f64;
    for t in 1..num_frame {
        running += emission[[t, blank_id]];
        trellis[[t, 0]] = running;
    }
    for j in 1..num_tokens {
        trellis[[0, j]] = f64::NEG_INFINITY;
    }
    if num_tokens > 1 && num_frame >= num_tokens - 1 {
        let start_t = num_frame.saturating_sub(num_tokens - 1);
        for t in start_t..num_frame {
            trellis[[t, 0]] = f64::INFINITY;
        }
    }
    for t in 0..num_frame - 1 {
        for j in 1..num_tokens {
            let stay = trellis[[t, j]] + emission[[t, blank_id]];
            let change = trellis[[t, j - 1]] + emission[[t, tokens[j]]];
            trellis[[t + 1, j]] = stay.max(change);
        }
    }
    trellis
}

/// Backtrack through the trellis to recover the best path.
pub fn backtrack(
    trellis: &Array2<f64>,
    emission: ArrayView2<f64>,
    tokens: &[usize],
    blank_id: usize,
) -> Vec<Point> {
    let mut t = trellis.shape()[0] - 1;
    let mut j = trellis.shape()[1] - 1;

    let mut path = vec![Point {
        token_index: j,
        time_index: t,
        score: emission[[t, blank_id]].exp(),
    }];

    while j > 0 {
        assert!(t > 0, "time index exhausted before reaching start token");
        let p_stay = emission[[t - 1, blank_id]];
        let p_change = emission[[t - 1, tokens[j]]];
        let stayed = trellis[[t - 1, j]] + p_stay;
        let changed = trellis[[t - 1, j - 1]] + p_change;
        t -= 1;
        let (prob, advance) = if changed > stayed {
            (p_change.exp(), true)
        } else {
            (p_stay.exp(), false)
        };
        if advance {
            j -= 1;
        }
        path.push(Point { token_index: j, time_index: t, score: prob });
    }
    while t > 0 {
        let prob = emission[[t - 1, blank_id]].exp();
        path.push(Point { token_index: j, time_index: t - 1, score: prob });
        t -= 1;
    }
    path.reverse();
    path
}

/// Merge consecutive points that refer to the same token index into segments.
pub fn merge_repeats(path: &[Point], transcript: &[char]) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut i1 = 0;
    while i1 < path.len() {
        let mut i2 = i1;
        while i2 < path.len() && path[i1].token_index == path[i2].token_index {
            i2 += 1;
        }
        let score: f64 = (i1..i2).map(|k| path[k].score).sum::<f64>() / (i2 - i1) as f64;
        let label = if path[i1].token_index < transcript.len() {
            transcript[path[i1].token_index].to_string()
        } else {
            String::new()
        };
        segments.push(Segment {
            label,
            start: path[i1].time_index,
            end: path[i2 - 1].time_index + 1,
            score,
        });
        i1 = i2;
    }
    segments
}

/// Merge character-level segments into word-level segments, splitting on `separator`.
pub fn merge_words(segments: &[Segment], separator: &str) -> Vec<Segment> {
    let mut words = Vec::new();
    let mut i1 = 0;
    let mut i2 = 0;
    while i1 < segments.len() {
        if i2 >= segments.len() || segments[i2].label == separator {
            if i1 != i2 {
                let segs = &segments[i1..i2];
                let word: String = segs.iter().map(|s| s.label.as_str()).collect();
                let total_len: usize = segs.iter().map(|s| s.length()).sum();
                let score = if total_len == 0 {
                    0.0
                } else {
                    segs.iter().map(|s| s.score * s.length() as f64).sum::<f64>() / total_len as f64
                };
                words.push(Segment {
                    label: word,
                    start: segments[i1].start,
                    end: segments[i2 - 1].end,
                    score,
                });
            }
            i1 = i2 + 1;
            i2 = i1;
        } else {
            i2 += 1;
        }
    }
    words
}

/// Apply log-softmax along the last axis (rows) of a 2-D f64 array, in place.
pub fn log_softmax_last_axis(arr: &mut Array2<f64>) {
    for mut row in arr.rows_mut() {
        let max = row.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let mut sum = 0.0;
        for v in row.iter_mut() {
            *v -= max;
            sum += v.exp();
        }
        let log_sum = sum.ln();
        for v in row.iter_mut() {
            *v -= log_sum;
        }
    }
}
