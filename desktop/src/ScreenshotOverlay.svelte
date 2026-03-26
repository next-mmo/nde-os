<script lang="ts">
  import { emit } from "@tauri-apps/api/event";

  let startX = $state(0);
  let startY = $state(0);
  let currentX = $state(0);
  let currentY = $state(0);
  let isDragging = $state(false);

  function handlePointerDown(e: PointerEvent) {
    startX = e.clientX;
    startY = e.clientY;
    currentX = e.clientX;
    currentY = e.clientY;
    isDragging = true;
  }

  function handlePointerMove(e: PointerEvent) {
    if (!isDragging) return;
    currentX = e.clientX;
    currentY = e.clientY;
  }

  async function handlePointerUp() {
    if (!isDragging) return;
    isDragging = false;
    
    const x = Math.min(startX, currentX);
    const y = Math.min(startY, currentY);
    const width = Math.abs(currentX - startX);
    const height = Math.abs(currentY - startY);

    if (width > 10 && height > 10) {
      await emit("screenshot-selected", { x, y, width, height });
    } else {
      await emit("screenshot-cancelled");
    }
  }

  async function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      await emit("screenshot-cancelled");
    }
  }

  let boxStyle = $derived(`left: ${Math.min(startX, currentX)}px; top: ${Math.min(startY, currentY)}px; width: ${Math.abs(currentX - startX)}px; height: ${Math.abs(currentY - startY)}px;`);
</script>

<svelte:window on:keydown={handleKeyDown} />

<main 
  class="w-screen h-screen bg-black/20 cursor-crosshair fixed inset-0 z-[9999]"
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerUp}
>
  {#if isDragging}
    <div 
      class="absolute border-2 border-primary bg-primary/20 pointer-events-none"
      style={boxStyle}
    ></div>
  {/if}
</main>
