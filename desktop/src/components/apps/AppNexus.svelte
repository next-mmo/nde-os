<svelte:options runes={true} />

<script lang="ts">
  import type { DesktopWindow } from "🍎/state/desktop.svelte";

  interface Props {
    window: DesktopWindow;
  }

  let { window }: Props = $props();
</script>

{#if window.app_id === "ai-launcher"}
  {#await import("🍎/components/apps/Launcher/Launcher.svelte") then { default: Launcher }}
    <Launcher />
  {/await}
{:else if window.app_id === "browser"}
  {#await import("🍎/components/apps/Browser/Browser.svelte") then { default: Browser }}
    <Browser {window} />
  {/await}
{:else if window.app_id === "logs"}
  {#await import("🍎/components/apps/Logs/Logs.svelte") then { default: Logs }}
    <Logs />
  {/await}
{:else if window.app_id === "settings"}
  {#await import("🍎/components/apps/Settings/Settings.svelte") then { default: Settings }}
    <Settings />
  {/await}
{:else}
  {#await import("🍎/components/apps/Placeholder/Placeholder.svelte") then { default: Placeholder }}
    <Placeholder app_id={window.app_id} />
  {/await}
{/if}
