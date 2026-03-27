---
status: 🟡 yolo mode
---

# Terminal App

## Purpose
Currently, the standalone Desktop App for Terminal is a placeholder stating "Terminal is reserved in the shell, but the actual app logic has not been added yet." This ticket replaces the placeholder with a true native-like Terminal app that uses `xterm.js` and the Rust `portable-pty` backend via Tauri.

## Feature
- Add a new full-fledged Svelte component for the Terminal app (`Desktop.svelte` / `AppNexus.svelte` integration).
- Use `xterm.js` combined with Tauri's `spawn_pty`, `write_pty` commands and sidecar events to handle IO.
- Re-use the proven logic from `TerminalPanel.svelte` (used in Vibe Code Studio).
- Provide a clean macOS-like interface.
- Add an End-to-End (E2E) Test using Playwright and capture a screenshot. 

## Inputs & Outputs
- **Input**: User launches the terminal app from the macOS-like desktop dock or app catalog.
- **Output**: Terminal opens rendering a local shell interacting with the Rust sandbox PTY backend. User can run commands seamlessly as they would natively.

## Edge Cases & Security
- **Sandbox Compliance**: The PTY backend (`portable-pty`) must be instantiated properly within the Tauri `invoke` call in existing backend logic.
- **Resize handling**: `xterm.js` should resize dynamically with `FitAddon` and window `ResizeObserver`.
- **Memory Leaks**: Subprocess unlisten cleanup in `onDestroy`.

## Task Checklist
- [ ] **Create Interface**: Create `desktop/src/components/apps/Terminal/Terminal.svelte`. Port `xterm.js` and `portable-pty` logic from `VibeCodeStudio/ide/TerminalPanel.svelte` into a standalone app window component.
- [ ] **App Registration**: Update `desktop/src/components/apps/AppNexus.svelte` to route `window.app_id === "terminal"` to the new `Terminal/Terminal.svelte` component.
- [ ] **E2E Test**: Update/Add `desktop/e2e/terminal.spec.ts` using Playwright CDP to launch the Terminal, assert terminal text is visible via `page.getByRole('application')` or similar, and take a screenshot `terminal-mac.png` to satisfy DoD.
- [ ] **Review**: Self-review. Run E2E tests inside `WebView2`. Fix any bugs.
- [ ] **Global DoD**: Code meets system-wide definitions of done (Rust safety, cross-platform paths, tests pass). Check off items and progress `🟢 done by AI`.

## Definition of Done
- [ ] Local: The user can open "Terminal", type a native command like `echo Hello`, and see the output in an `xterm.js`-powered window container without mocks.
- [ ] Global: No panics, no raw CSS (shadcn-svelte + Tailwind), cross-platform, E2E test passes with screenshot saved to `test-results/terminal-app/screenshot.png`.
