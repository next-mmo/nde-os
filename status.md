# NDE-OS Feature Status Tracker

> **Last Updated:** 2026-04-06  
> **Legend:** ✅ Working | ⚠️ Partial | ❌ Broken | 🔲 Not Tested  
> **Severity:** 🔴 Critical | 🟠 Major | 🟡 Minor | 🔵 Cosmetic

---

## 1. Desktop Shell

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 1.1 | Desktop wallpaper renders | 🔲 | | |
| 1.2 | Desktop icons show & open apps | 🔲 | | |
| 1.3 | Desktop right-click context menu | 🔲 | | |
| 1.4 | Desktop icon drag & rearrange | 🔲 | | |
| 1.5 | "Open With" menu on files | 🔲 | | |
| 1.6 | Lock Screen display / unlock | 🔲 | | |
| 1.7 | Notification Center | 🔲 | | |

## 2. Window Manager

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 2.1 | Window open / close | 🔲 | | |
| 2.2 | Window minimize | 🔲 | | |
| 2.3 | Window maximize / restore | 🔲 | | |
| 2.4 | Window drag (move) | 🔲 | | |
| 2.5 | Window resize | 🔲 | | |
| 2.6 | Window focus / z-order stacking | 🔲 | | |
| 2.7 | Traffic-light controls (close/min/max) | 🔲 | | |
| 2.8 | Multiple windows at once | 🔲 | | |

## 3. Dock

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 3.1 | Dock renders with app icons | 🔲 | | |
| 3.2 | Click dock icon → opens app | 🔲 | | |
| 3.3 | Dock hover magnification animation | 🔲 | | |
| 3.4 | Dock shows running-app dot indicator | 🔲 | | |
| 3.5 | Dock separators between groups | 🔲 | | |

## 4. Top Bar (Menu Bar)

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 4.1 | Top bar renders (clock, metrics) | 🔲 | | |
| 4.2 | Top bar time/date display | 🔲 | | |
| 4.3 | System metrics (CPU/RAM/GPU) | 🔲 | | |
| 4.4 | Action Center toggle | 🔲 | | |
| 4.5 | Menu bar shows active app name | 🔲 | | |

## 5. Launchpad

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 5.1 | Launchpad opens (grid of all apps) | 🔲 | | |
| 5.2 | Click app icon → opens app window | 🔲 | | |
| 5.3 | Launchpad search/filter | 🔲 | | |
| 5.4 | Launchpad close on click outside | 🔲 | | |

## 6. Spotlight Search

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 6.1 | Spotlight opens (keyboard shortcut / UI) | 🔲 | | |
| 6.2 | Search apps by name | 🔲 | | |
| 6.3 | Select result → opens app | 🔲 | | |

---

## 7. AI Launcher (App Catalog)

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 7.1 | Catalog loads & shows app cards | 🔲 | | |
| 7.2 | Install an app from catalog | 🔲 | | |
| 7.3 | Uninstall an app | 🔲 | | |
| 7.4 | Launch (start) an installed app | 🔲 | | |
| 7.5 | Stop a running app | 🔲 | | |
| 7.6 | App status indicator (running/stopped) | 🔲 | | |
| 7.7 | Install progress / streaming output | 🔲 | | |

### Catalog Apps

| # | App | Install | Launch | Stop | Bug Description |
|---|-----|---------|--------|------|-----------------|
| 7.a | ComfyUI | 🔲 | 🔲 | 🔲 | |
| 7.b | Fooocus | 🔲 | 🔲 | 🔲 | |
| 7.c | Kohya-SS | 🔲 | 🔲 | 🔲 | |
| 7.d | Ollama | 🔲 | 🔲 | 🔲 | |
| 7.e | Open WebUI | 🔲 | 🔲 | 🔲 | |
| 7.f | Stable Diffusion | 🔲 | 🔲 | 🔲 | |
| 7.g | Whisper Web | 🔲 | 🔲 | 🔲 | |
| 7.h | Sample Gradio | 🔲 | 🔲 | 🔲 | |
| 7.i | Sample Node | 🔲 | 🔲 | 🔲 | |

## 8. NDE Chat (LLM Chat)

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 8.1 | Chat window opens | 🔲 | | |
| 8.2 | Send message & get response | 🔲 | | |
| 8.3 | Streaming response display | 🔲 | | |
| 8.4 | Conversation history | 🔲 | | |
| 8.5 | Model selection / switching | 🔲 | | |

## 9. LLM Providers / Model Settings

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 9.1 | Model settings window opens | 🔲 | | |
| 9.2 | Add / configure providers (API keys) | 🔲 | | |
| 9.3 | List available models | 🔲 | | |
| 9.4 | Test provider connection | 🔲 | | |

## 10. Terminal

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 10.1 | Terminal window opens | 🔲 | | |
| 10.2 | PTY session starts (interactive shell) | 🔲 | | |
| 10.3 | Execute commands | 🔲 | | |
| 10.4 | Output rendering | 🔲 | | |
| 10.5 | Multiple terminal tabs/instances | 🔲 | | |

## 11. Code Editor

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 11.1 | Code editor window opens | 🔲 | | |
| 11.2 | Open / edit files | 🔲 | | |
| 11.3 | Syntax highlighting | 🔲 | | |
| 11.4 | Save files | 🔲 | | |

## 12. File Explorer

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 12.1 | File explorer window opens | 🔲 | | |
| 12.2 | Browse sandbox filesystem | 🔲 | | |
| 12.3 | Create / delete files & folders | 🔲 | | |
| 12.4 | Rename files | 🔲 | | |
| 12.5 | Navigate directories | 🔲 | | |

## 13. Browser

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 13.1 | Browser window opens | 🔲 | | |
| 13.2 | Navigate to URLs | 🔲 | | |
| 13.3 | URL bar works | 🔲 | | |
| 13.4 | Page loads & renders | 🔲 | | |

## 14. Shield Browser

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 14.1 | Shield browser opens | 🔲 | | |
| 14.2 | Shield protection features | 🔲 | | |
| 14.3 | Browsing works through shield | 🔲 | | |

## 15. Command Center

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 15.1 | Command center opens | 🔲 | | |
| 15.2 | Run commands / agents | 🔲 | | |
| 15.3 | View command output | 🔲 | | |

## 16. Vibe Code Studio (Kanban / Tasks)

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 16.1 | Vibe Studio window opens | 🔲 | | |
| 16.2 | Kanban board renders columns | 🔲 | | |
| 16.3 | Task cards display | 🔲 | | |
| 16.4 | Drag-and-drop cards between columns | 🔲 | | |
| 16.5 | Create / edit / delete tasks | 🔲 | | |

## 17. FreeCut (Video Editor)

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 17.1 | FreeCut window opens | 🔲 | | |
| 17.2 | Import media files | 🔲 | | |
| 17.3 | Timeline renders | 🔲 | | |
| 17.4 | Drag clips to timeline | 🔲 | | |
| 17.5 | Playback controls (play/pause/seek) | 🔲 | | |
| 17.6 | Export / render video | 🔲 | | |

## 18. Plugins

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 18.1 | Plugins window opens | 🔲 | | |
| 18.2 | List installed plugins | 🔲 | | |
| 18.3 | Install / uninstall plugins | 🔲 | | |
| 18.4 | Plugin functionality works | 🔲 | | |

### Plugin Subsystems (from `plugins/` directory)

| # | Plugin | Status | Bug Description |
|---|--------|--------|-----------------|
| 18.a | Backup & Restore | 🔲 | |
| 18.b | Disk Cleaner | 🔲 | |
| 18.c | GPU Monitor | 🔲 | |
| 18.d | Log Viewer | 🔲 | |
| 18.e | Model Downloader | 🔲 | |
| 18.f | Port Manager | 🔲 | |

## 19. Channels

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 19.1 | Channels window opens | 🔲 | | |
| 19.2 | Create / join channels | 🔲 | | |
| 19.3 | Send / receive messages | 🔲 | | |

## 20. MCP Tools

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 20.1 | MCP Tools window opens | 🔲 | | |
| 20.2 | List MCP servers / tools | 🔲 | | |
| 20.3 | Invoke / test tools | 🔲 | | |

## 21. Skills

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 21.1 | Skills window opens | 🔲 | | |
| 21.2 | List available skills | 🔲 | | |
| 21.3 | Execute / run a skill | 🔲 | | |

## 22. Knowledge

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 22.1 | Knowledge window opens | 🔲 | | |
| 22.2 | Browse knowledge items | 🔲 | | |
| 22.3 | Search knowledge | 🔲 | | |

## 23. Architecture Viewer

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 23.1 | Architecture window opens | 🔲 | | |
| 23.2 | System diagram / visualization | 🔲 | | |

## 24. Screenshot

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 24.1 | Screenshot window opens | 🔲 | | |
| 24.2 | Capture screenshot | 🔲 | | |
| 24.3 | View screenshot results | 🔲 | | |

## 25. Service Hub

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 25.1 | Service Hub window opens | 🔲 | | |
| 25.2 | List running services | 🔲 | | |
| 25.3 | Start / stop services | 🔲 | | |
| 25.4 | View service logs | 🔲 | | |

## 26. Settings

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 26.1 | Settings window opens | 🔲 | | |
| 26.2 | Change preferences | 🔲 | | |
| 26.3 | Settings persist on restart | 🔲 | | |

## 27. Logs

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 27.1 | Logs window opens | 🔲 | | |
| 27.2 | Logs stream in real-time | 🔲 | | |
| 27.3 | Filter / search logs | 🔲 | | |

## 28. Figma / JSON Playground

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 28.1 | Figma Render opens | 🔲 | | |
| 28.2 | JSON Playground opens | 🔲 | | |
| 28.3 | Render Figma JSON | 🔲 | | |

---

## 29. Backend / Core (Rust)

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 29.1 | REST API server starts | 🔲 | | |
| 29.2 | Sandbox filesystem jail | 🔲 | | |
| 29.3 | `uv` environment bootstrap | 🔲 | | |
| 29.4 | App install (pip deps via uv) | 🔲 | | |
| 29.5 | App launch (subprocess spawn) | 🔲 | | |
| 29.6 | App stop (process kill) | 🔲 | | |
| 29.7 | Disk usage reporting | 🔲 | | |
| 29.8 | OpenViking server lifecycle | 🔲 | | |
| 29.9 | Node.js environment bootstrap | 🔲 | | |
| 29.10 | Git clone / pull for app install | 🔲 | | |

## 30. Tauri IPC Bridge

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 30.1 | Tauri app launches | 🔲 | | |
| 30.2 | Frontend ↔ backend IPC works | 🔲 | | |
| 30.3 | Events streaming (progress, logs) | 🔲 | | |
| 30.4 | Window management commands | 🔲 | | |

---

## How to Report a Bug

1. Change the **Status** from `🔲` to `❌` (broken) or `⚠️` (partial)
2. Set the **Severity**: `🔴` Critical, `🟠` Major, `🟡` Minor, `🔵` Cosmetic
3. Write a short **Bug Description** explaining what happens

**Example:**
```
| 3.2 | Click dock icon → opens app | ❌ | 🔴 | Clicking Finder icon does nothing, no window opens |
```

---

## Summary

| Category | Total Features | ✅ Working | ⚠️ Partial | ❌ Broken | 🔲 Not Tested |
|----------|---------------|-----------|-----------|----------|--------------|
| Desktop Shell | 7 | 0 | 0 | 0 | 7 |
| Window Manager | 8 | 0 | 0 | 0 | 8 |
| Dock | 5 | 0 | 0 | 0 | 5 |
| Top Bar | 5 | 0 | 0 | 0 | 5 |
| Launchpad | 4 | 0 | 0 | 0 | 4 |
| Spotlight | 3 | 0 | 0 | 0 | 3 |
| AI Launcher | 7+9 | 0 | 0 | 0 | 16 |
| Chat | 5 | 0 | 0 | 0 | 5 |
| LLM Providers | 4 | 0 | 0 | 0 | 4 |
| Terminal | 5 | 0 | 0 | 0 | 5 |
| Code Editor | 4 | 0 | 0 | 0 | 4 |
| File Explorer | 5 | 0 | 0 | 0 | 5 |
| Browser | 4 | 0 | 0 | 0 | 4 |
| Shield Browser | 3 | 0 | 0 | 0 | 3 |
| Command Center | 3 | 0 | 0 | 0 | 3 |
| Vibe Studio | 5 | 0 | 0 | 0 | 5 |
| FreeCut | 6 | 0 | 0 | 0 | 6 |
| Plugins | 4+6 | 0 | 0 | 0 | 10 |
| Channels | 3 | 0 | 0 | 0 | 3 |
| MCP Tools | 3 | 0 | 0 | 0 | 3 |
| Skills | 3 | 0 | 0 | 0 | 3 |
| Knowledge | 3 | 0 | 0 | 0 | 3 |
| Architecture | 2 | 0 | 0 | 0 | 2 |
| Screenshot | 3 | 0 | 0 | 0 | 3 |
| Service Hub | 4 | 0 | 0 | 0 | 4 |
| Settings | 3 | 0 | 0 | 0 | 3 |
| Logs | 3 | 0 | 0 | 0 | 3 |
| Figma/JSON | 3 | 0 | 0 | 0 | 3 |
| Backend/Core | 10 | 0 | 0 | 0 | 10 |
| Tauri IPC | 4 | 0 | 0 | 0 | 4 |
| **TOTAL** | **~140** | **0** | **0** | **0** | **~140** |
