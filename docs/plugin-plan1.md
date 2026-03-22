# Standalone Tauri Design Editor With shadcn + JSON Render

## Summary

Build a standalone [Tauri 2](https://v2.tauri.app/) desktop app with a custom high-performance DOM-based editor, [shadcn/ui](https://ui.shadcn.com/docs) for the app chrome and inspector, and [json-render](https://json-render.dev/) as the constrained JSON export/import layer rather than the primary editing engine.

Use the PocketPaw `design-builder` only as inspiration for panel layout and provider/catalog ideas, not as a host dependency. The useful parts to borrow are the left-panel/layers/props structure in [App.tsx](C:/Users/dila/Documents/GitHub/pocketpaw-/src/pocketpaw/extensions/builtin/design-builder/ui/src/App.tsx), the store patterns in [store.ts](C:/Users/dila/Documents/GitHub/pocketpaw-/src/pocketpaw/extensions/builtin/design-builder/ui/src/store.ts), and the provider split in [providers/index.ts](C:/Users/dila/Documents/GitHub/pocketpaw-/src/pocketpaw/extensions/builtin/design-builder/ui/src/providers/index.ts). Do not reuse its plugin SDK, monolithic app structure, or current canvas approach, because it has no true direct-manipulation editing.

## Key Changes

- App shell:
  - Tauri 2 + React + TypeScript + Vite.
  - shadcn open-code components for top bar, layer tree, inspector, dialogs, context menus, command palette, tabs, and settings.
  - Desktop-first file actions via Tauri dialogs and filesystem APIs: `New`, `Open`, `Save`, `Save As`, `Export JSON Render Spec`.

- Editor architecture:
  - Build a custom scene renderer for the canvas instead of using `@json-render/react` as the live editor surface.
  - Render the canvas with absolute-positioned DOM nodes, CSS transforms, a separate overlay for selection boxes/guides/handles, and `requestAnimationFrame`-driven pointer interactions.
  - Keep interaction state separate from persisted state so drag/resize/pan/zoom stay smooth and do not rerender the whole tree on every pointer move.
  - Support in v1: pan/zoom, single and marquee selection, multi-select, drag, resize, reorder, duplicate, delete, keyboard nudge, copy/paste, group/ungroup, undo/redo, snap guides, and layer locking/hiding.

- Document model and interfaces:
  - Introduce a `DesignDocument` as the editor’s saved format.
  - `DesignDocument` contains:
    - `scene`: editor-only metadata such as bounds, transforms, z-order, grouping, lock/hidden flags, and canvas settings.
    - `elements`: normalized component nodes keyed by id with `type`, `props`, `children`, and layout mode.
    - `meta`: theme, viewport, selection defaults, version.
  - Each node id is stable across editor state and exported JSON Render spec.
  - Add an importer/exporter layer:
    - `DesignDocument -> JsonRenderSpec`
    - `JsonRenderSpec -> DesignDocument`
  - Export pure JSON Render as a separate action; do not make the raw renderer spec the only internal state, because it cannot cleanly represent Figma-style transforms and editor metadata.

- shadcn + json-render integration:
  - Use shadcn’s “open code” model for the actual editor UI and preview components, not as an opaque package dependency for everything.
  - Define a curated component subset for the editor first: `Frame`, `Stack/Flex`, `Grid`, `Text`, `Heading`, `Image`, `Button`, `Input`, `Textarea`, `Card`, `Tabs`, `Dialog`, `Sheet`, `Table`, `Badge`, `Avatar`, `Separator`.
  - Create one shared component registry that drives:
    - the palette
    - inspector prop schemas
    - AI prompt generation
    - JSON Render export
    - preview rendering
  - Keep AI generation and json-render export behind this registry so every generated/exported node stays within the supported component set.

- AI and generation flow:
  - Preserve the good idea from the PocketPaw builder: AI generates structured JSON against a catalog.
  - In this app, AI should return either:
    - a full `JsonRenderSpec` for blank-canvas generation, or
    - a patch format (`add/update/move/remove`) against existing node ids for iterative edits.
  - Apply AI output through the same command/reducer pipeline as manual edits so undo/redo and validation remain consistent.

- Project structure:
  - `src/features/editor/model`: document types, serializers, import/export.
  - `src/features/editor/state`: Zustand slices, command stack, selection/history.
  - `src/features/editor/canvas`: viewport, scene renderer, overlay, interaction engine.
  - `src/features/editor/components`: shared preview components and prop schemas.
  - `src/features/editor/ui`: layers, inspector, palette, toolbar, menus.
  - `src-tauri/src`: file commands, recent files, app menus, native shortcuts.

## Test Plan

- Unit tests:
  - import/export round-trip between `DesignDocument` and `JsonRenderSpec`
  - move/resize/group/ungroup reducers
  - snapping, alignment, bounds math, z-order changes
  - undo/redo correctness after mixed manual + AI edits

- Integration/UI tests:
  - create component from palette, drag it, resize it, edit props, and verify layers + inspector stay in sync
  - multi-select, duplicate, delete, copy/paste, and keyboard nudge
  - save/open/export flows and invalid-file handling
  - pan/zoom and selection overlay stability on large documents

- Desktop smoke tests:
  - Tauri build and open/save/export on Windows first
  - shortcut parity for Ctrl/Cmd where relevant
  - one large-canvas performance smoke test with many nodes

## Assumptions And Defaults

- Standalone app only; PocketPaw is reference material, not a dependency.
- Tauri desktop is the first runtime target.
- The editor is “deep Figma-like” for UI layout/editing, but not a full vector tool.
- No pen tool, boolean ops, multiplayer, comments, or prototyping wires in v1.
- Single primary frame/page in v1, but the scene model must allow multi-artboard later.
- Custom DOM/canvas interaction layer is the default choice; do not use tldraw or craft.js as the primary editor engine unless performance or scope proves the custom path unworkable.
- JSON is the primary saved/exported format; code export is secondary and can follow after the editor core is stable.
