# Fix App State Loss on Host OS Switch and Window Minimize

## Status
🟢 done by AI

## Feature
Preserve application state (terminals, browser iframes, downloads, forms) when users switch back and forth between NDE OS and the Host OS (by collapsing the NDE desktop), and when windows are minimized inside NDE.

## Purpose
Currently, two UI behaviors destroy Svelte component state:
1. **Collapsing the Desktop ("Switch to Host OS")**: Applies `display: none` to the entire desktop shell wrapper. In most browsers (like WebKit/Blink), setting `display: none` on an iframe (or deeply nested inside one) causes Safari/Firefox to reload the iframe content. This causes web apps to lose state (e.g. 10% download progress).
2. **Minimizing Windows**: The Svelte loop in `WindowsArea.svelte` uses `visibleWindows()`, which explicitly filters out minimized windows. Svelte completely unmounts them from the DOM. Re-opening a minimized window forces a full remount (Terminal re-invokes PTY shell, Browser reloads iframe).

## Inputs & Outputs
- **Input**: User clicks "Collapse" tab -> Tauri window resizes. User clicks "Expand" tab -> Tauri window restores. User clicks "Minimize" on window -> Window hides.
- **Output**: The application DOM nodes remain preserved but visually invisible, ensuring no state or websocket drop.

## Edge Cases & Security
- We must ensure hidden elements are truly `pointer-events: none` and not focusable, so users cannot accidentally tab into invisible desktop apps while in the floating FAB mode.
- Changing `display: none` to `transform: translate(...)` must be tested to ensure the layout remains correct and doesn't cause overflow issues.
- `WindowsArea` should render `desktop.windows` directly, but we need to bind a reactive CSS class or inline style to hide the window components correctly when `window.minimized` is true.

## Task Checklist
- [x] **Task 1: Desktop Shell Preservation**: In `Desktop.svelte`, replace the `style:display={desktop.collapsed ? 'none' : 'block'}` with an approach (like offset screen coordinates, opacity 0, pointer-events none) that keeps the DOM rendered but invisible.
- [x] **Task 2: Keep Minimized Windows Mounted**: In `WindowsArea.svelte`, change `visibleWindows()` to `desktop.windows`. Update `Window.svelte` to apply `opacity: 0; pointer-events: none; visibility: hidden; transform: scale(0.9); transition` when `window.minimized` is true, avoiding complete Svelte unmounting.
- [x] **Task 3: Prevent Desktop Drag/Focus while Collapsed**: Ensure the wrapper in `Desktop.svelte` does not capture keyboard/mouse events when collapsed. Provide `aria-hidden` attribute syncing.

## Definition of Done (DoD)
- [x] **Local DoD**: When a terminal is opened and a command (like `sleep 10`) is running, hitting "Minimize" and then "Expand" preserves the running process. When collapsing the whole NDE launcher to the side tab and expanding it back, a loaded Browser iframe does not refresh.
- [x] **Global DoD**: Code has no hacks, panics, or TODOs.
- [x] **E2E Screenshots**: Verify and screenshot the visual transitions to ensure the dock or desktop isn't broken.
