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
    position,
  } from "@neodrag/svelte";
  import { activeWindow, currentBrowserUrl, focusWindow, type DesktopWindow } from "🍎/state/desktop.svelte";
  import TrafficLights from "🍎/components/Desktop/Window/TrafficLights.svelte";
  import AppNexus from "🍎/components/apps/AppNexus.svelte";

  interface Props {
    window: DesktopWindow;
  }

  let { window }: Props = $props();

  let draggingEnabled = $state(true);
  let windowEl = $state<HTMLElement>();
  let previousTransform = $state("");

  const dragDisabled = Compartment.of(() => disabled(window.fullscreen || !draggingEnabled));
  const isActive = $derived(activeWindow()?.id === window.id);
  const defaultPosition = () => ({
    x: ((window.id.length * 53) % 280) + 72,
    y: ((window.id.length * 37) % 120) + 62,
  });

  $effect(() => {
    if (!windowEl) return;
    if (window.fullscreen) {
      draggingEnabled = false;
      previousTransform = windowEl.style.transform;
      windowEl.style.transform = "translate(0px, 0px)";
      windowEl.style.width = "calc(100vw - 48px)";
      windowEl.style.height = "calc(100vh - 126px)";
    } else {
      draggingEnabled = true;
      if (previousTransform) {
        windowEl.style.transform = previousTransform;
      }
      windowEl.style.width = `${window.width}px`;
      windowEl.style.height = `${window.height}px`;
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
  onclick={() => focusWindow(window.id)}
  onkeydown={(event) => {
    if (event.key === "Enter" || event.key === " ") {
      focusWindow(window.id);
    }
  }}
  {@attach draggable(() => [
    controls({ allow: ControlFrom.selector(".window-drag-handle") }),
    bounds(BoundsFrom.viewport({ top: 30, left: -5000, right: -5000, bottom: -5000 })),
    position({ default: defaultPosition() }),
    dragDisabled,
  ])}
>
  <header class="window-chrome window-drag-handle">
    <TrafficLights {window} />
    <div class="window-title">
      <strong>{windowTitle}</strong>
      {#if window.session_id}
        <span>localhost session</span>
      {/if}
    </div>
    <div class="window-accent"></div>
  </header>

  <div class="window-body">
    <AppNexus {window} />
  </div>
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
    top: 0;
    left: 0;
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

  :global(body.dark) .window-body {
    background: linear-gradient(180deg, hsla(225 16% 17% / 0.96), hsla(225 16% 14% / 0.96));
  }
</style>
