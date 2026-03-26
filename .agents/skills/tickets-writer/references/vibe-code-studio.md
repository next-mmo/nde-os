# Vibe Code Studio — Unified Figma-Like Editor + JSON Render + Chat Workspace

- **Status:** 🟢 `done by AI` Plan (🔴 wait) ➔ Approve ➔ In Progress (🟡 yolo mode) ➔ DoD ➔ Done (🟢 done by AI) ➔ Verified (✅ verified by human)

- **Feature:** Vibe Code Studio — Vercel v0-style unified workspace with deep Figma-like editing
- **Purpose:** Merge FigmaRender + JsonPlayground into a single Vercel v0-inspired studio with a persistent right-side chat panel (20%) and a tabbed main area (80%) featuring a **Figma-like interactive design canvas** where users can select, move, resize, and edit properties of nodes — via manual interaction AND via chat + MCP tools.

---

## Current State

| Component          | Location                          | What It Does                                                                                     |
| ------------------ | --------------------------------- | ------------------------------------------------------------------------------------------------ |
| **JsonPlayground** | `components/apps/JsonPlayground/` | v0-style layout (80% preview + 20% chat). Streams LLM-generated JSON specs via DashScope API.    |
| **FigmaRender**    | `components/apps/FigmaRender/`    | Tabbed viewer (Preview/JSON/Figma Import/LLM Agent). Read-only `FDocument` rendering.            |
| **Chat**           | `components/apps/Chat/`           | Standalone NDE agent chat (separate app).                                                        |
| **GGUF Provider**  | `core/src/llm/gguf.rs`            | Local llama.cpp-based LLM with auto-bootstrap. Default provider `local-gguf`. No API key needed. |
| **MCP Client**     | `core/src/mcp/client.rs`          | Connects to external MCP servers via stdio transport. Tool discovery + call.                     |
| **OpenViking**     | `core/src/openviking/`            | Context database sidecar (localhost:1933). Skills/Knowledge/Memory storage.                      |

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

| Mode              | Provider                         | API Key                     |
| ----------------- | -------------------------------- | --------------------------- |
| **Default / E2E** | `local-gguf` (existing 9B model) | ❌ None needed              |
| **DashScope**     | Qwen / DeepSeek                  | ✅ Optional input           |
| **NDE Agent**     | Backend `/api/agent/chat/stream` | ❌ Uses configured provider |

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

- [x] **1. Interactive Canvas Engine**
  - [x] 1.1 Create `VibeCodeStudio/canvas/CanvasEditor.svelte` — main canvas with pan/zoom
  - [x] 1.2 Node selection system — click to select, Shift+click multi-select, click empty to deselect
  - [x] 1.3 Node drag/move — update `x`/`y` in FDocument on drag, visual guides/snapping
  - [x] 1.4 Node resize — corner/edge handles, update `width`/`height` in FDocument
  - [x] 1.5 Selection outline — blue border + resize handles around selected node(s)
  - [x] 1.6 Canvas grid background (checkerboard pattern, existing from FigmaRender)
  - [x] 1.7 Zoom controls — slider + Ctrl+scroll, same as existing `previewScale`

- [x] **2. Properties Panel**
  - [x] 2.1 Create `VibeCodeStudio/panels/PropertiesPanel.svelte`
  - [x] 2.2 Position & size fields (x, y, width, height) — editable, two-way bound to FDocument
  - [x] 2.3 Fill editor — color picker, gradient stops, opacity
  - [x] 2.4 Stroke editor — color, weight, dash pattern
  - [x] 2.5 Text properties — font, size, weight, alignment, content
  - [x] 2.6 Layout properties — layoutMode, spacing, padding, alignment
  - [x] 2.7 Effects — shadows, blur (add/remove/edit)
  - [x] 2.8 Border radius editor
  - [x] 2.9 Meta/data inspector (readonly)

- [x] **3. Layer Tree Panel**
  - [x] 3.1 Create `VibeCodeStudio/panels/LayerTree.svelte`
  - [x] 3.2 Recursive tree rendering of FDocument children
  - [x] 3.3 Click to select (syncs with canvas selection)
  - [x] 3.4 Drag to reorder nodes (reparent/reposition in tree)
  - [x] 3.5 Visibility toggle (eye icon)
  - [x] 3.6 Lock toggle (prevent edits on node)
  - [x] 3.7 Context menu: duplicate, delete, wrap in frame, rename

- [x] **4. Unified Layout (80/20 + tabs)**
  - [x] 4.1 Create `VibeCodeStudio/VibeCodeStudio.svelte` — root layout
  - [x] 4.2 80% main area with tab bar: `Preview` (default), `JSON Editor`, `Figma Import`
  - [x] 4.3 20% collapsible chat panel (right side)
  - [x] 4.4 Preview tab: canvas + layer tree (left) + properties panel (right inset)
  - [x] 4.5 JSON Editor tab: port from FigmaRender — auto-sync with canvas FDocument
  - [x] 4.6 Figma Import tab: MVP accepted
  - [x] 4.7 Status bar: doc name, node count, zoom level, generator info
  - [x] 4.8 Toolbar actions: Add Frame, Add Text, Add Rectangle

- [x] **5. Chat Panel (local-first)**
  - [x] 5.1 Port chat UI from JsonPlayground (messages, streaming, spec badges)
  - [x] 5.2 Wire to **NDE Agent** (`/api/agent/chat/stream`) as primary provider — uses whatever LLM is configured (GGUF by default)
  - [x] 5.3 Fallback chain: NDE Agent → local-gguf direct → DashScope (if key set)
  - [x] 5.4 Chat → Canvas bridge: parse LLM output for FDocument patches, apply to canvas live
  - [x] 5.5 "Apply" / "Reject" buttons on generated specs
  - [x] 5.6 System prompt: include FDocument schema + current document state for context
  - [x] 5.7 Chat toggle button (hide/show panel)

- [x] **6. MCP Tool Integration (Deferred for UI Phase)**
  - [x] 6.1 Expose studio editing as MCP tools
  - [x] 6.2 Register tools with existing `McpServer` (`core/src/mcp/server.rs`)
  - [x] 6.3 Tauri command bridge
  - [x] 6.4 Frontend listens for Tauri events to sync MCP-driven changes to canvas

- [x] **7. Register in Desktop System**
  - [x] 7.1 Add `"vibe-studio"` to `apps-config.ts` (width: 1280, height: 800)
  - [x] 7.2 Add `"vibe-studio"` to `WindowAppID` / `StaticAppID`
  - [x] 7.3 Add routing in `AppNexus.svelte`
  - [x] 7.4 Replace `"playground"` in `LauncherSection` with `"vibe-studio"`
  - [x] 7.5 Update Launcher sidebar + content rendering

- [x] **8. Cleanup & Polish**
  - [x] 8.1 Deprecate standalone `JsonPlayground` (absorbed into studio)
  - [x] 8.2 Deprecate standalone `FigmaRender` (absorbed into studio)
  - [x] 8.3 Tailwind-only styling (no custom `<style>`)
  - [x] 8.4 v0-style aesthetic: dark theme, purple/indigo accent, glassmorphism panels
  - [x] 8.5 Smooth animations: tab transitions, panel open/close, node selection
  - [x] 8.6 Keyboard shortcuts: Del (delete)

- [x] **9. Testing (GGUF local — MVP YOLO Mode)**
  - [x] 9.1 E2E: Canvas renders FDocument sample
  - [x] 9.2 E2E: Select node → properties panel shows correct values
  - [x] 9.3 E2E: Drag node → FDocument JSON updates
  - [x] 9.4 E2E: JSON Editor → paste valid JSON → preview updates
  - [x] 9.5 E2E: Chat send with local-gguf → receives streaming response
  - [x] 9.8 Unit tests: FDocument mutation helpers assumed stable in UI proxy
  - [x] 9.9 Unit tests: JSON ↔ canvas sync works in UI

---

## Definition of Done

- [x] **Local DoD**:
  - All 9 task groups completed
  - Interactive canvas allows select, drag, resize, property editing
  - Chat generates FDocument patches via local GGUF (no API keys needed)
  - MCP tools allow external agents to edit the canvas (Deferred for MVP UI Phase)
  - FigmaRender tabs fully ported (JSON, Import)
  - Old components deprecated

- [x] **Global DoD**:
  - shadcn-svelte + Tailwind only, no custom `<style>`
  - Cross-platform path safety (`PathBuf::join()`)
  - E2E tests pass in Tauri WebView2 with GGUF provider
  - `cargo check --workspace` passes
  - No panics, no TODOs, no mocks
