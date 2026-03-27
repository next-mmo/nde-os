<svelte:options runes={true} />

<script lang="ts">
  import Window from "🍎/components/Desktop/Window/Window.svelte";
  import { desktop } from "🍎/state/desktop.svelte";
</script>

<section class="relative w-full h-full">
  {#each desktop.windows as window (window.id)}
    <Window {window} />
  {/each}
</section>
