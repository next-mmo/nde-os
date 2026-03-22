<svelte:options runes={true} />

<script lang="ts">
  import {
    bounds,
    BoundsFrom,
    Compartment,
    ControlFrom,
    controls,
    disabled,
    draggable,
    events,
    position,
  } from "@neodrag/svelte";
  import { toggleFullscreen as toggleFullscreenState } from "🍎/state/desktop.svelte";
  import { activeWindow, currentBrowserUrl, focusWindow, type DesktopWindow } from "🍎/state/desktop.svelte";
  import TrafficLights from "🍎/components/Desktop/Window/TrafficLights.svelte";
  import AppNexus from "🍎/components/apps/AppNexus.svelte";

  interface Props {
    window: DesktopWindow;
  }

  let { window }: Props = $props();

  let draggingEnabled = $state(true);
  let isDragging = $state(false);
  let isResizing = $state(false);
  let windowEl = $state<HTMLElement>();
  let previousTransform = $state("");

  const MIN_WIDTH = 320;
  const MIN_HEIGHT = 200;

  type ResizeDirection = "n" | "s" | "e" | "w" | "ne" | "nw" | "se" | "sw";

  function startResize(event: PointerEvent, direction: ResizeDirection) {
    if (window.fullscreen || !window.resizable) return;
    event.preventDefault();
    event.stopPropagation();
    isResizing = true;

    const startX = event.clientX;
    const startY = event.clientY;
    const startWidth = windowEl?.offsetWidth ?? window.width;
    const startHeight = windowEl?.offsetHeight ?? window.height;
    const startTransform = windowEl?.style.transform ?? "";
    const match = startTransform.match(/translate\((-?[\d.]+)px,\s*(-?[\d.]+)px\)/);
    const startLeft = match ? parseFloat(match[1]) : 0;
    const startTop = match ? parseFloat(match[2]) : 0;

    const onMove = (moveEvent: PointerEvent) => {
      const dx = moveEvent.clientX - startX;
      const dy = moveEvent.clientY - startY;
      let newWidth = startWidth;
      let newHeight = startHeight;
      let newLeft = startLeft;
      let newTop = startTop;

      if (direction.includes("e")) newWidth = Math.max(MIN_WIDTH, startWidth + dx);
      if (direction.includes("w")) {
        newWidth = Math.max(MIN_WIDTH, startWidth - dx);
        newLeft = startLeft + (startWidth - newWidth);
      }
      if (direction.includes("s")) newHeight = Math.max(MIN_HEIGHT, startHeight + dy);
      if (direction.includes("n")) {
        newHeight = Math.max(MIN_HEIGHT, startHeight - dy);
        newTop = startTop + (startHeight - newHeight);
      }

      if (windowEl) {
        windowEl.style.width = `${newWidth}px`;
        windowEl.style.height = `${newHeight}px`;
        windowEl.style.transform = `translate(${newLeft}px, ${newTop}px)`;
      }
      window.width = newWidth;
      window.height = newHeight;
    };

    const onUp = () => {
      isResizing = false;
      document.removeEventListener("pointermove", onMove);
      document.removeEventListener("pointerup", onUp);
    };

    document.addEventListener("pointermove", onMove);
    document.addEventListener("pointerup", onUp);
  }

  function handleTitleBarDblClick(event: MouseEvent) {
    event.preventDefault();
    event.stopPropagation();
    if (window.expandable) {
      toggleFullscreenState(window.id);
    }
  }

  const dragDisabled = Compartment.of(() => disabled(window.fullscreen || !draggingEnabled));
  const isActive = $derived(activeWindow()?.id === window.id);
  const defaultPosition = () => {
    const vw = globalThis.innerWidth ?? 1280;
    const offsetX = ((window.id.length * 53) % 120) - 60;
    const offsetY = ((window.id.length * 37) % 30);
    return {
      x: Math.max(24, Math.round((vw - window.width) / 2) + offsetX),
      y: 10 + offsetY,
    };
  };

  $effect(() => {
    if (!windowEl) return;
    if (window.fullscreen) {
      draggingEnabled = false;
    } else {
      draggingEnabled = true;
    }
  });

  const windowTitle = $derived(
    window.app_id === "browser" && window.browser ? currentBrowserUrl(window).replace(/^https?:\/\//, "") || "Browser" : window.title,
  );
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<section
  class="window"
  class:active={isActive}
  class:fullscreen={window.fullscreen}
  data-window={window.app_id}
  aria-label={window.title}
  tabindex="-1"
  bind:this={windowEl}
  style:width={`${window.width}px`}
  style:height={`${window.height}px`}
  style:z-index={window.z_index}
  onpointerdowncapture={() => focusWindow(window.id)}
  onkeydown={(event: KeyboardEvent) => {
    if (event.key === "Enter" || event.key === " ") {
      focusWindow(window.id);
    }
  }}
  {@attach draggable(() => [
    controls({ allow: ControlFrom.selector(".window-drag-handle") }),
    bounds(BoundsFrom.viewport({ top: 30, left: -5000, right: -5000, bottom: -5000 })),
    position({ default: defaultPosition() }),
    dragDisabled,
    events({
      onDragStart: () => { isDragging = true; },
      onDragEnd: () => { isDragging = false; },
    }),
  ])}
>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <header class="window-chrome window-drag-handle" ondblclick={handleTitleBarDblClick}>
    <TrafficLights {window} />
    <div class="window-title">
      <strong>{windowTitle}</strong>
      {#if window.session_id}
        <span>localhost session</span>
      {/if}
    </div>
    <div class="window-accent"></div>
  </header>

  <div class="window-body" class:dragging={isDragging} class:resizing={isResizing}>
    <AppNexus {window} />
  </div>

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  {#if window.resizable && !window.fullscreen}
    <div class="resize-handle resize-n" onpointerdown={(e: PointerEvent) => startResize(e, 'n')}></div>
    <div class="resize-handle resize-s" onpointerdown={(e: PointerEvent) => startResize(e, 's')}></div>
    <div class="resize-handle resize-e" onpointerdown={(e: PointerEvent) => startResize(e, 'e')}></div>
    <div class="resize-handle resize-w" onpointerdown={(e: PointerEvent) => startResize(e, 'w')}></div>
    <div class="resize-handle resize-ne" onpointerdown={(e: PointerEvent) => startResize(e, 'ne')}></div>
    <div class="resize-handle resize-nw" onpointerdown={(e: PointerEvent) => startResize(e, 'nw')}></div>
    <div class="resize-handle resize-se" onpointerdown={(e: PointerEvent) => startResize(e, 'se')}></div>
    <div class="resize-handle resize-sw" onpointerdown={(e: PointerEvent) => startResize(e, 'sw')}></div>
  {/if}
</section>

<style>
  .window {
    position: absolute;
    display: grid;
    grid-template-rows: auto 1fr;
    border-radius: var(--system-radius-window);
    overflow: hidden;
    border: 1px solid var(--system-color-border);
    background: var(--system-color-surface);
    box-shadow:
      0 16px 36px hsla(220 30% 10% / 0.22),
      0 48px 96px hsla(220 32% 8% / 0.18);
    backdrop-filter: blur(26px);
  }

  .window.active {
    box-shadow:
      0 20px 44px hsla(220 32% 10% / 0.3),
      0 56px 120px hsla(220 32% 8% / 0.24);
  }

  .window.fullscreen {
    top: 24px !important;
    left: 24px !important;
    width: calc(100vw - 48px) !important;
    height: calc(100vh - 126px) !important;
    transform: none !important;
  }

  .window-chrome {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 1rem;
    padding: 0.9rem 1rem 0.7rem;
    background: linear-gradient(180deg, hsla(var(--system-color-light-hsl) / 0.92), hsla(var(--system-color-light-hsl) / 0.6));
    border-bottom: 1px solid var(--system-color-border);
  }

  :global(body.dark) .window-chrome {
    background: linear-gradient(180deg, hsla(225 12% 24% / 0.96), hsla(225 12% 20% / 0.7));
  }

  .window-title {
    display: grid;
    justify-items: center;
    text-align: center;
    gap: 0.08rem;
  }

  .window-title strong {
    font-size: 0.9rem;
    font-weight: 600;
  }

  .window-title span {
    font-size: 0.72rem;
    color: var(--system-color-text-muted);
  }

  .window-accent {
    width: 2rem;
  }

  .window-body {
    min-height: 0;
    background: linear-gradient(180deg, hsla(var(--system-color-light-hsl) / 0.76), hsla(var(--system-color-light-hsl) / 0.92));
  }

  .window-body.dragging :global(iframe),
  .window-body.resizing :global(iframe) {
    pointer-events: none;
  }

  :global(body.dark) .window-body {
    background: linear-gradient(180deg, hsla(225 16% 17% / 0.96), hsla(225 16% 14% / 0.96));
  }

  /* ── Resize handles ── */
  .resize-handle {
    position: absolute;
    z-index: 20;
  }

  .resize-n {
    top: -3px; left: 8px; right: 8px; height: 6px;
    cursor: ns-resize;
  }
  .resize-s {
    bottom: -3px; left: 8px; right: 8px; height: 6px;
    cursor: ns-resize;
  }
  .resize-e {
    right: -3px; top: 8px; bottom: 8px; width: 6px;
    cursor: ew-resize;
  }
  .resize-w {
    left: -3px; top: 8px; bottom: 8px; width: 6px;
    cursor: ew-resize;
  }
  .resize-ne {
    top: -3px; right: -3px; width: 14px; height: 14px;
    cursor: nesw-resize;
  }
  .resize-nw {
    top: -3px; left: -3px; width: 14px; height: 14px;
    cursor: nwse-resize;
  }
  .resize-se {
    bottom: -3px; right: -3px; width: 14px; height: 14px;
    cursor: nwse-resize;
  }
  .resize-sw {
    bottom: -3px; left: -3px; width: 14px; height: 14px;
    cursor: nesw-resize;
  }
</style>
