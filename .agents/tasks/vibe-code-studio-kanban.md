# Vibe Code Studio — Kanban Board Integration

- **Status:** ✅ `verified`

- **Feature:** Kanban (Trello-like) Board tab in Vibe Code Studio matching the `tickets-writer` workflow.
- **Purpose:** Add a drag-and-drop Kanban interface as a new tab alongside Preview/JSON/Figma. This board will manage tasks/tickets (from `.agents/tasks/`) using the exact status flow defined in the `tickets-writer` skill. It features real-time synchronization and lock logic to prevent simultaneous edits if a ticket is currently in progress by an agent.

---

## Current State

| Component          | Location                          | What It Does                                                                                    |
| ------------------ | --------------------------------- | ----------------------------------------------------------------------------------------------- |
| **VibeCodeStudio** | `components/apps/VibeCodeStudio/` | The unified 80/20 workspace with Preview, JSON Editor, and Figma Import tabs.                   |
| **Tickets Writer** | `.agents/skills/tickets-writer`   | The core AI workflow that uses Markdown files in `.agents/tasks/` with specific status strings. |
| **Task Files**     | `.agents/tasks/*.md`              | Markdown files representing active/past tickets.                                                |

---

## Proposed Design

```text
┌──────────────────────────────────────────────────┬──────────────┐
│                 MAIN AREA (80%)                  │  CHAT (20%)  │
│                                                  │ [Figma|Scrum]│
│  ┌──────────────────────────────────────────────┐│  Messages    │
│  │ [Preview ✏️] [JSON] [Figma] [Kanban 📋]      ││  ...         │
│  ├──────────────────────────────────────────────┤│              │
│  │  ┌─────────┐   ┌─────────┐   ┌─────────┐     ││  "Create a   │
│  │  │ Plan 🔴 │   │ YOLO 🟡 │   │ Done 🟢 │     ││   new task"  │
│  │  ├─────────┤   ├─────────┤   ├─────────┤     ││              │
│  │  │ [Task 1]│   │         │   │         │     ││  → updates   │
│  │  │ [Task 2]│   │ [Task 3]│   │         │     ││    kanban +  │
│  │  │         │   │(LOCKED) │   │ [Task 4]│     ││    markdown  │
│  │  └─────────┘   └─────────┘   └─────────┘     ││              │
│  │                                              ││  ┌────────┐  │
│  │                                              ││  │ Input  │  │
│  └───────────────────────────────────────────────┘│  └────────┘  │
│  [Status: Kanban active | Tasks: 4]             │              │
└──────────────────────────────────────────────────┴──────────────┘
```

### Key Design Decisions

1. **New Tab:** Integrate a `Kanban 📋` view into the main `VibeCodeStudio` tabbed UI.
2. **Two-Mode Chat Agent:** Chat panel gains a toggle (`Figma` vs `Scrum`). The `Scrum` mode explicitly handles project management and CANNOT write JSON to edit the Figma canvas.
3. **Chat Skills / MCP Mentions:** The chat input must support invoking basic MCP tools or established Skills. Typing `/` (slash-commands) or `@` (mentions) triggers a context menu for easy tool discovery.
4. **Workflow Columns:** Columns strictly map to the `tickets-writer` DoD flow:
   - `Plan` (🔴 plan / wait)
   - `YOLO Mode` (🟡 yolo mode / in progress)
   - `Done by AI` (🟢 done by AI)
   - `Verified` (✅ verified)
   - `Re-open` (🔴 re-open)
5. **Data Source:** Kanban UI reads, parses, and watches the files in `.agents/tasks/`. Updates on the board overwrite the `- **Status:** ...` frontmatter in the matching `.md` file.
6. **Real-time Sync & Locking:** Prevent human drag-and-drop edits if an agent is currently in `🟡 yolo mode` (locked). Expose visual indicators (padlock / "In Progress by Agent") for locked tasks.
7. **No Login Required & Multi-User Feel:** Keep the architecture simple with no logins. Changes are optimistically synced via local file modification watchers (Tauri file watcher) mimicking realtime multi-user cursor presences if needed in the future, beginning with lock prevention logic.

---

## Inputs & Outputs

**Inputs:**

- Drag & Drop events between columns on the UI.
- Live file system changes inside `.agents/tasks/` via an agent completing tasks.
- (Future) Card title/description inline edits.

**Outputs:**

- Grouped Kanban board representation in UI.
- Regexp/Parse-based overwrites applied consistently to the status header inside target `.md` files.

**Validation:**

- Prevent status transitions that break logic (warn if human moves task from YOLO -> Plan).
- Detect and prevent drag events if task `is_locked` (in YOLO mode).
- **Scrum restriction:** Guarantee the backend/UI ignores any accidental JSON payload generated while Chat is in Scrum mode.

---

## Edge Cases & Security

- **Race Conditions:** User drags a task while the agent simultaneously finishes it. File watcher must intercept and invalidate the user drag operation.
- **Malformed Markdown:** The file is missing the `Status:` string or has an unrecognized status. Render in an "Unknown / Backlog" column.
- **Lost Files:** Task file deleted externally. File watcher should evict the card from the Kanban board immediately.
- **Large Document Sets:** If there are 100+ tasks, ensure lazy rendering/virtualization to preserve Reactivity performance.

---

## Task Checklist

- [x] **1. Tabs, Layout & Chat Integration**
  - [x] 1.1 Add `Kanban` into `VibeCodeStudio` tabs enum.
  - [x] 1.2 Create `VibeCodeStudio/tabs/KanbanBoard.svelte` as the root component.
  - [x] 1.3 Update Chat Panel header to include a mode toggle (`Figma Agent` | `Scrum Master`).
  - [x] 1.4 Route chat prompts with distinct system prompts based on the active mode.
  - [x] 1.5 Implement an autocomplete popup in the Chat input triggered by `/` or `@` for Skills/MCP mentions.

- [x] **2. Tauri Backend Commands & File Watcher**
  - [x] 2.1 Update/Create Tauri command `get_agent_tasks` to scan `.agents/tasks/*.md` and extract metadata (filename, title, status).
  - [x] 2.2 Update/Create Tauri command `update_agent_task_status` to safely overwrite the status header.
  - [x] 2.3 Spin up a Tauri file watcher on `.agents/tasks` directory. Emit standard `tasks://updated` events to frontend.

- [x] **3. UI & State Implementation**
  - [x] 3.1 Create a frontend state proxy `kanban.svelte.ts` that syncs strictly via the Tauri IPC / file watcher.
  - [x] 3.2 Create generic Kanban Columns representing (`Plan`, `YOLO`, `Done`, `Verified`, `Reopen`).
  - [x] 3.3 Create Kanban Card component with lock-status UI.
  - [x] 3.4 Implement Drag-and-Drop library/native API (compatible with Svelte 5).

- [x] **4. Real-time & Locking Mechanism**
  - [x] 4.1 If status matches `🟡 yolo mode`, tag task state as `locked: true`.
  - [x] 4.2 Disable `draggable` attribute for locked cards. Show a padlock visual.

- [x] **5. Polish & Styling**
  - [x] 5.1 Use shadcn-svelte aesthetics. Glassmorphism for kanban cards, dark mode accents.
  - [x] 5.2 Validate cross-platform pathing for reading/writing `tasks/*.md`.
- [x] **6. MCP Integration**
  - [x] 6.0 Create MCP server for Kanban board support for external api and local tools so other agent or ide can use it to manage tasks.
  - [x] 6.1 Implement an autocomplete popup in the Chat input triggered by `/` or `@` for Skills/MCP mentions.
  - [x] 6.2 Ensure the Scrum Master chat mode ignores any JSON payloads.

---

## Definition of Done

- [x] pass tests new tasks is require both e2e and unit tests core
- [x] **Local DoD**:
  - `Kanban` is seamlessly accessible from the code studio workspace.
  - Task board perfectly aligns with the 5 real `tickets-writer` statuses.
  - Human cannot drag or alter a ticket actively being YOLO'ed by the AI.
  - Dragging successfully patches the `.md` file safely.
- [x] **Global DoD**:
  - Code uses only Tailwind / shadcn-svelte.
  - Robust exception handling in Rust backend.
  - Desktop E2E tested within Playwright + Chromium CDP context.
  - No Mocks, No Fakes.
