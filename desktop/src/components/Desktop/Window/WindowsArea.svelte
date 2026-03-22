<svelte:options runes={true} />

<script lang="ts">
  import Window from "🍎/components/Desktop/Window/Window.svelte";
  import { visibleWindows } from "🍎/state/desktop.svelte";
</script>

<section class="windows-area">
  {#each visibleWindows() as window (window.id)}
    <Window {window} />
  {/each}
</section>

<style>
  .windows-area {
    position: relative;
    width: 100%;
    height: 100%;
  }
</style>
