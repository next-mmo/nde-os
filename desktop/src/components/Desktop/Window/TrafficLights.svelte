<svelte:options runes={true} />

<script lang="ts">
  import { activeWindow, closeWindow, minimizeWindow, toggleFullscreen, type DesktopWindow } from "🍎/state/desktop.svelte";

  interface Props {
    window: DesktopWindow;
  }

  let { window }: Props = $props();

  const isFocused = $derived(activeWindow()?.id === window.id);

  function handleClick(event: MouseEvent, action: () => void) {
    event.stopPropagation();
    event.preventDefault();
    action();
  }
</script>

<div class="traffic-lights" class:unfocused={!isFocused}>
  <button
    class="tl-btn close"
    class:disabled={!window.closable}
    aria-label="Close window"
    disabled={!window.closable}
    onclick={(e) => handleClick(e, () => closeWindow(window.id))}
  >
    <span class="dot"></span>
  </button>
  <button class="tl-btn minimize" aria-label="Minimize window" onclick={(e) => handleClick(e, () => minimizeWindow(window.id))}>
    <span class="dot"></span>
  </button>
  <button class="tl-btn fullscreen" aria-label="Toggle fullscreen" onclick={(e) => handleClick(e, () => toggleFullscreen(window.id))}>
    <span class="dot"></span>
  </button>
</div>

<style>
  .traffic-lights {
    display: flex;
    gap: 0;
    position: relative;
    z-index: 10;
  }

  .tl-btn {
    display: grid;
    place-items: center;
    width: 1.75rem;
    height: 1.75rem;
    border-radius: 50%;
    background: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
    -webkit-app-region: no-drag;
    position: relative;
  }

  .dot {
    display: block;
    width: 0.8rem;
    height: 0.8rem;
    border-radius: 50%;
    box-shadow: 0 0 0 0.5px hsla(0 0% 0% / 0.22);
    pointer-events: none;
    transition: transform 0.12s ease;
  }

  .tl-btn:hover .dot {
    transform: scale(1.15);
  }

  .tl-btn:active .dot {
    transform: scale(0.9);
  }

  .close .dot {
    background: #ff5f57;
  }

  .close.disabled .dot {
    background: #bfc2c8 !important;
    cursor: default;
  }

  .close.disabled {
    cursor: default;
    pointer-events: none;
  }

  .minimize .dot {
    background: #febc2e;
  }

  .fullscreen .dot {
    background: #28c840;
  }

  .unfocused .dot {
    background: #bfc2c8;
  }

  .unfocused .tl-btn:hover .dot {
    background: inherit;
  }

  .unfocused .close:hover .dot {
    background: #ff5f57;
  }

  .unfocused .minimize:hover .dot {
    background: #febc2e;
  }

  .unfocused .fullscreen:hover .dot {
    background: #28c840;
  }
</style>
