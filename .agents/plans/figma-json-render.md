# Figma JSON Render Engine

> Figma + JSON-render: manual + LLM agent auto-generate JSON file

## Status: ✅ Implemented

## Architecture

```
Figma REST API ──→ figma-converter.ts ──→ FDocument JSON ──→ Renderer
LLM Agent ──→ llm-prompt.ts ──→ (paste output) ──→ FDocument JSON ──→ Renderer
Manual ──→ JSON Editor ──→ FDocument JSON ──→ Renderer
```

## Files

### Engine: `desktop/src/lib/figma-json/`
- `types.ts` — FDocument schema (6 node types, fills, strokes, shadows, auto-layout)
- `style-resolver.ts` — FNode → CSS (fills→background, layout→flexbox, effects→shadows)
- `JsonRenderer.svelte` — Recursive Svelte 5 component (svelte:self)
- `DocumentRenderer.svelte` — Canvas wrapper with scale
- `figma-converter.ts` — Figma REST API → FDocument (with live fetch)
- `llm-prompt.ts` — System + user prompt for LLM generation
- `sample-document.ts` — Demo dark card UI
- `index.ts` — Barrel export

### App: `desktop/src/components/apps/FigmaRender/`
- `FigmaRender.svelte` — 4-tab app (Preview + JSON Editor + Figma Import + LLM Agent)

### Integration
- `apps-config.ts` — registered as `figma-render` (960×680)
- `AppNexus.svelte` — lazy-load route
- `public/app-icons/figma-render/256.webp` — dock icon
