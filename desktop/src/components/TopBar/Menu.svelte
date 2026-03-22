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

<section class="container" class:dark={isDark}>
  {#each Object.entries(menu) as [_, val]}
    <button
      class="menu-item"
      disabled={val.disabled}
      onclick={() => val.action?.()}
    >{val.title}</button>
    {#if val.breakAfter}
      <div class="divider"></div>
    {/if}
  {/each}
</section>

<style>
  .container {
    --additional-box-shadow: 0 0 0 0 white;

    display: block;
    min-width: 14rem;
    width: max-content;
    padding: 0.35rem;
    position: relative;
    user-select: none;

    background-color: hsla(var(--system-color-light-hsl) / 0.35);
    backdrop-filter: blur(25px);
    border-radius: 0.5rem;

    box-shadow:
      hsla(0, 0%, 0%, 0.3) 0px 0px 11px 0px,
      var(--additional-box-shadow);
  }

  .container.dark {
    --additional-box-shadow:
      inset 0 0 0 0.9px hsla(var(--system-color-dark-hsl) / 0.3),
      0 0 0 1.2px hsla(var(--system-color-light-hsl) / 0.3);
  }

  .menu-item {
    display: flex;
    justify-content: flex-start;
    width: 100%;
    padding: 0.18rem 0.5rem;
    margin: 0.05rem 0;
    letter-spacing: 0.3px;
    font-weight: 400;
    font-size: 0.82rem;
    border-radius: 0.25rem;
    transition: none;
    color: var(--system-color-text);
    text-align: left;
  }

  .menu-item:disabled {
    opacity: 0.4;
  }

  .menu-item:not(:disabled):hover,
  .menu-item:not(:disabled):focus-visible {
    background-color: var(--system-color-primary);
    color: white;
    font-weight: 500;
  }

  .divider {
    width: 100%;
    height: 0.5px;
    background-color: var(--system-color-border);
    margin: 2px 0;
  }
</style>
