<svelte:options runes={true} />

<script lang="ts">
  import { onMount } from "svelte";
  import { QueryClient, QueryClientProvider } from "@tanstack/svelte-query";
  import Dock from "🍎/components/Dock/Dock.svelte";
  import Launchpad from "🍎/components/Desktop/Launchpad.svelte";
  import DesktopIcons from "🍎/components/Desktop/DesktopIcons.svelte";
  import WindowsArea from "🍎/components/Desktop/Window/WindowsArea.svelte";
  import TopBar from "🍎/components/TopBar/TopBar.svelte";
  import { refreshAll, refreshResourceUsage } from "$lib/stores/state";
  import { bootDesktop, desktop, closeDrawer, closeFullscreenSession, getSessionById } from "🍎/state/desktop.svelte";

  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 30_000,
        refetchOnWindowFocus: false,
      },
    },
  });

  const drawerSession = $derived(
    desktop.drawer ? getSessionById(desktop.drawer.session_id) : null,
  );
  const fullscreenSession = $derived(
    desktop.fullscreen_session_id ? getSessionById(desktop.fullscreen_session_id) : null,
  );

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

    <!-- Desktop shortcut icons (on wallpaper) -->
    <DesktopIcons />

    <main class="relative z-10 w-full h-full grid grid-rows-[auto_1fr_auto] pointer-events-none">
      <div class="pointer-events-auto"><TopBar /></div>
      <div class="pointer-events-auto relative">
        <WindowsArea />

        <!-- Drawer panels -->
        {#if desktop.drawer && drawerSession}
          <div
            class="absolute z-50 top-0 bottom-0 {desktop.drawer.side === 'left' ? 'left-0 border-r' : 'right-0 border-l'} w-[50vw] max-w-[800px] min-w-[320px] flex flex-col bg-white/95 dark:bg-gray-900/95 backdrop-blur-2xl border-black/10 dark:border-white/10 shadow-2xl"
            data-testid="drawer-panel"
            data-drawer-side={desktop.drawer.side}
          >
            <header class="flex items-center justify-between gap-3 px-4 py-3 border-b border-black/8 dark:border-white/8 bg-linear-to-b from-white/90 to-white/60 dark:from-gray-800/90 dark:to-gray-800/60 shrink-0">
              <div class="flex items-center gap-2 min-w-0">
                <span class="text-sm">
                  {desktop.drawer.side === "left" ? "◀" : "▶"}
                </span>
                <strong class="text-sm font-semibold text-gray-900 dark:text-gray-100 truncate">{drawerSession.title}</strong>
                <span class="text-xs text-gray-500 dark:text-gray-400 shrink-0">{desktop.drawer.side} drawer</span>
              </div>
              <button
                class="shrink-0 w-7 h-7 rounded-lg grid place-items-center text-gray-500 hover:text-gray-900 dark:hover:text-gray-100 hover:bg-black/8 dark:hover:bg-white/8 transition-colors"
                onclick={closeDrawer}
                aria-label="Close drawer"
                data-testid="drawer-close"
              >
                ✕
              </button>
            </header>
            <div class="flex-1 min-h-0">
              <iframe
                src={drawerSession.url}
                title={drawerSession.title}
                class="w-full h-full border-none bg-white dark:bg-gray-900"
                sandbox="allow-same-origin allow-scripts allow-forms allow-popups allow-modals"
              ></iframe>
            </div>
          </div>
        {/if}

        <!-- Fullscreen session overlay -->
        {#if fullscreenSession}
          <div
            class="absolute inset-0 z-50 flex flex-col bg-white/95 dark:bg-gray-900/95 backdrop-blur-2xl"
            data-testid="fullscreen-panel"
          >
            <header class="flex items-center justify-between gap-3 px-4 py-2.5 border-b border-black/8 dark:border-white/8 bg-linear-to-b from-white/90 to-white/60 dark:from-gray-800/90 dark:to-gray-800/60 shrink-0">
              <div class="flex items-center gap-2 min-w-0">
                <span class="text-sm">⛶</span>
                <strong class="text-sm font-semibold text-gray-900 dark:text-gray-100 truncate">{fullscreenSession.title}</strong>
                <span class="text-xs text-gray-500 dark:text-gray-400 shrink-0">fullscreen</span>
              </div>
              <button
                class="shrink-0 w-7 h-7 rounded-lg grid place-items-center text-gray-500 hover:text-gray-900 dark:hover:text-gray-100 hover:bg-black/8 dark:hover:bg-white/8 transition-colors"
                onclick={closeFullscreenSession}
                aria-label="Close fullscreen"
                data-testid="fullscreen-close"
              >
                ✕
              </button>
            </header>
            <div class="flex-1 min-h-0">
              <iframe
                src={fullscreenSession.url}
                title={fullscreenSession.title}
                class="w-full h-full border-none bg-white dark:bg-gray-900"
                sandbox="allow-same-origin allow-scripts allow-forms allow-popups allow-modals"
              ></iframe>
            </div>
          </div>
        {/if}
      </div>
      <div class="pointer-events-auto"><Dock /></div>
    </main>

    {#if desktop.launchpad_open}
      <Launchpad />
    {/if}
  </div>
</QueryClientProvider>
