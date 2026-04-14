//! Core media utilities — shared across all NDE-OS apps.
//!
//! Anything that touches FFmpeg, FFprobe, or generic media handling
//! belongs here rather than inside a specific app module.

pub mod ffmpeg;
