//! Khmer text analysis for dubbing duration estimation.
//!
//! Khmer script (អក្សរខ្មែរ) has unique properties:
//! - No spaces between words (spaces only between clauses)
//! - Consonant clusters use Coeng (្) U+17D2 to stack consonants
//! - Each syllable = consonant onset + optional vowels/signs
//!
//! We count syllables by identifying consonant onsets that are NOT
//! part of a Coeng cluster (subscript consonant = same syllable).

/// Average Khmer syllable duration at natural speech rate (ms).
const KHMER_MS_PER_SYLLABLE: f32 = 150.0;
/// Minimum credible syllable duration (very fast speech).
const KHMER_MS_PER_SYLLABLE_FAST: f32 = 120.0;
/// Maximum comfortable syllable duration (slow/clear speech).
const KHMER_MS_PER_SYLLABLE_SLOW: f32 = 190.0;

/// Unicode ranges for Khmer script analysis.
const KHMER_CONSONANT_START: char = '\u{1780}'; // ក
const KHMER_CONSONANT_END: char = '\u{17A2}';   // អ
const KHMER_INDEP_VOWEL_START: char = '\u{17A3}';
const KHMER_INDEP_VOWEL_END: char = '\u{17B3}';
const KHMER_DEP_VOWEL_START: char = '\u{17B6}';
const KHMER_DEP_VOWEL_END: char = '\u{17D1}';
const KHMER_COENG: char = '\u{17D2}'; // ្ (subscript marker)

/// Estimate syllable count in Khmer text.
///
/// Algorithm:
/// 1. Walk through characters.
/// 2. Each consonant NOT preceded by Coeng = new syllable onset.
/// 3. Independent vowels at word-start = syllable onset.
/// 4. Coeng + consonant = same syllable (cluster, not new onset).
pub fn estimate_syllables(text: &str) -> u32 {
    if text.is_empty() {
        return 0;
    }

    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut count: u32 = 0;
    let mut has_khmer = false;
    let mut i = 0;

    while i < len {
        let ch = chars[i];

        if is_khmer_consonant(ch) {
            has_khmer = true;

            // Check if this consonant is a subscript (preceded by Coeng).
            let is_subscript = i > 0 && chars[i - 1] == KHMER_COENG;

            if !is_subscript {
                count += 1;

                // Skip the rest of this syllable:
                // consonant [Coeng + consonant]* [vowels/signs]*
                i += 1;
                while i < len {
                    if chars[i] == KHMER_COENG && i + 1 < len && is_khmer_consonant(chars[i + 1]) {
                        i += 2;
                    } else if is_khmer_dependent(chars[i]) {
                        i += 1;
                    } else {
                        break;
                    }
                }
                continue;
            }
        } else if is_khmer_independent_vowel(ch) {
            has_khmer = true;
            count += 1;
        }

        i += 1;
    }

    // Fallback for non-Khmer text (Latin/Chinese).
    if !has_khmer && count == 0 {
        count = estimate_non_khmer_syllables(text);
    }

    count.max(1)
}

/// Estimate syllables for non-Khmer text (English, Chinese, etc.).
fn estimate_non_khmer_syllables(text: &str) -> u32 {
    let mut count = 0u32;

    for word in text.split_whitespace() {
        // Chinese characters: ~1 syllable each.
        let cjk_chars = word.chars().filter(|c| is_cjk(*c)).count();
        if cjk_chars > 0 {
            count += cjk_chars as u32;
            continue;
        }

        // English: rough heuristic based on vowel groups.
        let word_lower = word.to_lowercase();
        let mut in_vowel = false;
        let mut word_syls = 0u32;

        for ch in word_lower.chars() {
            if "aeiouy".contains(ch) {
                if !in_vowel {
                    word_syls += 1;
                    in_vowel = true;
                }
            } else if ch.is_alphabetic() {
                in_vowel = false;
            }
        }

        // Silent 'e' at end.
        if word_lower.ends_with('e') && word_syls > 1 {
            word_syls -= 1;
        }

        count += word_syls.max(1);
    }

    count.max(1)
}

/// Convert syllable count → estimated duration (ms) at natural speed.
pub fn syllables_to_ms(syllables: u32) -> u64 {
    (syllables as f32 * KHMER_MS_PER_SYLLABLE) as u64
}

/// Convert duration → max syllable count at natural speed.
pub fn ms_to_max_syllables(duration_ms: u64) -> u32 {
    (duration_ms as f32 / KHMER_MS_PER_SYLLABLE).floor() as u32
}

/// Estimate comfortable duration range for given syllable count.
pub fn syllable_duration_range(syllables: u32) -> (u64, u64) {
    let min_ms = (syllables as f32 * KHMER_MS_PER_SYLLABLE_FAST) as u64;
    let max_ms = (syllables as f32 * KHMER_MS_PER_SYLLABLE_SLOW) as u64;
    (min_ms, max_ms)
}

// ── Unicode helpers ──

fn is_khmer_consonant(ch: char) -> bool {
    ch >= KHMER_CONSONANT_START && ch <= KHMER_CONSONANT_END
}

fn is_khmer_independent_vowel(ch: char) -> bool {
    ch >= KHMER_INDEP_VOWEL_START && ch <= KHMER_INDEP_VOWEL_END
}

fn is_khmer_dependent(ch: char) -> bool {
    ch >= KHMER_DEP_VOWEL_START && ch <= KHMER_DEP_VOWEL_END
}

fn is_cjk(ch: char) -> bool {
    let c = ch as u32;
    (0x4E00..=0x9FFF).contains(&c)
        || (0x3400..=0x4DBF).contains(&c)
        || (0xF900..=0xFAFF).contains(&c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_khmer_hello() {
        // "សួស្តី" (suostei) — should be ~2 syllables
        let text = "សួស្តី";
        let count = estimate_syllables(text);
        assert!(count >= 1 && count <= 3, "សួស្តី got {} syllables", count);
    }

    #[test]
    fn test_khmer_sentence() {
        // "ខ្ញុំស្រលាញ់អ្នក" (I love you) — ~4-5 syllables
        let text = "ខ្ញុំស្រលាញ់អ្នក";
        let count = estimate_syllables(text);
        assert!(count >= 3 && count <= 6, "got {} syllables", count);
    }

    #[test]
    fn test_coeng_not_double_counted() {
        // "ក្រ" = ក + ្ + រ = ONE syllable (kr cluster), not two
        let text = "ក្រ";
        let count = estimate_syllables(text);
        assert_eq!(count, 1, "ក្រ should be 1 syllable, got {count}");
    }

    #[test]
    fn test_english_fallback() {
        let text = "I will be right back";
        let count = estimate_syllables(text);
        assert!(count >= 4 && count <= 7, "got {count}");
    }

    #[test]
    fn test_chinese_fallback() {
        // 你好世界 = 4 characters = 4 syllables
        let text = "你好世界";
        let count = estimate_syllables(text);
        assert_eq!(count, 4, "got {count}");
    }

    #[test]
    fn test_empty() {
        assert_eq!(estimate_syllables(""), 0);
    }

    #[test]
    fn test_duration_calc() {
        let ms = syllables_to_ms(6);
        assert_eq!(ms, 900); // 6 * 150
    }

    #[test]
    fn test_max_syllables() {
        let max = ms_to_max_syllables(2000);
        assert_eq!(max, 13); // 2000 / 150 = 13.3 → 13
    }

    #[test]
    fn test_duration_range() {
        let (min, max) = syllable_duration_range(10);
        assert_eq!(min, 1200); // 10 * 120
        assert_eq!(max, 1900); // 10 * 190
    }
}
