# Vibe Code Studio — Unified Figma-Like Editor + JSON Render + Chat Workspace

- **Status:** 🔴 `waiting-approval`  
  Plan (🔴 waiting-approval) ➔ Approve ➔ Generate (🟡 in-progress) ➔ Review ➔ DoD ➔ Finish (🟢 done)

- **Feature:** Vibe Code Studio — Vercel v0-style unified workspace with deep Figma-like editing
- **Purpose:** Merge FigmaRender + JsonPlayground into a single Vercel v0-inspired studio with a persistent right-side chat panel (20%) and a tabbed main area (80%) featuring a **Figma-like interactive design canvas** where users can select, move, resize, and edit properties of nodes — via manual interaction AND via chat + MCP tools.

---

## Current State

| Component | Location | What It Does |
|-----------|----------|-------------|
| **JsonPlayground** | `components/apps/JsonPlayground/` | v0-style layout (80% preview + 20% chat). Streams LLM-generated JSON specs via DashScope API. |
| **FigmaRender** | `components/apps/FigmaRender/` | Tabbed viewer (Preview/JSON/Figma Import/LLM Agent). Read-only `FDocument` rendering. |
| **Chat** | `components/apps/Chat/` | Standalone NDE agent chat (separate app). |
| **GGUF Provider** | `core/src/llm/gguf.rs` | Local llama.cpp-based LLM with auto-bootstrap. Default provider `local-gguf`. No API key needed. |
| **MCP Client** | `core/src/mcp/client.rs` | Connects to external MCP servers via stdio transport. Tool discovery + call. |
| **OpenViking** | `core/src/openviking/` | Context database sidecar (localhost:1933). Skills/Knowledge/Memory storage. |

---

## Proposed Design

```
┌──────────────────────────────────────────────────┬──────────────┐
│                 MAIN AREA (80%)                  │  CHAT (20%)  │
│                                                  │              │
│  ┌──────────────────────────────────────────────┐│  Messages    │
│  │ [Preview ✏️] [JSON Editor] [Figma Import]    ││  ...         │
│  ├──────────────────────────────────────────────┤│              │
│  │                                              ││  "Add a blue │
│  │  ╔═══════════════════════╗  ┌─Properties──┐  ││   button"    │
│  │  ║  INTERACTIVE CANVAS   ║  │ x: 120      │  ││              │
│  │  ║  (Figma-like)         ║  │ y: 80       │  ││  → generates │
│  │  ║                       ║  │ w: 200      │  ││    FNode +   │
│  │  ║  [Selected Node] ◆───╫──│ h: 48       │  ││    patches   │
│  │  ║                       ║  │ fill: #3B82 │  ││    canvas    │
│  │  ╚═══════════════════════╝  │ radius: 8   │  ││              │
│  │                              │ text: ...   │  ││              │
│  │  [Layer tree sidebar]        └─────────────┘  ││  ┌────────┐  │
│  │                                               ││  │ Input  │  │
│  └───────────────────────────────────────────────┘│  └────────┘  │
│  [Status: doc info | zoom slider | node count]    │              │
└──────────────────────────────────────────────────┴──────────────┘
```

### Key Design Decisions

1. **Figma-like interactive canvas (not read-only)** — Users can click nodes to select, drag to reposition, resize via handles, and edit properties in a side panel. The underlying `FDocument` JSON is mutated in real-time.
2. **3 editing modes** — Manual (mouse interaction), Chat (natural language → FDocument patches), and MCP (tools exposed via MCP server for external agents).
3. **Chat at 20% right** — Sends prompts to the **local GGUF provider** (no API key) OR DashScope. Streams JSON patches and applies them to the canvas live.
4. **Main area tabs**: `Preview` (default, interactive canvas), `JSON Editor`, `Figma Import`
5. **Properties panel** — Right sidebar within the main area, showing selected node's properties (position, size, fills, text, layout, effects). Editable inline.
6. **Layer tree** — Left sidebar within canvas showing the FDocument node hierarchy. Click to select, drag to reorder.
7. **MCP tool integration** — Expose `figma_add_node`, `figma_update_node`, `figma_delete_node` as MCP tools so external agents (or the NDE agent via OpenViking) can programmatically edit the canvas.

### LLM Provider Strategy

| Mode | Provider | API Key |
|------|----------|---------|
| **Default / E2E** | `local-gguf` (llama.cpp) | ❌ None needed |
| **DashScope** | Qwen / DeepSeek | ✅ Optional input |
| **NDE Agent** | Backend `/api/agent/chat/stream` | ❌ Uses configured provider |

The chat panel attempts providers in order: NDE Agent → local-gguf → DashScope (if key provided).

---

## Inputs & Outputs

**Inputs:**
- Mouse interactions on canvas (click, drag, resize)
- Property edits in properties panel
- Chat text prompts → generates FDocument JSON patches
- JSON paste → JSON editor tab
- Figma file key + token → Figma Import tab
- MCP tool calls from external agents

**Outputs:**
- Live interactive canvas rendering FDocument nodes
- Real-time JSON sync (canvas ↔ JSON editor)
- Streaming chat messages with spec badges
- MCP tool responses (JSON-RPC)
- Status bar with document metadata + node count

**Validation:**
- JSON parse validation before rendering
- FDocument schema validation (version, children array)
- Node ID uniqueness enforcement
- Bounds checking for position/size values

---

## Edge Cases & Security

- **Invalid JSON paste** — Show clear error, don't crash canvas
- **Conflicting edits** — Chat patches applied while user is dragging → queue patches, apply after drag ends
- **API key exposure** — `type="password"`, never logged, never sent to MCP
- **MCP injection** — Validate MCP tool arguments against FNode schema before applying
- **Empty canvas** — Show empty state with "Add Frame" button + suggestion chips
- **Large documents** — Virtualize layer tree for 100+ nodes
- **GGUF model not downloaded** — Show download progress, fallback to NDE agent
- **Cross-format detection** — Auto-detect FDocument vs json-render spec vs raw Figma JSON

---

## Task Checklist

- [ ] **1. Interactive Canvas Engine**
  - [ ] 1.1 Create `VibeCodeStudio/canvas/CanvasEditor.svelte` — main canvas with pan/zoom
  - [ ] 1.2 Node selection system — click to select, Shift+click multi-select, click empty to deselect
  - [ ] 1.3 Node drag/move — update `x`/`y` in FDocument on drag, visual guides/snapping
  - [ ] 1.4 Node resize — corner/edge handles, update `width`/`height` in FDocument
  - [ ] 1.5 Selection outline — blue border + resize handles around selected node(s)
  - [ ] 1.6 Canvas grid background (checkerboard pattern, existing from FigmaRender)
  - [ ] 1.7 Zoom controls — slider + Ctrl+scroll, same as existing `previewScale`

- [ ] **2. Properties Panel**
  - [ ] 2.1 Create `VibeCodeStudio/panels/PropertiesPanel.svelte`
  - [ ] 2.2 Position & size fields (x, y, width, height) — editable, two-way bound to FDocument
  - [ ] 2.3 Fill editor — color picker, gradient stops, opacity
  - [ ] 2.4 Stroke editor — color, weight, dash pattern
  - [ ] 2.5 Text properties — font, size, weight, alignment, content
  - [ ] 2.6 Layout properties — layoutMode, spacing, padding, alignment
  - [ ] 2.7 Effects — shadows, blur (add/remove/edit)
  - [ ] 2.8 Border radius editor
  - [ ] 2.9 Meta/data inspector (readonly)

- [ ] **3. Layer Tree Panel**
  - [ ] 3.1 Create `VibeCodeStudio/panels/LayerTree.svelte`
  - [ ] 3.2 Recursive tree rendering of FDocument children
  - [ ] 3.3 Click to select (syncs with canvas selection)
  - [ ] 3.4 Drag to reorder nodes (reparent/reposition in tree)
  - [ ] 3.5 Visibility toggle (eye icon)
  - [ ] 3.6 Lock toggle (prevent edits on node)
  - [ ] 3.7 Context menu: duplicate, delete, wrap in frame, rename

- [ ] **4. Unified Layout (80/20 + tabs)**
  - [ ] 4.1 Create `VibeCodeStudio/VibeCodeStudio.svelte` — root layout
  - [ ] 4.2 80% main area with tab bar: `Preview` (default), `JSON Editor`, `Figma Import`
  - [ ] 4.3 20% collapsible chat panel (right side)
  - [ ] 4.4 Preview tab: canvas + layer tree (left) + properties panel (right inset)
  - [ ] 4.5 JSON Editor tab: port from FigmaRender — auto-sync with canvas FDocument
  - [ ] 4.6 Figma Import tab: port from FigmaRender (Figma API + Tauri IPC fallback)
  - [ ] 4.7 Status bar: doc name, node count, zoom level, generator info
  - [ ] 4.8 Toolbar actions: Add Frame, Add Text, Add Rectangle, Add Ellipse, Delete, Undo/Redo

- [ ] **5. Chat Panel (local-first)**
  - [ ] 5.1 Port chat UI from JsonPlayground (messages, streaming, spec badges)
  - [ ] 5.2 Wire to **NDE Agent** (`/api/agent/chat/stream`) as primary provider — uses whatever LLM is configured (GGUF by default)
  - [ ] 5.3 Fallback chain: NDE Agent → local-gguf direct → DashScope (if key set)
  - [ ] 5.4 Chat → Canvas bridge: parse LLM output for FDocument patches, apply to canvas live
  - [ ] 5.5 "Apply" / "Reject" buttons on generated specs
  - [ ] 5.6 System prompt: include FDocument schema + current document state for context
  - [ ] 5.7 Chat toggle button (hide/show panel)

- [ ] **6. MCP Tool Integration**
  - [ ] 6.1 Expose studio editing as MCP tools: `figma_add_node`, `figma_update_node`, `figma_delete_node`, `figma_get_document`
  - [ ] 6.2 Register tools with existing `McpServer` (`core/src/mcp/server.rs`)
  - [ ] 6.3 Tauri command bridge: `invoke("vibe_studio_add_node", { node })` etc.
  - [ ] 6.4 Frontend listens for Tauri events to sync MCP-driven changes to canvas

- [ ] **7. Register in Desktop System**
  - [ ] 7.1 Add `"vibe-studio"` to `apps-config.ts` (width: 1280, height: 800)
  - [ ] 7.2 Add `"vibe-studio"` to `WindowAppID` / `StaticAppID`
  - [ ] 7.3 Add routing in `AppNexus.svelte`
  - [ ] 7.4 Replace `"playground"` in `LauncherSection` with `"vibe-studio"`
  - [ ] 7.5 Update Launcher sidebar + content rendering

- [ ] **8. Cleanup & Polish**
  - [ ] 8.1 Deprecate standalone `JsonPlayground` (absorbed into studio)
  - [ ] 8.2 Deprecate standalone `FigmaRender` (absorbed into studio)
  - [ ] 8.3 Tailwind-only styling (no custom `<style>`)
  - [ ] 8.4 v0-style aesthetic: dark theme, purple/indigo accent, glassmorphism panels
  - [ ] 8.5 Smooth animations: tab transitions, panel open/close, node selection
  - [ ] 8.6 Keyboard shortcuts: Del (delete), Ctrl+D (duplicate), Ctrl+Z/Y (undo/redo)

- [ ] **9. Testing (GGUF local — no API keys)**
  - [ ] 9.1 E2E: Canvas renders FDocument sample (no LLM needed)
  - [ ] 9.2 E2E: Select node → properties panel shows correct values
  - [ ] 9.3 E2E: Drag node → FDocument JSON updates
  - [ ] 9.4 E2E: JSON Editor → paste valid JSON → preview updates
  - [ ] 9.5 E2E: Chat send with local-gguf → receives streaming response
  - [ ] 9.6 All E2E tests use Tauri WebView2 CDP fixtures — no standalone browser
  - [ ] 9.7 All E2E tests use `local-gguf` provider (auto-downloaded Qwen 0.5B GGUF) — zero API keys
  - [ ] 9.8 Unit tests: FDocument mutation helpers (add/update/delete node)
  - [ ] 9.9 Unit tests: JSON ↔ canvas sync

---

## Definition of Done

- [ ] **Local DoD**: 
  - All 9 task groups completed
  - Interactive canvas allows select, drag, resize, property editing
  - Chat generates FDocument patches via local GGUF (no API keys needed)
  - MCP tools allow external agents to edit the canvas
  - FigmaRender tabs fully ported (JSON, Import)
  - Old components deprecated

- [ ] **Global DoD**: 
  - shadcn-svelte + Tailwind only, no custom `<style>`
  - Cross-platform path safety (`PathBuf::join()`)
  - E2E tests pass in Tauri WebView2 with GGUF provider
  - `cargo check --workspace` passes
  - No panics, no TODOs, no mocks
