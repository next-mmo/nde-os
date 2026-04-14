//! FreeCut — Native video editor engine for NDE-OS.
//!
//! All media processing (decode, encode, compose, analyze) runs in Rust.
//! The Svelte frontend receives rendered frames and status via Tauri events.

pub mod dubbing;
pub mod ffmpeg_bootstrap;
pub mod media_probe;
pub mod movie_dub;
pub mod project;
pub mod render_engine;
pub mod storage;
pub mod vision;
