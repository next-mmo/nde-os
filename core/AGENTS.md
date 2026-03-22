# core/ — Rust Sandbox & Manager

> Inherits all rules from root `AGENTS.md`. This file adds core-specific guidance.

- ZERO deps on `server/` or `desktop/`. This crate is standalone.
- `anyhow::Result` everywhere, no `.unwrap()` or panics.
- Canonicalize all paths — defeat symlink/traversal attacks. Readonly stays readonly.
- Minimize `Arc<Mutex<>>` lock scopes.
- `uv` not `pip`. One venv per workspace (`workspace/.venv`).
- Cross-platform: `PathBuf::join()`, `cfg!(windows)` branching, set `HOME` + `USERPROFILE`.
- Test: `cargo test -p ai-launcher-core -- <test_name>` (only what you touched).
