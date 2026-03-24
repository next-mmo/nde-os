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

<div class="flex gap-2 relative z-10 group/tl {isFocused ? '' : 'opacity-70 grayscale-[0.3] hover:grayscale-0 hover:opacity-100'}">
  <button
    class="grid place-items-center w-3 h-3 rounded-full bg-transparent border-[0.5px] border-black/10 shadow-[inset_0_1px_1px_rgba(255,255,255,0.2)] p-0 cursor-pointer [-webkit-app-region:no-drag] relative {window.closable ? 'bg-[#ff5f57] hover:bg-[#ff5f57]' : 'bg-[#bfc2c8] cursor-default pointer-events-none'}"
    aria-label="Close window"
    disabled={!window.closable}
    onclick={(e) => handleClick(e, () => closeWindow(window.id))}
  >
    <!-- icon dot -->
  </button>
  <button 
    class="grid place-items-center w-3 h-3 rounded-full bg-[#febc2e] border-[0.5px] border-black/10 shadow-[inset_0_1px_1px_rgba(255,255,255,0.2)] p-0 cursor-pointer [-webkit-app-region:no-drag] relative" 
    aria-label="Minimize window" 
    onclick={(e) => handleClick(e, () => minimizeWindow(window.id))}
  >
  </button>
  <button 
    class="grid place-items-center w-3 h-3 rounded-full bg-[#28c840] border-[0.5px] border-black/10 shadow-[inset_0_1px_1px_rgba(255,255,255,0.2)] p-0 cursor-pointer [-webkit-app-region:no-drag] relative" 
    aria-label="Toggle fullscreen" 
    onclick={(e) => handleClick(e, () => toggleFullscreen(window.id))}
  >
  </button>
</div>
