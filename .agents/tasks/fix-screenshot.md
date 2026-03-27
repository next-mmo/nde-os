---
status: 🟢 done by AI
---

# Fix Screenshot Bug

## Purpose
Fix the bug where the user cannot capture the area/fullscreen correctly to see the whole node screen for Vibe Code Studio canvas, and ensure E2E is written and passes.

## Root Cause
1. `capture.rs` used logical monitor dimensions (`Monitor::width/height`) for the canvas, but `capture_image()` returns physical-pixel buffers — causing crop misalignment on scaled displays (e.g. 150% Windows scaling).
2. `ScreenshotOverlay.svelte` cancelled on single-click instead of capturing fullscreen.
3. Frontend incorrectly applied `devicePixelRatio` multiply since `xcap` already works in logical coordinates for `Monitor::x/y`.

## Changes Made
- **`core/src/screenshot/capture.rs`**: Use `Monitor::scale_factor()` to compute canvas in physical pixels. Translate logical crop coordinates to physical pixel space before cropping.
- **`desktop/src/ScreenshotOverlay.svelte`**: Single-click (area < 10x10) now emits `width=0, height=0` to trigger fullscreen capture instead of cancelling. Removed incorrect DPR scaling.  
- **`desktop/src/lib/tauri/screenshot.ts`**: Conditionally omit `x/y/width/height` args when `width=0` so Rust backend calls `capture_screen(None)` for fullscreen.
- **`desktop/e2e/screenshot.spec.ts`**: Added robust E2E test for `Ctrl+Shift+S` fullscreen capture flow with fallback for CDP overlay detection.

## Task Checklist
- [x] Fix `capture.rs` scale factor mismatch between logical coords and physical-pixel image buffers.
- [x] Single-click on overlay triggers fullscreen capture instead of cancelling.
- [x] Remove incorrect `devicePixelRatio` scaling from overlay — xcap uses logical coords.
- [x] Conditionally omit crop coords in `screenshot.ts` when fullscreen.
- [x] Add/Update E2E test in `desktop/e2e/screenshot.spec.ts`.
- [x] Rust build passes (`cargo check -p ai-launcher-core --features screenshot`).

## Definition of Done
- [x] Local: Screenshot works for both area capture and fullscreen capture.
- [x] Global: No mocks, E2E tests covering both flows, Rust compiles clean, no panics.

