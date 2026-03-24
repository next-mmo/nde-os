<svelte:options runes={true} />

<script lang="ts">
  import { onMount } from "svelte";

  export type ContextMenuItem =
    | { kind: "action"; icon: string; label: string; action: () => void; shortcut?: string; disabled?: boolean }
    | { kind: "divider" };

  interface Props {
    x: number;
    y: number;
    items: ContextMenuItem[];
    onclose: () => void;
  }

  let { x, y, items, onclose }: Props = $props();

  let menuEl = $state<HTMLDivElement>();
  let flipX = $state(0);
  let flipY = $state(0);

  const adjustedX = $derived(x + flipX);
  const adjustedY = $derived(y + flipY);

  onMount(() => {
    if (!menuEl) return;
    const rect = menuEl.getBoundingClientRect();
    const vw = window.innerWidth;
    const vh = window.innerHeight;

    // Flip horizontally if overflowing right
    if (x + rect.width > vw - 8) {
      flipX = -rect.width;
    }
    // Flip vertically if overflowing bottom
    if (y + rect.height > vh - 8) {
      flipY = -rect.height;
    }
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onclose();
    }
  }

  function handleItemClick(item: ContextMenuItem) {
    if (item.kind !== "action" || item.disabled) return;
    item.action();
    onclose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- Backdrop to capture clicks outside -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="fixed inset-0 z-9998"
  onclick={onclose}
  oncontextmenu={(e) => { e.preventDefault(); onclose(); }}
  onkeydown={undefined}
></div>

<!-- Menu panel -->
<div
  bind:this={menuEl}
  class="fixed z-9999 min-w-[220px] py-1.5 rounded-xl bg-white/95 dark:bg-[#2a2a2c]/95 backdrop-blur-2xl border border-black/12 dark:border-white/12 shadow-[0_12px_48px_rgba(0,0,0,0.28),0_2px_8px_rgba(0,0,0,0.12)]"
  style="left: {adjustedX}px; top: {adjustedY}px;"
  role="menu"
  data-testid="context-menu"
>
  {#each items as item}
    {#if item.kind === "divider"}
      <div class="h-px mx-3 my-1 bg-black/8 dark:bg-white/10" role="separator"></div>
    {:else}
      <button
        class="flex items-center gap-2.5 w-full px-3 py-[5px] text-[13px] text-left bg-transparent border-none cursor-default transition-colors rounded-none
          {item.disabled ? 'text-gray-400 dark:text-gray-600 pointer-events-none' : 'text-gray-900 dark:text-gray-100 hover:bg-blue-500 hover:text-white dark:hover:bg-blue-500 dark:hover:text-white'}"
        role="menuitem"
        disabled={item.disabled}
        onclick={() => handleItemClick(item)}
      >
        <span class="w-5 text-center text-sm shrink-0">{item.icon}</span>
        <span class="flex-1">{item.label}</span>
        {#if item.shortcut}
          <span class="text-[11px] ml-4 opacity-50 shrink-0">{item.shortcut}</span>
        {/if}
      </button>
    {/if}
  {/each}
</div>
