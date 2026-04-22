//! SRT parsing and building utilities for the translation service.

use anyhow::{Context, Result};

/// A parsed SRT cue.
#[derive(Debug, Clone)]
pub struct SrtCue {
    /// 1-indexed sequence number.
    pub index: u32,
    /// Start timestamp in milliseconds.
    pub start_ms: u64,
    /// End timestamp in milliseconds.
    pub end_ms: u64,
    /// Subtitle text (may span multiple lines).
    pub text: String,
}

/// Parse an SRT string into cues.
///
/// Handles:
/// - UTF-8 BOM
/// - Both `\r\n` and `\n` line endings
/// - Comma and period timestamp separators
/// - Multi-line subtitle text
pub fn parse_srt(content: &str) -> Result<Vec<SrtCue>> {
    // Strip UTF-8 BOM if present.
    let content = content.strip_prefix('\u{FEFF}').unwrap_or(content);
    let mut cues = Vec::new();
    let mut lines = content.lines().peekable();

    while lines.peek().is_some() {
        // Skip blank lines.
        while lines.peek().map_or(false, |l| l.trim().is_empty()) {
            lines.next();
        }

        // Sequence number.
        let seq_line = match lines.next() {
            Some(l) => l.trim(),
            None => break,
        };
        let index: u32 = match seq_line.parse() {
            Ok(n) => n,
            Err(_) => continue, // Skip non-numeric lines.
        };

        // Timestamp line: "HH:MM:SS,mmm --> HH:MM:SS,mmm"
        let ts_line = match lines.next() {
            Some(l) => l.trim().to_string(),
            None => break,
        };
        let (start_ms, end_ms) = parse_timestamp_line(&ts_line)
            .with_context(|| format!("Invalid timestamp at cue {index}: {ts_line}"))?;

        // Text lines (until blank line or EOF).
        let mut text_parts = Vec::new();
        while lines.peek().map_or(false, |l| !l.trim().is_empty()) {
            text_parts.push(lines.next().unwrap().trim().to_string());
        }
        let text = text_parts.join("\n");
        if text.is_empty() {
            continue;
        }

        cues.push(SrtCue {
            index,
            start_ms,
            end_ms,
            text,
        });
    }

    Ok(cues)
}

/// Build an SRT string from translated cues.
pub fn build_srt(cues: &[super::TranslatedCue]) -> String {
    let mut srt = String::new();
    for cue in cues {
        srt.push_str(&format!("{}\n", cue.index));
        srt.push_str(&format!(
            "{} --> {}\n",
            format_timestamp(cue.start_ms),
            format_timestamp(cue.end_ms),
        ));
        srt.push_str(&cue.translated_text);
        srt.push_str("\n\n");
    }
    srt
}

/// Build an SRT string from raw cues (before translation).
pub fn build_srt_from_cues(cues: &[SrtCue]) -> String {
    let mut srt = String::new();
    for cue in cues {
        srt.push_str(&format!("{}\n", cue.index));
        srt.push_str(&format!(
            "{} --> {}\n",
            format_timestamp(cue.start_ms),
            format_timestamp(cue.end_ms),
        ));
        srt.push_str(&cue.text);
        srt.push_str("\n\n");
    }
    srt
}

/// Parse "HH:MM:SS,mmm --> HH:MM:SS,mmm" into (start_ms, end_ms).
fn parse_timestamp_line(line: &str) -> Result<(u64, u64)> {
    let parts: Vec<&str> = line.split("-->").collect();
    if parts.len() != 2 {
        anyhow::bail!("Expected '-->' separator in timestamp line");
    }
    let start = parse_time(parts[0].trim())?;
    let end = parse_time(parts[1].trim())?;
    Ok((start, end))
}

/// Parse "HH:MM:SS,mmm" or "HH:MM:SS.mmm" into milliseconds.
fn parse_time(s: &str) -> Result<u64> {
    let s = s.replace(',', ".");
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        anyhow::bail!("Invalid time format: {s} (expected HH:MM:SS,mmm)");
    }
    let h: u64 = parts[0].parse().context("Invalid hours")?;
    let m: u64 = parts[1].parse().context("Invalid minutes")?;
    let sec_parts: Vec<&str> = parts[2].split('.').collect();
    let sec: u64 = sec_parts[0].parse().context("Invalid seconds")?;
    let ms: u64 = if sec_parts.len() > 1 {
        let ms_str = sec_parts[1];
        // Normalize to 3 digits (pad or truncate).
        let normalized = match ms_str.len() {
            0 => "000",
            1 => &format!("{}00", ms_str),
            2 => &format!("{}0", ms_str),
            _ => &ms_str[..3],
        };
        normalized.parse().unwrap_or(0)
    } else {
        0
    };
    Ok(h * 3_600_000 + m * 60_000 + sec * 1_000 + ms)
}

/// Format milliseconds as "HH:MM:SS,mmm".
fn format_timestamp(ms: u64) -> String {
    let h = ms / 3_600_000;
    let m = (ms % 3_600_000) / 60_000;
    let s = (ms % 60_000) / 1_000;
    let millis = ms % 1_000;
    format!("{:02}:{:02}:{:02},{:03}", h, m, s, millis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_srt_basic() {
        let srt = r#"1
00:00:00,000 --> 00:00:05,100
Hello world

2
00:00:05,200 --> 00:00:10,300
How are you?
"#;
        let cues = parse_srt(srt).unwrap();
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].index, 1);
        assert_eq!(cues[0].start_ms, 0);
        assert_eq!(cues[0].end_ms, 5100);
        assert_eq!(cues[0].text, "Hello world");
        assert_eq!(cues[1].index, 2);
        assert_eq!(cues[1].start_ms, 5200);
        assert_eq!(cues[1].end_ms, 10300);
        assert_eq!(cues[1].text, "How are you?");
    }

    #[test]
    fn test_parse_srt_with_bom() {
        let srt = "\u{FEFF}1\n00:00:00,000 --> 00:00:01,000\nBOM test\n\n";
        let cues = parse_srt(srt).unwrap();
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].text, "BOM test");
    }

    #[test]
    fn test_parse_srt_multiline_text() {
        let srt = "1\n00:00:00,000 --> 00:00:05,000\nLine one\nLine two\n\n";
        let cues = parse_srt(srt).unwrap();
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].text, "Line one\nLine two");
    }

    #[test]
    fn test_format_timestamp() {
        assert_eq!(format_timestamp(0), "00:00:00,000");
        assert_eq!(format_timestamp(5100), "00:00:05,100");
        assert_eq!(format_timestamp(3661500), "01:01:01,500");
    }

    #[test]
    fn test_parse_time_period_separator() {
        let ms = parse_time("01:02:03.456").unwrap();
        assert_eq!(ms, 3_723_456);
    }

    #[test]
    fn test_roundtrip() {
        let original = r#"1
00:00:00,000 --> 00:00:05,100
Hello world

2
00:00:05,200 --> 00:00:10,300
How are you?

"#;
        let cues = parse_srt(original).unwrap();
        let rebuilt = build_srt_from_cues(&cues);
        let reparsed = parse_srt(&rebuilt).unwrap();
        assert_eq!(cues.len(), reparsed.len());
        for (a, b) in cues.iter().zip(reparsed.iter()) {
            assert_eq!(a.index, b.index);
            assert_eq!(a.start_ms, b.start_ms);
            assert_eq!(a.end_ms, b.end_ms);
            assert_eq!(a.text, b.text);
        }
    }
}
