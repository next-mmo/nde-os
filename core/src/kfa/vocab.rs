//! CTC vocabulary table for the wav2vec2 Khmer model.

use once_cell::sync::Lazy;
use std::collections::HashMap;

pub static VOCABS: Lazy<HashMap<&'static str, usize>> = Lazy::new(|| {
    let entries: &[(&str, usize)] = &[
        (".", 0), ("a", 1), ("c", 2), ("e", 3), ("f", 4),
        ("g", 5), ("h", 6), ("i", 7), ("j", 8), ("k", 9),
        ("l", 10), ("m", 11), ("n", 12), ("o", 13), ("p", 14),
        ("r", 15), ("s", 16), ("t", 17), ("u", 18), ("w", 19),
        ("z", 20),
        ("\u{014b}", 21), // ŋ
        ("\u{0251}", 22), // ɑ
        ("\u{0253}", 23), // ɓ
        ("\u{0254}", 24), // ɔ
        ("\u{0257}", 25), // ɗ
        ("\u{0259}", 26), // ə
        ("\u{025b}", 27), // ɛ
        ("\u{0268}", 28), // ɨ
        ("\u{0272}", 29), // ɲ
        ("\u{0294}", 30), // ʔ
        ("|", 31), ("[UNK]", 32), ("[PAD]", 33),
    ];
    entries.iter().copied().collect()
});

pub const BLANK_ID: usize = 33; // [PAD]
pub const SEPARATOR_ID: usize = 31; // |

/// Look up a single token's id. Falls back to `[UNK]` (32).
pub fn lookup(ch: &str) -> usize {
    *VOCABS.get(ch).unwrap_or(&32)
}

pub fn time_to_frame(time: f64) -> usize {
    let frames_per_sec = 1000.0 / 20.0; // stride_msec = 20
    (time * frames_per_sec) as usize
}

pub fn intersperse<T: Clone>(lst: &[T], item: T) -> Vec<T> {
    let mut out = Vec::with_capacity(lst.len() * 2 + 1);
    out.push(item.clone());
    for x in lst {
        out.push(x.clone());
        out.push(item.clone());
    }
    out
}
