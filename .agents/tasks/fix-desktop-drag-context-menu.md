---
status: 🟢 done by AI
---

# Fix Desktop Drag and Context Menus

## Purpose
Ensure the home desktop drag, move interactions, and context menus behave naturally like a real OS. Implement stable draggable interactions that don't accidentally open apps, add marquee selection, and offer full-featured context menus. Guarantee these fixes are verified via Playwright E2E tests capturing actual visual screenshots.

## Feature
- **App Drag/Move Fix:** Suppress `onclick` and `ondblclick` events on desktop icons if a drag movement exceeded the threshold, preventing accidental "move event open app" issues.
- **Desktop Drag Selection:** Implement a marquee (selection box) when clicking and dragging on the empty desktop.
- **Full Context Menus:** Expand both Desktop and Icon right-click context menus with detailed actions (e.g., "New Folder", "Pin to Dock", "Change Wallpaper", etc).
- **E2E Screenshots:** E2E tests explicitly verify drag/move events and context menus, capturing real-user result screenshots as proof of correct UI behavior.

## Task Checklist
- [x] **Fix Move/Open Conflict:** Updated `DesktopIcons.svelte` — `wasDragRecent()` guard based on `performance.now()` timestamps suppresses click/dblclick within 250ms of a drag-end.
- [x] **Expand Desktop Context Menu:** Added macOS-style items in `Desktop.svelte`: New Folder, Sort By Name, Clean Up, Use Stacks, Show View Options, Change Wallpaper, Toggle Dark Mode, Auto-Hide Dock, Open Launchpad, Spotlight Search.
- [x] **Expand Icon Context Menu:** Added Pin to Dock, Copy, Get Info items for static app icons in `DesktopIcons.svelte`.
- [x] **Desktop Selection Box:** Added rubber-band marquee selection in `DesktopIcons.svelte` — clicking and dragging on empty desktop draws a translucent selection rectangle and selects enclosed icons.
- [x] **Write E2E Test:** Created `desktop/e2e/desktop-interaction.spec.ts` with 5 tests covering icons, context menus, drag-without-open, rich menu items, and window dragging. All capture `page.screenshot()` to `e2e-results/`.

## Definition of Done
- [x] Local: Moving icons does not trigger app open. Context menus feature multiple native-style options. Marquee selection works.
- [x] Global: E2E tests written and capture screenshots. No mocks. data-testid attributes added for testability.
