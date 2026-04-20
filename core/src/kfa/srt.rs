//! SRT (SubRip Subtitle) formatting utilities.
//!
//! Takes word-level `Alignment` segments and groups them into
//! time-indexed SRT cues.

use crate::kfa::session::Alignment;

/// Format a timestamp (seconds) as `HH:MM:SS,mmm`.
pub fn format_timestamp(secs: f64) -> String {
    let total_ms = (secs * 1000.0).round() as u64;
    let ms = total_ms % 1000;
    let total_s = total_ms / 1000;
    let s = total_s % 60;
    let total_m = total_s / 60;
    let m = total_m % 60;
    let h = total_m / 60;
    format!("{:02}:{:02}:{:02},{:03}", h, m, s, ms)
}

/// Group `Alignment` segments into SRT cues of `words_per_cue` words each.
/// Returns the complete SRT file content as a `String`.
pub fn alignments_to_srt(segments: &[Alignment], words_per_cue: usize) -> String {
    if segments.is_empty() {
        return String::new();
    }

    let words_per_cue = words_per_cue.max(1);
    let mut srt = String::new();
    let mut index = 1usize;

    for chunk in segments.chunks(words_per_cue) {
        let start = chunk.first().map(|s| s.start).unwrap_or(0.0);
        let end = chunk.last().map(|s| s.end).unwrap_or(start);
        let text: String = chunk.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");

        srt.push_str(&format!("{}\n", index));
        srt.push_str(&format!(
            "{} --> {}\n",
            format_timestamp(start),
            format_timestamp(end)
        ));
        srt.push_str(&text);
        srt.push_str("\n\n");
        index += 1;
    }

    srt
}

/// Group segments into SRT cues where each cue is at most `max_secs` long.
/// Falls back to splitting by time rather than word count.
pub fn alignments_to_srt_by_duration(segments: &[Alignment], max_secs: f64) -> String {
    if segments.is_empty() {
        return String::new();
    }

    let mut srt = String::new();
    let mut index = 1usize;
    let mut cue_start = 0usize;

    while cue_start < segments.len() {
        let cue_time_start = segments[cue_start].start;
        let mut cue_end = cue_start + 1;

        // Extend cue until duration budget exceeded (min 1 word)
        while cue_end < segments.len()
            && (segments[cue_end - 1].end - cue_time_start) < max_secs
        {
            cue_end += 1;
        }

        let chunk = &segments[cue_start..cue_end];
        let start = chunk.first().map(|s| s.start).unwrap_or(0.0);
        let end = chunk.last().map(|s| s.end).unwrap_or(start);
        let text: String = chunk.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");

        srt.push_str(&format!("{}\n", index));
        srt.push_str(&format!(
            "{} --> {}\n",
            format_timestamp(start),
            format_timestamp(end)
        ));
        srt.push_str(&text);
        srt.push_str("\n\n");

        index += 1;
        cue_start = cue_end;
    }

    srt
}
