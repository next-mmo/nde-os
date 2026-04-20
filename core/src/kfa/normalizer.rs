//! Khmer text normalization.
//!
//! Ported from Python `khmernormalizer` package.
//! Provides Unicode normalization, character replacements,
//! quote fixing, and Khmer-specific syllable reordering.

use once_cell::sync::Lazy;
use fancy_regex::Regex as FancyRegex;
use regex::Regex;
use unicode_normalization::UnicodeNormalization;

// ── Mappings (from khmernormalizer/mappings.rs) ─────────────────────────────

static ZWSP_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[\u{200B}\u{FEFF}\u{200C}\u{200D}]").unwrap());

static SINGLE_QUOTE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new("[\u{2018}\u{2019}\u{201A}\u{201B}\u{2032}\u{0060}\u{00B4}\u{AB}\u{BB}]").unwrap());

static DOUBLE_QUOTE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new("[\u{201C}\u{201D}\u{201E}\u{201F}\u{2033}\u{00AB}\u{00BB}]").unwrap());

static MULTIPLE_PUNCT_REGEX: Lazy<FancyRegex> =
    Lazy::new(|| FancyRegex::new(r"([!?,;:។])\1+").unwrap());

static TRAILING_VOWEL_RE: Lazy<FancyRegex> =
    Lazy::new(|| FancyRegex::new(r"([\u{17B6}-\u{17C5}])\1+").unwrap());

static DUPLICATE_COENG_RE: Lazy<FancyRegex> =
    Lazy::new(|| FancyRegex::new(r"(\u{17D2}[\u{1780}-\u{17B3}])\1+").unwrap());

static WHITESPACES_HANDLER_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[^\S\n]+").unwrap());

static ELLIPSIS_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\.{2,}").unwrap());

static SPACE_BEFORE_PUNCT_RE: Lazy<FancyRegex> =
    Lazy::new(|| FancyRegex::new(r"\s+([!?,;:។])").unwrap());

static SPACE_BEFORE_REPEAT_RE: Lazy<FancyRegex> =
    Lazy::new(|| FancyRegex::new(r"\s+ៗ").unwrap());

static UNICODE_REPLACEMENTS_REGEX: Lazy<FancyRegex> = Lazy::new(|| {
    let patterns: Vec<String> = UNICODE_REPLACEMENTS
        .iter()
        .map(|(k, _)| fancy_regex::escape(k).into_owned())
        .collect();
    FancyRegex::new(&patterns.join("|")).unwrap()
});

static UNICODE_REPLACEMENTS: &[(&str, &str)] = &[
    ("\u{17A3}", "\u{17A2}"),
    ("\u{17A4}", "\u{17A2}\u{17B6}"),
    ("\u{17B4}", ""),
    ("\u{17B5}", ""),
    ("\u{17D8}", "\u{17D4}\u{179B}\u{17D4}"),
    ("\u{17DD}", "\u{17D1}"),
];

fn unicode_replacement(s: &str) -> Option<&'static str> {
    UNICODE_REPLACEMENTS
        .iter()
        .find(|(k, _)| *k == s)
        .map(|(_, v)| *v)
}

// ASCII char replacements (accented Latin → plain ASCII)
fn char_replacement(c: char) -> Option<char> {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => Some('a'),
        'è' | 'é' | 'ê' | 'ë' => Some('e'),
        'ì' | 'í' | 'î' | 'ï' => Some('i'),
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' => Some('o'),
        'ù' | 'ú' | 'û' | 'ü' => Some('u'),
        'ý' | 'ÿ' => Some('y'),
        'ñ' => Some('n'),
        'ç' => Some('c'),
        'ß' => Some('s'),
        _ => None,
    }
}

// ── Syllable normalizer (from khmernormalizer/khnormal.rs) ──────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum Cat {
    Other = 0,
    Base = 1,
    Robat = 2,
    Coeng = 3,
    ZFCoeng = 4,
    Shift = 5,
    Z = 6,
    VPre = 7,
    VB = 8,
    VA = 9,
    VPost = 10,
    MS = 11,
    MF = 12,
}

static CATEGORIES: &[Cat] = &[
    Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base,
    Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base,
    Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base,
    Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base,
    Cat::Base, Cat::Base, Cat::Base,
    Cat::Other, Cat::Other,
    Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base,
    Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base, Cat::Base,
    Cat::Other, Cat::Other,
    Cat::VPost,
    Cat::VA, Cat::VA, Cat::VA, Cat::VA,
    Cat::VB, Cat::VB, Cat::VB,
    Cat::VPre, Cat::VPre, Cat::VPre, Cat::VPre, Cat::VPre, Cat::VPre, Cat::VPre, Cat::VPre,
    Cat::MS,
    Cat::MF, Cat::MF,
    Cat::Shift, Cat::Shift,
    Cat::MS,
    Cat::Robat,
    Cat::MS, Cat::MS, Cat::MS, Cat::MS, Cat::MS,
    Cat::Coeng,
    Cat::MS,
    Cat::Other, Cat::Other, Cat::Other, Cat::Other, Cat::Other,
    Cat::Other, Cat::Other, Cat::Other, Cat::Other,
    Cat::MS,
];

fn charcat(c: char) -> Cat {
    let o = c as u32;
    if (0x1780..=0x17DD).contains(&o) {
        CATEGORIES[(o - 0x1780) as usize]
    } else if o == 0x200C {
        Cat::Z
    } else if o == 0x200D {
        Cat::ZFCoeng
    } else {
        Cat::Other
    }
}

static RE_MULTI_INVIS: Lazy<FancyRegex> = Lazy::new(|| {
    FancyRegex::new(r"([\u{200C}\u{200D}]\u{17D2}?|\u{17D2}\u{200D})[\u{17D2}\u{200C}\u{200D}]+").unwrap()
});
static RE_COMPOUND_EI: Lazy<FancyRegex> = Lazy::new(|| {
    FancyRegex::new(r"\u{17C1}([\u{17BB}-\u{17BD}]?)\u{17B8}").unwrap()
});
static RE_COMPOUND_EA: Lazy<FancyRegex> = Lazy::new(|| {
    FancyRegex::new(r"\u{17C1}([\u{17BB}-\u{17BD}]?)\u{17B6}").unwrap()
});
static RE_SWAP_BE_BB: Lazy<FancyRegex> = Lazy::new(|| {
    FancyRegex::new(r"(\u{17BE})(\u{17BB})").unwrap()
});
static RE_STRONG_BB: Lazy<FancyRegex> = Lazy::new(|| {
    FancyRegex::new(
        r"([\u{1780}-\u{1783}\u{1785}-\u{1788}\u{178A}-\u{178D}\u{178F}-\u{1792}\u{1795}-\u{1797}\u{179E}-\u{17A0}\u{17A2}](?:\u{17CC})?(?:\u{17D2}[\u{1780}-\u{17B3}](?:\u{17D2}[\u{1780}-\u{17B3}])?)?(?:\u{17D2}\u{200D}[\u{1780}-\u{1799}\u{179B}-\u{17A2}\u{17A5}-\u{17B3}])?[\u{17C1}-\u{17C5}]?)\u{17BB}(?=[\u{17B7}-\u{17BA}\u{17BE}\u{17BF}\u{17DD}]|\u{17B6}\u{17C6}|\u{17D0})"
    ).unwrap()
});
static RE_NSTRONG_BB: Lazy<FancyRegex> = Lazy::new(|| {
    FancyRegex::new(
        r"([\u{1784}\u{1780}\u{178E}\u{1793}\u{1794}\u{1798}-\u{179D}\u{17A1}\u{17A3}-\u{17B3}](?:\u{17CC})?(?:\u{17D2}[\u{1780}-\u{17B3}](?:\u{17D2}[\u{1780}-\u{17B3}])?)?(?:\u{17D2}\u{200D}[\u{1780}-\u{1799}\u{179B}-\u{17A2}\u{17A5}-\u{17B3}])?[\u{17C1}-\u{17C5}]?)\u{17BB}(?=[\u{17B7}-\u{17BA}\u{17BE}\u{17BF}\u{17DD}]|\u{17B6}\u{17C6}|\u{17D0})"
    ).unwrap()
});
static RE_COENG_RO_SECOND: Lazy<FancyRegex> = Lazy::new(|| {
    FancyRegex::new(r"(\u{17D2}\u{179A})(\u{17D2}[\u{1780}-\u{17B3}])").unwrap()
});
static RE_COENG_DA_TO_TA: Lazy<FancyRegex> = Lazy::new(|| {
    FancyRegex::new(r"(\u{17D2})\u{178A}").unwrap()
});

fn khmer_normalize(txt: &str) -> String {
    let chars: Vec<char> = txt.chars().collect();
    let mut charcats: Vec<Cat> = chars.iter().map(|&c| charcat(c)).collect();

    for i in 1..charcats.len() {
        if charcats[i - 1] == Cat::Coeng
            && (charcats[i] == Cat::Base || charcats[i] == Cat::ZFCoeng)
        {
            charcats[i] = Cat::Coeng;
        }
    }

    let mut i = 0;
    let mut res = String::with_capacity(txt.len());

    while i < charcats.len() {
        let c = charcats[i];
        if c != Cat::Base {
            res.push(chars[i]);
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < charcats.len() && (charcats[j] as u8) > (Cat::Base as u8) {
            j += 1;
        }
        let mut indices: Vec<usize> = (i..j).collect();
        indices.sort_by(|&a, &b| (charcats[a] as u8).cmp(&(charcats[b] as u8)).then(a.cmp(&b)));
        let mut replaces: String = indices.iter().map(|&n| chars[n]).collect();

        replaces = RE_MULTI_INVIS.replace_all(&replaces, "$1").into_owned();
        replaces = RE_COMPOUND_EI.replace_all(&replaces, "\u{17BE}$1").into_owned();
        replaces = RE_COMPOUND_EA.replace_all(&replaces, "\u{17C4}$1").into_owned();
        replaces = RE_SWAP_BE_BB.replace_all(&replaces, "$2$1").into_owned();
        replaces = RE_STRONG_BB.replace_all(&replaces, "$1\u{17CA}").into_owned();
        replaces = RE_NSTRONG_BB.replace_all(&replaces, "$1\u{17C9}").into_owned();
        replaces = RE_COENG_RO_SECOND.replace_all(&replaces, "$2$1").into_owned();
        replaces = RE_COENG_DA_TO_TA.replace_all(&replaces, "$1\u{178F}").into_owned();

        res.push_str(&replaces);
        i = j;
    }
    res
}

// ── Public normalize function ────────────────────────────────────────────────

/// Normalize Khmer text. Applies ZWSP removal, quote fixing, NFKC, char
/// replacements, whitespace normalization, and syllable-level reordering.
pub fn normalize(text: &str, remove_zwsp: bool) -> String {
    let mut text = text.to_string();

    if remove_zwsp {
        text = ZWSP_RE.replace_all(&text, "").to_string();
    }
    text = SINGLE_QUOTE_REGEX.replace_all(&text, "'").to_string();
    text = DOUBLE_QUOTE_REGEX.replace_all(&text, "\"").to_string();
    text = MULTIPLE_PUNCT_REGEX.replace_all(&text, "$1").into_owned();
    text = TRAILING_VOWEL_RE.replace_all(&text, "$1").into_owned();
    text = DUPLICATE_COENG_RE.replace_all(&text, "$1").into_owned();
    text = text.nfkc().collect::<String>();
    text = text
        .chars()
        .map(|c| {
            if let Some(r) = char_replacement(c) {
                r.to_string()
            } else {
                c.to_string()
            }
        })
        .collect();
    text = UNICODE_REPLACEMENTS_REGEX
        .replace_all(&text, |caps: &fancy_regex::Captures| {
            let matched = caps.get(0).unwrap().as_str();
            unicode_replacement(matched).unwrap_or(matched).to_string()
        })
        .into_owned();
    text = WHITESPACES_HANDLER_REGEX.replace_all(&text, " ").into_owned();
    text = ELLIPSIS_RE.replace_all(&text, "\u{2026}").into_owned();
    text = SPACE_BEFORE_PUNCT_RE.replace_all(&text, "$1").into_owned();
    text = SPACE_BEFORE_REPEAT_RE.replace_all(&text, "ៗ").into_owned();

    let text = text.trim().to_string();
    khmer_normalize(&text)
}
