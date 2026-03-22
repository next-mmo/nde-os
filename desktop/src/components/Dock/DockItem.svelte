<svelte:options runes={true} />

<script lang="ts">
  import { interpolate } from "popmotion";
  import { isDockAppOpen, openStaticApp, type StaticAppID } from "🍎/state/desktop.svelte";

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

  const label = $derived(app_id === "ai-launcher" ? "AI Launcher" : app_id.replace(/-/g, " "));
</script>

<button class="dock-item" aria-label={label} onclick={() => openStaticApp(app_id as StaticAppID)}>
  <p class="tooltip">{label}</p>
  <img bind:this={imageEl} src="/app-icons/{app_id}/256.webp" alt="" style:width={`${width / 16}rem`} />
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
