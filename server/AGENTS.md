# server/ — Rust HTTP API

> Inherits all rules from root `AGENTS.md`. This file adds server-specific guidance.

- `anyhow::Result` everywhere, no `.unwrap()` or panics.
- Cross-platform: `PathBuf::join()`, `cfg!(windows)` branching, set `HOME` + `USERPROFILE`.
- Minimize `Arc<Mutex<>>` lock scopes.
- No UI code here — this is backend only.
- Test: `cargo test -p ai-launcher-server -- <test_name>` (only what you touched).
- Verify: `curl -s http://localhost:8080/api/sandbox/test/verify`
