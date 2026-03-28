---
status: "🟢 done by AI"
---

# Feature: Resizable Sidebars for VibeCode Studio

## Purpose
Make the left (LayerTree), right (PropertiesPanel), and chat (AgentChat) panels resizable in VibeCode Studio, matching standard IDE behavior.

## Layout Model

There are 3 independent resize boundaries at different nesting levels:

```
Top-level flex (VibeCodeStudio.svelte)
├── Main Area (flex-1)              ← resize handle on right edge
│   └── Preview tab inner flex
│       ├── LayerTree (w-64)        ← resize handle on right edge
│       ├── CanvasEditor (flex-1)
│       └── PropertiesPanel (w-72)  ← resize handle on left edge
└── Chat Panel (w-80)
```

- LayerTree and PropertiesPanel only exist inside the Preview tab.
- Chat Panel is a top-level sibling, always visible on `lg+` breakpoint.

## Existing Pattern to Follow

`IDE.svelte` (terminal resize) already implements vertical panel resizing using:
- `$state` for `terminalHeight`
- `svelte:window` `onmousemove` / `onmouseup` for global drag tracking
- Inline `style="height: {terminalHeight}px"` for dynamic sizing

Adapt this pattern for horizontal / `col-resize` behavior.

## Inputs & Outputs
- **Inputs**: Mouse drag events on resize handles between panels.
- **Outputs**: Dynamically updated `$state` width values for each panel, applied via inline styles.

## Constraints
- **Min width**: 150px per panel.
- **Max width**: 50% of parent container.
- **Cursor lock**: During drag, cursor must stay `col-resize` globally via `svelte:window` handlers (not just on the handle element).
- **State lifetime**: Width state lives in `VibeCodeStudio.svelte` so it survives tab switches (LayerTree/PropertiesPanel are destroyed on tab change).

## Implementation

### 1. Create `<ResizeHandle>` component
- Vertical divider element, ~4px wide hit area.
- Visual: 1px line default, widens to 3-4px with `indigo-500` accent on hover.
- Props: `onResize(deltaX)` callback, `side: 'left' | 'right'` (which direction positive delta grows).
- Uses `onmousedown` on handle, `svelte:window` `onmousemove`/`onmouseup` for drag.
- Sets `col-resize` cursor on `document.body` during drag.

### 2. Add `$state` width variables in `VibeCodeStudio.svelte`
```ts
let layerTreeWidth = $state(256);    // was w-64
let propertiesWidth = $state(288);   // was w-72
let chatWidth = $state(320);         // was w-80
```

### 3. Replace fixed Tailwind widths with inline styles
- `LayerTree`: remove `w-64`, add `style="width: {layerTreeWidth}px"`
- `PropertiesPanel`: remove `w-72`, add `style="width: {propertiesWidth}px"`
- Chat Panel `<div>`: remove `w-80`, add `style="width: {chatWidth}px"`

### 4. Insert `<ResizeHandle>` at each boundary
- Between LayerTree and CanvasEditor (inside Preview tab flex).
- Between CanvasEditor and PropertiesPanel (inside Preview tab flex).
- Between Main Area and Chat Panel (top-level flex).

### 5. Enforce min/max in drag handler
Clamp values in each `onResize` callback before updating `$state`.

## Task Checklist
- [x] Create `ResizeHandle.svelte` component with horizontal drag logic
- [x] Add `$state` width variables in `VibeCodeStudio.svelte`
- [x] Replace LayerTree `w-64` with dynamic inline width + resize handle
- [x] Replace PropertiesPanel `w-72` with dynamic inline width + resize handle
- [x] Replace Chat Panel `w-80` with dynamic inline width + resize handle
- [x] Verify min/max constraints are enforced
- [x] Verify cursor stays `col-resize` globally during drag
- [x] Verify tab switching preserves width state
- [x] Verify responsive breakpoints (`md`, `lg`) still work

## Definition of Done (DoD)
- [x] All 3 panels can be freely resized horizontally via drag handles.
- [x] Min (150px) / Max (50% parent) constraints respected.
- [x] No layout overflow or breakage on any tab.
- [x] No performance jank during dragging.
- [x] Global DoD met (no panics, no TODOs).
