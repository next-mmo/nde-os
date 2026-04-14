//! Re-export shim — FFmpeg bootstrap has moved to [`crate::media::ffmpeg`].
//!
//! All new code should import from `crate::media::ffmpeg` directly.
//! This file exists only so existing `freecut::ffmpeg_bootstrap` references
//! inside this module continue to compile without a mass rename.

pub use crate::media::ffmpeg::*;
