<svelte:options runes={true} />

<script lang="ts">
  import { activeWindow, closeWindow, minimizeWindow, toggleFullscreen, type DesktopWindow } from "🍎/state/desktop.svelte";

  interface Props {
    window: DesktopWindow;
  }

  let { window }: Props = $props();

  const isFocused = $derived(activeWindow()?.id === window.id);
</script>

<div class="traffic-lights" class:unfocused={!isFocused}>
  <button class="close" aria-label="Close window" onclick={() => closeWindow(window.id)}></button>
  <button class="minimize" aria-label="Minimize window" onclick={() => minimizeWindow(window.id)}></button>
  <button class="fullscreen" aria-label="Toggle fullscreen" onclick={() => toggleFullscreen(window.id)}></button>
</div>

<style>
  .traffic-lights {
    display: flex;
    gap: 0.45rem;
  }

  button {
    width: 0.8rem;
    height: 0.8rem;
    border-radius: 50%;
    box-shadow: 0 0 0 0.5px hsla(0 0% 0% / 0.22);
  }

  .close {
    background: #ff5f57;
  }

  .minimize {
    background: #febc2e;
  }

  .fullscreen {
    background: #28c840;
  }

  .unfocused button {
    background: #bfc2c8;
  }
</style>
