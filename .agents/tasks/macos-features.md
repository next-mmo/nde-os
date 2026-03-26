---
status: 🟡 yolo mode
---

# Feature: Mac OS-Like System Enhancements

## Purpose
Evolve NDE OS to feel more like a full daily driver by implementing core Mac OS features that are currently missing, including system settings organization, shut down, restart, and sleep/lock screen functionality.

## Context & Inputs
- The current `MenuBar.svelte` has an "Apple menu" (`apple_menu`) where items like "Sleep", "Restart", and "Shut Down" are disabled.
- The `Settings.svelte` app currently only shows system diagnostics ("Server and runtime") without configuration sections.
- `desktop.svelte.ts` manages state, including theme, dock settings, and `is_locked` (to be added).

## Tasks Checklist
- [x] 1. **Add Locking State**: Add an `is_locked` boolean to `desktop.svelte.ts` state. Update `desktop.svelte.ts` to persist locked state optionally, but for now default to false on start.
- [x] 2. **Implement Lock Screen Component**: Create `LockScreen.svelte` inside `desktop/src/components/Desktop/`. Make it an absolute overlay that requires a simple click to unlock, bringing `desktop.is_locked` to `false`. Add it to `Desktop.svelte` with absolute positioning over the whole app (z-index 99999).
- [x] 3. **Implement Shut Down, Restart, and Sleep**: Update `desktop/src/components/TopBar/MenuBar.svelte` to enable these options. "Shut Down" will call Tauri's `getCurrentWindow().close()`. "Restart" will call `window.location.reload()`. "Sleep" will set `desktop.is_locked = true` and `desktop.launchpad_open = false`.
- [ ] 4. **Refactor Settings App Layout (macOS style)**: Overhaul `desktop/src/components/apps/Settings/Settings.svelte` to use a split-pane layout (sidebar on left with a search bar, content pane on the right).
- [ ] 5. **System / General Pane**: Move the current "Server and runtime" diagnostics into a "System" pane that acts as the default view.
- [ ] 6. **Appearance Pane**: Create a pane to toggle between Light/Dark mode and potentially choose an accent color or wallpaper setting.
- [ ] 7. **Desktop & Dock Pane**: Create a pane to configure `Dock Auto-Hide` and reset Desktop Icon positions.
- [ ] 8. **Control Center Pane**: Create a pane for simulated Control Center settings (e.g., toggling certain top-bar icons if supported).
- [ ] 9. **Shadcn-Svelte Full Refactor**: Refactor the newly created layout and all existing modified OS files (`Settings.svelte`, `LockScreen.svelte`, `MenuBar.svelte`) to strictly utilize `shadcn-svelte` structural components (`Tabs`, `Switch`, `Button`, `Card`, etc.) and basic Tailwind classes. Completely eliminate custom `<style>` and raw CSS inside these components to adhere to `AGENTS.md` guidelines.
- [ ] 10. **Apple-Style Visual Polish**: Perform a final UI refactoring pass specifically targeting Mac OS Ventura design language. Ensure the use of strong translucent backgrounds (`backdrop-blur-[25px]', 'bg-white/35', etc.), traffic-light controls styling (where applicable), seamless scrollbars, specific font weights/sizes (Inter/San Francisco system stacks), and pixel-perfect padding to maximize the "Apple-like" feel in all components touched.

## Definition of Done
- Local DoD: Users can shut down, restart, or sleep from the top bar Apple menu. Settings is organized into sections matching OS preference tabs, and appearance/dock settings can be adjusted.
- Global DoD: No mocks, Playwright / Rust tests pass (if modified), cross-platform path integrity maintained, UI follows macOS aesthetic (shadcn-svelte + Tailwind), everything in Svelte 5 style.

## 💡 Suggestions for Additional OS Features
To make this OS feel even more robust and capable for daily use, here are some features we could add to this plan (or tackle in future tickets):

1. **Spotlight Search (Cmd+Space / Ctrl+Space)**: A quick global search bar that floats in the center of the screen to launch apps, find files, or run terminal commands instantly.
2. **Control Center Dropdown**: A macOS-styled quick settings panel from the top right `TopBar` that provides fast toggles for Wi-Fi, Dark Mode, Volume, and Focus mode without opening full Settings.
3. **Wallpaper Engine / Picker**: Allow the user to pick different wallpapers or video backgrounds dynamically from the `Appearance` settings menu, saving the preference locally.
4. **Notification Center**: A slide-out sidebar for handling toast notifications (e.g., app updates, agent completion alerts) + maybe some quick glance widgets like a calendar.
5. **System Tray / App Indicators**: Moving the `running_apps` info natively into the top bar near the clock, giving the user a global view of agent statuses.
6. **"Trash" / Basic Files on Desktop**: Introducing a desktop folder where users can visually drop items and empty trash, simulating a true desktop experience.
