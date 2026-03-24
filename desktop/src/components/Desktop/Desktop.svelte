<svelte:options runes={true} />

<script lang="ts">
  import { onMount } from "svelte";
  import { QueryClient, QueryClientProvider } from "@tanstack/svelte-query";
  import Dock from "🍎/components/Dock/Dock.svelte";
  import Launchpad from "🍎/components/Desktop/Launchpad.svelte";
  import WindowsArea from "🍎/components/Desktop/Window/WindowsArea.svelte";
  import TopBar from "🍎/components/TopBar/TopBar.svelte";
  import { refreshAll, refreshResourceUsage } from "$lib/stores/state";
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

    const resourceTimer = window.setInterval(() => {
      refreshResourceUsage();
    }, 5_000);

    return () => {
      window.clearInterval(refreshTimer);
      window.clearInterval(resourceTimer);
    };
  });

  $effect(() => {
    document.body.classList.toggle("dark", desktop.theme === "dark");
  });
</script>

<QueryClientProvider client={queryClient}>
  <div class="w-full h-full relative overflow-hidden">
    <div class="absolute inset-0 scale-[1.04] saturate-[1.05]" style="background: linear-gradient(180deg, hsla(215 85% 80% / 0.18), transparent 18%), linear-gradient(135deg, hsla(8 100% 76% / 0.16), transparent 32%), center / cover no-repeat var(--system-wallpaper);" aria-hidden="true"></div>

    <main class="relative z-10 w-full h-full grid grid-rows-[auto_1fr_auto]">
      <TopBar />
      <WindowsArea />
      <Dock />
    </main>

    {#if desktop.launchpad_open}
      <Launchpad />
    {/if}
  </div>
</QueryClientProvider>
