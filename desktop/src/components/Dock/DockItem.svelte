<svelte:options runes={true} />

<script lang="ts">
  import { interpolate } from "popmotion";
  import { isDockAppOpen, openStaticApp, focusWindow, openSessionInDashboard, sessionForApp, windowForApp, type StaticAppID } from "🍎/state/desktop.svelte";
  import { apps_config } from "🍎/configs/apps/apps-config";

  const baseWidth = 58;
  const distanceLimit = baseWidth * 5;
  const distanceInput = [
    -distanceLimit,
    -distanceLimit / 1.25,
    -distanceLimit / 2,
    0,
    distanceLimit / 2,
    distanceLimit / 1.25,
    distanceLimit,
  ];
  const widthOutput = [
    baseWidth,
    baseWidth * 1.08,
    baseWidth * 1.32,
    baseWidth * 1.82,
    baseWidth * 1.32,
    baseWidth * 1.08,
    baseWidth,
  ];

  interface Props {
    app_id: string;
    mouse_x: number | null;
  }

  let { app_id, mouse_x }: Props = $props();

  let imageEl = $state<HTMLImageElement>();
  let distance = $state(distanceLimit + 1);
  let srcFallback = $state(false);

  const getWidth = interpolate(distanceInput, widthOutput);
  const width = $derived(getWidth(distance));

  $effect(() => {
    if (!imageEl || mouse_x === null) {
      distance = distanceLimit + 1;
      return;
    }
    const rect = imageEl.getBoundingClientRect();
    distance = mouse_x - (rect.left + rect.width / 2);
  });

  const session = $derived(sessionForApp(app_id));
  const label = $derived(session?.title || (app_id === "ai-launcher" ? "AI Launcher" : app_id.replace(/-/g, " ")));

  function handleClick() {
    if (session) {
      if (session.window_id) {
        focusWindow(session.window_id);
      } else {
        openSessionInDashboard(session.id);
      }
    } else if (app_id in apps_config) {
      openStaticApp(app_id as StaticAppID);
    } else {
      const window = windowForApp(app_id as any);
      if (window) focusWindow(window.id);
    }
  }
</script>

<button class="dock-item" aria-label={label} onclick={handleClick}>
  <p class="tooltip">{label}</p>
  {#if !srcFallback}
    <img bind:this={imageEl} src="/app-icons/{app_id}/256.webp" alt="" style:width={`${width / 16}rem`} onerror={() => srcFallback = true} />
  {:else}
    <div class="fallback-icon" style:width={`${width / 16}rem`} style:height={`${width / 16}rem`}>
      {label.slice(0, 2).toUpperCase()}
    </div>
  {/if}
  <div class="dot" style:opacity={isDockAppOpen(app_id as StaticAppID) ? 1 : 0}></div>
</button>

<style>
  .dock-item {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-end;
    gap: 0.15rem;
  }

  img {
    width: 3.625rem;
    will-change: width;
  }

  .fallback-icon {
    width: 3.625rem;
    height: 3.625rem;
    will-change: width, height;
    border-radius: 20%;
    background: linear-gradient(135deg, hsla(var(--system-color-primary-hsl) / 0.8), hsla(206 72% 44% / 0.8));
    display: grid;
    place-items: center;
    color: white;
    font-weight: bold;
    font-size: 1.2rem;
    box-shadow: 0 4px 12px hsla(0 0% 0% / 0.2);
    border: 1px solid hsla(0 0% 100% / 0.2);
  }

  .tooltip {
    position: absolute;
    bottom: calc(100% + 0.85rem);
    white-space: nowrap;
    margin: 0;
    display: none;
    padding: 0.5rem 0.75rem;
    border-radius: 0.7rem;
    background: hsla(var(--system-color-light-hsl) / 0.88);
    border: 1px solid var(--system-color-border);
    box-shadow: 0 12px 32px hsla(220 30% 10% / 0.2);
    font-size: 0.8rem;
    z-index: 100;
  }

  .dock-item:hover .tooltip,
  .dock-item:focus-visible .tooltip {
    display: block;
  }

  .dot {
    width: 0.28rem;
    height: 0.28rem;
    border-radius: 999px;
    background: var(--system-color-dark);
  }
</style>
