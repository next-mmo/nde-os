/// Shield Actor System — Apify-compatible browser automation framework.
///
/// Provides an actor runtime that uses Shield Browser (Camoufox/Wayfern)
/// as the browser backend, with dual-runtime support: same actor code
/// runs locally on NDE-OS or deploys to Apify cloud unchanged.
///
/// Architecture mirrors Apify's actor model:
/// - `manifest` — actor.json + input_schema.json parsing & validation
/// - `storage`  — local Dataset (JSONL) + Key-Value store
/// - `runner`   — process lifecycle (spawn → monitor → stop)
/// - `template` — built-in scaffolding templates
pub mod manifest;
pub mod runner;
pub mod storage;
pub mod template;
