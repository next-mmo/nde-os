<svelte:options runes={true} />

<script lang="ts">
  import { onMount } from "svelte";
  import { QueryClient, QueryClientProvider } from "@tanstack/svelte-query";
  import Dock from "🍎/components/Dock/Dock.svelte";
  import Launchpad from "🍎/components/Desktop/Launchpad.svelte";
  import WindowsArea from "🍎/components/Desktop/Window/WindowsArea.svelte";
  import TopBar from "🍎/components/TopBar/TopBar.svelte";
  import { refreshAll } from "$lib/stores/state";
  import { bootDesktop, desktop } from "🍎/state/desktop.svelte";

  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 30_000,
        refetchOnWindowFocus: false,
      },
    },
  });

  onMount(() => {
    bootDesktop();
    refreshAll();

    const refreshTimer = window.setInterval(() => {
      refreshAll();
    }, 20_000);

    return () => window.clearInterval(refreshTimer);
  });

  $effect(() => {
    document.body.classList.toggle("dark", desktop.theme === "dark");
  });
</script>

<QueryClientProvider client={queryClient}>
  <div class="desktop-shell">
    <div class="wallpaper" aria-hidden="true"></div>

    <main class="desktop-grid">
      <TopBar />
      <WindowsArea />
      <Dock />
    </main>

    {#if desktop.launchpad_open}
      <Launchpad />
    {/if}
  </div>
</QueryClientProvider>

<style>
  .desktop-shell {
    width: 100%;
    height: 100%;
    position: relative;
    overflow: hidden;
  }

  .wallpaper {
    position: absolute;
    inset: 0;
    background:
      linear-gradient(180deg, hsla(215 85% 80% / 0.18), transparent 18%),
      linear-gradient(135deg, hsla(8 100% 76% / 0.16), transparent 32%),
      center / cover no-repeat var(--system-wallpaper);
    transform: scale(1.04);
    filter: saturate(1.05);
  }

  .desktop-grid {
    position: relative;
    z-index: 1;
    width: 100%;
    height: 100%;
    display: grid;
    grid-template-rows: auto 1fr auto;
  }
</style>
