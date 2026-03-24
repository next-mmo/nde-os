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

<div class="flex items-center relative z-30 group/tl {isFocused ? '' : 'opacity-70 grayscale-[0.3] hover:grayscale-0 hover:opacity-100'}" style="-webkit-app-region: no-drag;">
  <button
    class="grid place-items-center p-[5px] cursor-pointer relative hover:brightness-90 transition-[filter] bg-transparent border-none {window.closable ? '' : 'cursor-default pointer-events-none'}"
    style="-webkit-app-region: no-drag;"
    aria-label="Close window"
    data-testid="traffic-close"
    disabled={!window.closable}
    onclick={(e) => handleClick(e, () => closeWindow(window.id))}
  >
    <span class="grid place-items-center w-3 h-3 rounded-full border-[0.5px] border-black/10 shadow-[inset_0_1px_1px_rgba(255,255,255,0.2)] pointer-events-none" style="background-color: {window.closable ? '#ff5f57' : '#bfc2c8'};">
      <svg class="w-[6px] h-[6px] opacity-0 group-hover/tl:opacity-100 transition-opacity" viewBox="0 0 12 12" fill="none" stroke="rgba(0,0,0,0.5)" stroke-width="2" stroke-linecap="round"><line x1="2" y1="2" x2="10" y2="10"/><line x1="10" y1="2" x2="2" y2="10"/></svg>
    </span>
  </button>
  <button
    class="grid place-items-center p-[5px] cursor-pointer relative hover:brightness-90 transition-[filter] bg-transparent border-none"
    style="-webkit-app-region: no-drag;"
    aria-label="Minimize window"
    data-testid="traffic-minimize"
    onclick={(e) => handleClick(e, () => minimizeWindow(window.id))}
  >
    <span class="grid place-items-center w-3 h-3 rounded-full border-[0.5px] border-black/10 shadow-[inset_0_1px_1px_rgba(255,255,255,0.2)] pointer-events-none" style="background-color: #febc2e;">
      <svg class="w-[6px] h-[6px] opacity-0 group-hover/tl:opacity-100 transition-opacity" viewBox="0 0 12 12" fill="none" stroke="rgba(0,0,0,0.5)" stroke-width="2" stroke-linecap="round"><line x1="1" y1="6" x2="11" y2="6"/></svg>
    </span>
  </button>
  <button
    class="grid place-items-center p-[5px] cursor-pointer relative hover:brightness-90 transition-[filter] bg-transparent border-none"
    style="-webkit-app-region: no-drag;"
    aria-label="Toggle fullscreen"
    data-testid="traffic-fullscreen"
    onclick={(e) => handleClick(e, () => toggleFullscreen(window.id))}
  >
    <span class="grid place-items-center w-3 h-3 rounded-full border-[0.5px] border-black/10 shadow-[inset_0_1px_1px_rgba(255,255,255,0.2)] pointer-events-none" style="background-color: #28c840;">
      <svg class="w-[6px] h-[6px] opacity-0 group-hover/tl:opacity-100 transition-opacity" viewBox="0 0 12 12" fill="none" stroke="rgba(0,0,0,0.5)" stroke-width="1.5"><path d="M2 3.5V2h1.5M10 3.5V2H8.5M2 8.5V10h1.5M10 8.5V10H8.5"/></svg>
    </span>
  </button>
</div>
