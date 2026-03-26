# Vibe Code Studio — OpenCode + Monaco IDE Integration

- **Status:** 🟡 `yolo mode`

- **Feature:** Integrate Monaco Editor and OpenCode server into Vibe Code Studio for a fully-featured, agent-powered IDE.
- **Purpose:** Evolve Vibe Code Studio from a playground into a powerful rudimentary IDE (similar to VS Code) focused on agent features. Implements real-time file editing using Monaco, full file browser, file tree navigation, file saving, and built-in Git support (diffs, typical daily git operations). Configured to work seamlessly with the existing local GGUF provider for powerful, local, open-source AI assistance.

---

## Current State

| Component          | Location                          | What It Does                                                                                     |
| ------------------ | --------------------------------- | ------------------------------------------------------------------------------------------------ |
| **VibeCodeStudio** | `components/apps/VibeCodeStudio/` | Will host the new IDE functionality as a first-class Tab with a dedicated left-sidebar.          |
| **Local LLM**      | `core/src/llm/gguf.rs`            | Provides local LLM capabilities via Llama.cpp.                                                   |

---

## Proposed Design

```text
┌────────────────────────────────────────────────────────┬─────────────┐
│                 MAIN IDE AREA (80%)                    │ CHAT (20%)  |
│                                                        │             │
│  ┌───────────────┐ ┌──────────────────────────────────┐│ [Scrum|Dev] │
│  │ EXPLORER      │ │ [index.ts x] [App.svelte]        ││             │
│  │ ▾ workspace   │ │ // Monaco Editor                 ││ Agent chat  │
│  │   📄 index.ts │ │ function start() {               ││ integrating │
│  │   📄 style.css│ │   console.log("IDE Ready");      ││ with the    │
│  │               │ │ }                                ││ active file │
│  │ ▾ GIT         │ ├──────────────────────────────────┤│             │
│  │   M index.ts  │ │ TERMINAL                 [×][^]  ││  ┌───────┐  │
│  │               │ │ nde-os$ npm run dev              ││  │ Input │  │
│  │               │ │ Server running on :5174          ││  └───────┘  │
│  └───────────────┘ └──────────────────────────────────┘│             │
│  [Status: Branch: main | Changes: +1 -0 | Lang: TS]    │             │
└────────────────────────────────────────────────────────┴─────────────┘
```

### Key Design Decisions

1.  **Monaco Editor Integration**: Incorporate a Monaco Editor component into the studio to support rich syntax highlighting, auto-completion, and real-time editing.
2.  **File Explorer**: A left sidebar (internal to the 80% view) showing the true file system tree of the workspace, allowing the user to browse, open, create, and delete files.
3.  **OpenCode Server Integration**: Integrate the OpenCode server backend configuration (using the local GGUF model) to power the agent features within Monaco (like inline suggestions or chat references).
4.  **Git Features**: Add a source control view within the sidebar displaying modified files. Allow Diff viewing and daily operations (commit, discard changes).
5.  **Real-Time Save**: Files edited in Monaco persist back to the sandbox file system.
6.  **Integrated Terminal**: A collapsible bottom panel within the IDE area running a real pseudo-terminal (PTY). Uses `xterm.js` in the frontend communicating via Tauri IPC to a Rust-managed shell process, bound securely to the workspace sandbox.

---

## Inputs & Outputs

**Inputs:**
- File selection in the file tree.
- Text input in the Monaco Editor.
- Git actions (click to stage, commit message).
- Code generation requests via Chat targeting the open file.

**Outputs:**
- Active Monaco Editor reflecting the selected file's content.
- Saved files on the local filesystem.
- Git diff views (side-by-side or inline) triggered by the source control panel.
- Applied code modifications from the agent.

---

## Edge Cases & Security

-   **Large Files**: Handle loading very large files safely without crashing the Monaco editor.
-   **Binary Files**: Block binary files from loading in the text editor.
-   **Simultaneous Edits**: If a file is changed externally, prompt to reload or auto-reload within Monaco.
-   **Unsaved Changes**: Warn the user before closing a tab with unsaved edits.
-   **Sandbox Boundary**: Ensure the file browser strictly respects the workspace sandbox limits (canonicalized paths).

---

## Task Checklist

- [x] **1. Monaco Editor Setup**
  - [x] 1.1 Install and configure Monaco Editor (e.g., using monaco-editor or a Svelte wrapper) in the desktop frontend.
  - [x] 1.2 Create a `CodeEditor.svelte` wrapper handling Monaco initialization and theme syncing.
  - [x] 1.3 Add standard editor features: syntax highlighting for core languages, manual save with `Cmd+S`/`Ctrl+S`.

- [x] **2. File System Tree & Workspace Browsing**
  - [x] 2.1 Develop Tauri backend commands in `core` to list, read, write, and delete files safely within the workspace sandbox.
  - [x] 2.2 Create a `FileExplorer.svelte` sidebar panel to display the recursive directory tree.
  - [x] 2.3 Wire the File Explorer to open files in `CodeEditor.svelte` as active editor tabs.
  - [x] 2.4 Prompt user to explicitly select a project folder (sub-directory) to load before displaying the entire workspace root in the Explorer.

- [x] **3. OpenCode Server & LLM Integration**
  - [x] 3.1 Configure the backend to launch/support the OpenCode server routing.
  - [x] 3.2 Wire the OpenCode server logic to the local GGUF provider for agent suggestions.
  - [x] 3.3 Allow the side chat panel to read the currently active file context from Monaco and patch code back.

- [x] **4. Build-in Git Support**
  - [x] 4.1 Write Tauri commands wrapping basic git CLI operations (status, diff, add, commit) via subprocess or a git crate.
  - [x] 4.2 Create a `SourceControl.svelte` view listing modified files (Added, Modified, Deleted).
  - [x] 4.3 Implement a Monaco Diff Editor view to show before/after changes when clicking a modified file.

- [x] **5. Unified IDE Layout & Polish**
  - [x] 5.1 Refactor `VibeCodeStudio.svelte` to support the new IDE layout (adding the left sidebar for Explorer/Git).
  - [x] 5.2 Add an `IDE` or `Code` tab alongside the existing ones, or adapt the primary workspace layout.
  - [x] 5.3 Ensure styling matches the sleek macOS/Ventura aesthetics using Tailwind + shadcn-svelte.
  - [x] 5.4 Write E2E/Playwright tests for file opening, editing, and saving.

- [ ] **6. Integrated Terminal**
  - [ ] 6.1 Create raw PTY backend in Rust (e.g. using `portable-pty` or a Tauri plugin) to spawn a sandboxed shell (`cmd.exe` or `sh`).
  - [ ] 6.2 Expose Tauri commands for terminal lifecycle: `spawn_pty`, `write_pty`, and emit `pty_read` events.
  - [ ] 6.3 Integrate `xterm.js` into a new `TerminalPanel.svelte` component that binds to the Tauri IPC streams.
  - [ ] 6.4 Add a resizable/collapsible bottom panel to `IDE.svelte` to host the Terminal.

---

## Definition of Done

- [x] **Local DoD**:
  - Monaco editor correctly renders, edits, and saves files.
  - File tree accurately represents the workspace and supports directory browsing.
  - Git view displays active modifications and supports viewing diffs.
  - OpenCode integration provides Agent with context from the IDE.
- [x] **Global DoD**:
  - Code uses only Tailwind / shadcn-svelte.
  - Robust exception handling in Rust backend (anyhow).
  - Desktop E2E tested within Playwright + Chromium CDP context.
  - No Mocks, No Fakes.
