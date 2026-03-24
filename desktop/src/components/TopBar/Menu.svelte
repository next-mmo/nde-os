<svelte:options runes={true} />

<script lang="ts">
  import type { MenuItem } from "🍎/state/menubar.svelte";
  import { desktop } from "🍎/state/desktop.svelte";

  interface Props {
    menu: Record<string, MenuItem>;
  }

  let { menu }: Props = $props();

  const isDark = $derived(desktop.theme === "dark");
</script>

<section class="block min-w-[14rem] w-max p-1.5 relative select-none bg-white/35 dark:bg-black/35 backdrop-blur-[25px] rounded-lg shadow-[0_0_11px_rgba(0,0,0,0.3)] dark:shadow-[inset_0_0_0_0.9px_rgba(255,255,255,0.1),0_0_0_1.2px_rgba(0,0,0,0.3),0_0_11px_rgba(0,0,0,0.3)]">
  {#each Object.entries(menu) as [_, val]}
    <button
      class="flex justify-start w-full px-2 py-1 my-[2px] tracking-wide font-normal text-[0.82rem] rounded text-black dark:text-white text-left transition-none disabled:opacity-40 {val.disabled ? '' : 'hover:bg-blue-500 hover:text-white hover:font-medium focus-visible:bg-blue-500 focus-visible:text-white focus-visible:font-medium'}"
      disabled={val.disabled}
      onclick={() => val.action?.()}
    >{val.title}</button>
    {#if val.breakAfter}
      <div class="w-full h-[0.5px] bg-black/10 dark:bg-white/10 my-[2px]"></div>
    {/if}
  {/each}
</section>
