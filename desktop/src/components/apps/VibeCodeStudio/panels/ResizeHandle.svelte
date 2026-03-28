<script lang="ts">
  interface Props {
    onResize: (delta: number) => void;
  }

  let { onResize }: Props = $props();
  let isDragging = $state(false);
  let isHovered = $state(false);
  let startX = 0;

  function onMouseDown(e: MouseEvent) {
    e.preventDefault();
    isDragging = true;
    startX = e.clientX;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  }

  function onMouseMove(e: MouseEvent) {
    if (!isDragging) return;
    const delta = e.clientX - startX;
    startX = e.clientX;
    onResize(delta);
  }

  function onMouseUp() {
    if (!isDragging) return;
    isDragging = false;
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  }
</script>

<svelte:window onmousemove={onMouseMove} onmouseup={onMouseUp} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="w-1 shrink-0 cursor-col-resize relative flex items-center justify-center transition-colors
    {isDragging ? 'bg-indigo-500/60' : isHovered ? 'bg-indigo-500/40' : 'bg-transparent'}"
  onmousedown={onMouseDown}
  onmouseenter={() => isHovered = true}
  onmouseleave={() => { if (!isDragging) isHovered = false }}
>
  <!-- Wider hit area -->
  <div class="absolute inset-y-0 -left-1.5 -right-1.5 z-10"></div>

  <!-- Grip dots indicator — visible on hover/drag -->
  {#if isHovered || isDragging}
    <div class="absolute z-20 flex flex-col gap-[3px] pointer-events-none">
      <div class="w-[3px] h-[3px] rounded-full bg-white/60"></div>
      <div class="w-[3px] h-[3px] rounded-full bg-white/60"></div>
      <div class="w-[3px] h-[3px] rounded-full bg-white/60"></div>
      <div class="w-[3px] h-[3px] rounded-full bg-white/60"></div>
      <div class="w-[3px] h-[3px] rounded-full bg-white/60"></div>
    </div>
  {/if}
</div>
