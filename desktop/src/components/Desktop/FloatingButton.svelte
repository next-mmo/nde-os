<svelte:options runes={true} />

<script lang="ts">
  import { onMount } from "svelte";
  import { expandDesktop, collapseDesktop, saveFabPosition } from "🍎/state/desktop.svelte";

  onMount(() => {
    collapseDesktop();

    // Persist position periodically while collapsed
    const interval = setInterval(() => saveFabPosition(), 2000);
    return () => clearInterval(interval);
  });
</script>

<!-- Right-edge tab: draggable vertically, click to expand -->
<div class="tab-wrapper" data-tauri-drag-region>
  <button
    class="tab-btn"
    onclick={expandDesktop}
    aria-label="Open AI Launcher"
    data-testid="floating-fab"
    style="-webkit-app-region: no-drag;"
  >
    <span class="tab-arrow" style="-webkit-app-region: no-drag;">‹</span>
  </button>
</div>

<style>
  .tab-wrapper {
    position: fixed;
    inset: 0;
    display: grid;
    place-items: center;
    background: linear-gradient(135deg, #5856d6, #af52de);
    border-radius: 8px 0 0 8px;
    cursor: grab;
    overflow: hidden;
  }

  .tab-wrapper:active {
    cursor: grabbing;
  }

  .tab-btn {
    width: 100%;
    height: 100%;
    border: none;
    cursor: pointer;
    background: transparent;
    display: grid;
    place-items: center;
    -webkit-app-region: no-drag;
    transition: background 0.15s;
  }

  .tab-btn:hover {
    background: rgba(255, 255, 255, 0.12);
  }

  .tab-btn:active {
    background: rgba(255, 255, 255, 0.06);
  }

  .tab-arrow {
    font-size: 20px;
    font-weight: 700;
    color: white;
    line-height: 1;
    pointer-events: none;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
  }
</style>
