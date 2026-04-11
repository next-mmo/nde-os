/// NDE Shield Browser — Anti-detect browser subsystem
///
/// Provides C++/Rust-level fingerprint spoofing via Wayfern (Chromium)
/// and Camoufox (Firefox) engines, inspired by DonutBrowser architecture.
/// AGPL-3.0 licensed component.
pub mod browser;
pub mod cdp;
pub mod emulator;
pub mod engine;
pub mod launcher;
pub mod ldplayer;
pub mod ldplayer_db;
pub mod profile;
