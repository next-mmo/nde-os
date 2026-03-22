# desktop/ — Tauri + Svelte 5

> Inherits all rules from root `AGENTS.md`. This file adds desktop-specific guidance.

- shadcn-svelte + Tailwind only. No custom `<style>` blocks or raw CSS.
- macOS Ventura aesthetic: blur effects, traffic-light controls (`@neodrag/svelte`), dock animations.
- Tauri backend (Rust): `anyhow::Result`, no panics, `PathBuf::join()`.
- Cross-platform: `cfg!(windows)` branching, set `HOME` + `USERPROFILE`.
- Test: `npx playwright test <changed-spec>` (only what you touched).
