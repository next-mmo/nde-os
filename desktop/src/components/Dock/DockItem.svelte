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
  let fallbackEl = $state<HTMLDivElement>();
  let distance = $state(distanceLimit + 1);
  let srcFallback = $state(false);

  const getWidth = interpolate(distanceInput, widthOutput);
  const width = $derived(getWidth(distance));

  // Track whether mouse is actively over the dock (mouse_x !== null)
  // Use a faster transition when mouse enters, slightly slower when leaving for smoothness
  const isHovering = $derived(mouse_x !== null);

  $effect(() => {
    const el = srcFallback ? fallbackEl : imageEl;
    if (!el || mouse_x === null) {
      distance = distanceLimit + 1;
      return;
    }
    const rect = el.getBoundingClientRect();
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

<button
  class="relative flex flex-col items-center justify-end gap-1 group/dockitem p-0 bg-transparent border-none appearance-none outline-none focus-visible:ring-2 focus-visible:ring-blue-500 rounded-2xl"
  aria-label={label}
  onclick={handleClick}
>
  <p class="absolute bottom-[calc(100%+0.85rem)] whitespace-nowrap m-0 hidden group-hover/dockitem:block group-focus-visible/dockitem:block px-3 py-2 rounded-[0.7rem] bg-white/90 dark:bg-gray-800/90 backdrop-blur-md border border-black/10 dark:border-white/10 shadow-xl text-sm font-medium z-50 text-black dark:text-white">
    {label}
  </p>
  {#if !srcFallback}
    <img
      class="will-change-[width] drop-shadow-md"
      bind:this={imageEl}
      src="/app-icons/{app_id}/256.webp"
      alt=""
      style:width="{width / 16}rem"
      style:transition="width {isHovering ? '0.15s' : '0.35s'} cubic-bezier(0.32, 0.72, 0, 1)"
      onerror={() => srcFallback = true}
    />
  {:else}
    <div
      class="will-change-[width,height] rounded-[20%] bg-gradient-to-br from-blue-500/80 to-blue-700/80 grid place-items-center text-white font-bold text-xl shadow-md border border-white/20"
      bind:this={fallbackEl}
      style:width="{width / 16}rem"
      style:height="{width / 16}rem"
      style:transition="width {isHovering ? '0.15s' : '0.35s'} cubic-bezier(0.32, 0.72, 0, 1), height {isHovering ? '0.15s' : '0.35s'} cubic-bezier(0.32, 0.72, 0, 1)"
    >
      {label.slice(0, 2).toUpperCase()}
    </div>
  {/if}
  <div class="w-1 h-1 rounded-full bg-black/60 dark:bg-white/60 transition-opacity duration-200" style:opacity={isDockAppOpen(app_id as StaticAppID) ? 1 : 0}></div>
</button>
