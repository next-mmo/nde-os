# 🐛 FreeCut Preview: Vertical/portrait video appears zoomed-in, can't see full frame

**Status:** 🔴 plan  
**Priority:** High  
**Component:** `desktop/src/components/apps/FreeCut/FreeCut.svelte`

## Problem

When playing or scrubbing a **vertical/portrait** video (e.g. 9:16 TikTok, Reels, or tall interview footage), the preview panel shows the video **zoomed into the center** — only a cropped portion of the frame is visible. The user cannot see the full video content.

**Expected:** The entire video frame should be visible inside the preview area, letterboxed/pillarboxed as needed.  
**Actual:** The video appears zoomed in (center-cropped), cutting off the top/bottom or sides.

## Root Cause Analysis

**File:** `FreeCut.svelte` — lines ~2794–2802

The timeline preview container uses this sizing strategy:

```svelte
<div
  class="border border-white/5 rounded-sm bg-black shadow-2xl overflow-hidden flex items-center justify-center relative"
  style="aspect-ratio: {currentProject.metadata.width} / {currentProject.metadata.height}; height: 100%; max-width: 100%; container-type: size;"
>
```

The issue is `height: 100%` — this forces the container to fill the entire vertical space of the preview area. For **portrait videos** (e.g. 1080×1920), the aspect-ratio constraint means the resulting width would exceed the container, but `max-width: 100%` clamps it. However the inner composition layer:

```svelte
<div 
  class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 origin-center"
  style="width: {currentProject.metadata.width}px; height: {currentProject.metadata.height}px; transform: scale(min(calc(100cqw / {width}), calc(100cqh / {height})));"
>
```

…uses `container query` units (`100cqw`, `100cqh`) to scale the native-resolution layer down to fit. If the outer container overflows or is clipped by `overflow-hidden`, the scale calculation can be wrong, making the video appear zoomed-in when the aspect ratio doesn't match a landscape 16:9.

## Fix Approach

Change the preview container sizing to properly constrain **both** dimensions:

```svelte
<div
  style="
    aspect-ratio: {currentProject.metadata.width} / {currentProject.metadata.height};
    max-height: 100%;
    max-width: 100%;
    container-type: size;
  "
>
```

Using `max-height` + `max-width` (instead of `height: 100%`) lets the browser shrink the container to fit whichever dimension is limiting, ensuring the full frame is always visible regardless of portrait or landscape.

## Acceptance Criteria

- [ ] Portrait video (9:16) shows full frame in preview (pillarboxed with black bars on sides)
- [ ] Landscape video (16:9) still fills preview correctly
- [ ] Square video (1:1) shows full frame correctly
- [ ] Preview DOM overlays (text, images, video layers) align correctly after fix
- [ ] Timecode overlay at bottom of preview remains properly positioned
- [ ] No regression in playback or scrubbing behavior

## Files Likely Affected

- `desktop/src/components/apps/FreeCut/FreeCut.svelte` (lines ~2794–2802)

## Reproduction Steps

1. Open FreeCut in NDE-OS
2. Import a vertical/portrait video (e.g. 1080×1920 or 9:16 aspect ratio)
3. Observe the preview panel — the video appears cropped/zoomed rather than showing the full frame

## Screenshot

See attached screenshot showing the issue — only the center portion of a portrait video is visible, with the subject's face zoomed in and text cut off at edges.
