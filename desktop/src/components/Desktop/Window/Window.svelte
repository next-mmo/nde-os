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
  import { toggleFullscreen as toggleFullscreenState, closeWindow } from "🍎/state/desktop.svelte";
  import { activeWindow, currentBrowserUrl, focusWindow, loadWindowGeometry, saveWindowGeometry, type DesktopWindow } from "🍎/state/desktop.svelte";
  import TrafficLights from "🍎/components/Desktop/Window/TrafficLights.svelte";
  import AppNexus from "🍎/components/apps/AppNexus.svelte";
  import ContextMenu, { type ContextMenuItem } from "🍎/components/Desktop/ContextMenu.svelte";

  interface Props {
    window: DesktopWindow;
  }

  let { window }: Props = $props();

  let draggingEnabled = $state(true);
  let isDragging = $state(false);
  let isResizing = $state(false);
  let windowEl = $state<HTMLElement>();

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
      persistGeometry();
      document.removeEventListener("pointermove", onMove);
      document.removeEventListener("pointerup", onUp);
    };

    document.addEventListener("pointermove", onMove);
    document.addEventListener("pointerup", onUp);
  }

  let lastTitleBarClick = 0;
  function handleTitleBarPointerDown(event: PointerEvent) {
    const now = Date.now();
    if (now - lastTitleBarClick < 350) {
      event.preventDefault();
      event.stopPropagation();
      if (window.expandable) {
        toggleFullscreenState(window.id);
      }
      lastTitleBarClick = 0;
    } else {
      lastTitleBarClick = now;
    }
  }

  const dragDisabled = Compartment.of(() => disabled(window.fullscreen || !draggingEnabled));
  const isActive = $derived(activeWindow()?.id === window.id);

  const savedGeo = loadWindowGeometry(window.app_id);
  if (savedGeo) {
    window.width = savedGeo.width;
    window.height = savedGeo.height;
  }

  const defaultPosition = () => {
    const vw = globalThis.innerWidth ?? 1280;
    const vh = globalThis.innerHeight ?? 800;
    return {
      x: Math.max(24, Math.round((vw - window.width) / 2)),
      y: Math.max(30, Math.round((vh - window.height) / 2 * 0.6)),
    };
  };

  function persistGeometry() {
    if (!windowEl) return;
    const transform = windowEl.style.transform ?? "";
    const match = transform.match(/translate\((-?[\d.]+)px,\s*(-?[\d.]+)px\)/);
    const x = match ? parseFloat(match[1]) : 0;
    const y = match ? parseFloat(match[2]) : 0;
    saveWindowGeometry(window.app_id, {
      x, y,
      width: window.width,
      height: window.height,
      fullscreen: window.fullscreen,
    });
  }

  $effect(() => {
    if (!windowEl) return;
    draggingEnabled = !window.fullscreen;
  });

  const windowTitle = $derived(
    window.app_id === "browser" && window.browser ? currentBrowserUrl(window).replace(/^https?:\/\//, "") || "Browser" : window.title,
  );

  let windowCtx = $state<{ x: number; y: number } | null>(null);

  function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    windowCtx = { x: e.clientX, y: e.clientY };
  }

  const windowMenuItems = $derived<ContextMenuItem[]>([
    { kind: "action", icon: "⤢", label: window.fullscreen ? "Exit Full Screen" : "Enter Full Screen", action: () => { toggleFullscreenState(window.id); windowCtx = null; } },
    { kind: "divider" },
    { kind: "action", icon: "🔴", label: "Close Window", action: () => { closeWindow(window.id); windowCtx = null; } },
  ]);
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<section
  class="absolute grid grid-rows-[auto_1fr] rounded-xl overflow-hidden border border-black/10 dark:border-white/10 bg-white/50 dark:bg-gray-800/50 backdrop-blur-[26px] shadow-[0_16px_36px_rgba(0,0,0,0.22),0_48px_96px_rgba(0,0,0,0.18)] transition-shadow {isActive ? 'shadow-[0_20px_44px_rgba(0,0,0,0.3),0_56px_120px_rgba(0,0,0,0.24)]! ring-1 ring-black/5 dark:ring-white/10' : ''} {window.fullscreen ? 'inset-0! w-full! h-full! rounded-none! translate-none!' : ''} {window.minimized ? 'opacity-0 pointer-events-none invisible' : 'opacity-100 visible pointer-events-auto'}"
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
      onDragEnd: () => { isDragging = false; persistGeometry(); },
    }),
  ])}
>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <header class="window-drag-handle grid grid-cols-[auto_1fr_auto] items-center gap-4 px-4 pt-[0.9rem] pb-[0.7rem] bg-linear-to-b from-white/90 to-white/60 dark:from-gray-700/90 dark:to-gray-800/70 border-b border-black/10 dark:border-white/10" onpointerdown={handleTitleBarPointerDown} oncontextmenu={handleContextMenu}>
    <TrafficLights {window} />
    <div class="grid justify-items-center text-center gap-[0.08rem]">
      <strong class="text-[0.9rem] font-semibold text-gray-900 dark:text-gray-100">{windowTitle}</strong>
      {#if window.session_id}
        <span class="text-[0.72rem] text-gray-500 dark:text-gray-400">localhost session</span>
      {/if}
    </div>
    <div class="w-8"></div>
  </header>

  <div class="min-h-0 overflow-hidden bg-linear-to-b from-white/75 to-white/90 dark:from-gray-800/95 dark:to-gray-900/95 {isDragging || isResizing ? 'pointer-events-none' : ''}">
    <AppNexus {window} />
  </div>

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  {#if window.resizable && !window.fullscreen}
    <div class="absolute z-20 cursor-ns-resize -top-[3px] left-28 right-2 h-[6px]" onpointerdown={(e: PointerEvent) => startResize(e, 'n')}></div>
    <div class="absolute z-20 cursor-ns-resize -bottom-[3px] left-2 right-2 h-[6px]" onpointerdown={(e: PointerEvent) => startResize(e, 's')}></div>
    <div class="absolute z-20 cursor-ew-resize -right-[3px] top-2 bottom-2 w-[6px]" onpointerdown={(e: PointerEvent) => startResize(e, 'e')}></div>
    <div class="absolute z-20 cursor-ew-resize -left-[3px] top-12 bottom-2 w-[6px]" onpointerdown={(e: PointerEvent) => startResize(e, 'w')}></div>
    <div class="absolute z-20 cursor-nesw-resize -top-[3px] -right-[3px] w-[14px] h-[14px]" onpointerdown={(e: PointerEvent) => startResize(e, 'ne')}></div>
    <div class="absolute z-20 cursor-nwse-resize -top-[3px] left-[100px] w-[14px] h-[14px]" onpointerdown={(e: PointerEvent) => startResize(e, 'nw')}></div>
    <div class="absolute z-20 cursor-nwse-resize -bottom-[3px] -right-[3px] w-[14px] h-[14px]" onpointerdown={(e: PointerEvent) => startResize(e, 'se')}></div>
    <div class="absolute z-20 cursor-nesw-resize -bottom-[3px] -left-[3px] w-[14px] h-[14px]" onpointerdown={(e: PointerEvent) => startResize(e, 'sw')}></div>
  {/if}

  {#if windowCtx}
    <ContextMenu x={windowCtx.x} y={windowCtx.y} items={windowMenuItems} onclose={() => (windowCtx = null)} />
  {/if}
</section>
