<svelte:options runes={true} />

<script lang="ts">
  import {
    openSessionInDashboard,
    openSessionInWindow,
    openSessionInDrawer,
    openSessionFullscreen,
    openGenericBrowserWindow,
  } from "🍎/state/desktop.svelte";

  interface Props {
    session_id: string;
    port: number;
    title: string;
  }

  let { session_id, port, title }: Props = $props();

  let menuOpen = $state(false);
  let buttonEl = $state<HTMLButtonElement>();

  function toggle() {
    menuOpen = !menuOpen;
  }

  function close() {
    menuOpen = false;
  }

  function handleDashboard() {
    openSessionInDashboard(session_id);
    close();
  }
  function handleLeftDrawer() {
    openSessionInDrawer(session_id, "left");
    close();
  }
  function handleRightDrawer() {
    openSessionInDrawer(session_id, "right");
    close();
  }
  function handleFullscreen() {
    openSessionFullscreen(session_id);
    close();
  }
  function handleWindow() {
    openSessionInWindow(session_id);
    close();
  }
  function handleOsWindow() {
    const url = `http://localhost:${port}`;
    openGenericBrowserWindow(url, title ?? url.replace(/^https?:\/\//, ""));
    close();
  }

  const items = [
    { icon: "📊", label: "Dashboard", action: handleDashboard },
    { icon: "◀️", label: "Left Drawer", action: handleLeftDrawer },
    { icon: "▶️", label: "Right Drawer", action: handleRightDrawer },
    { icon: "⛶", label: "Fullscreen", action: handleFullscreen },
    { divider: true },
    { icon: "🪟", label: "Floating Window", action: handleWindow },
    { icon: "🌐", label: "New OS Window", action: handleOsWindow },
  ] as const;
</script>

<div class="relative inline-block">
  <button
    bind:this={buttonEl}
    class="rounded-full px-3 py-1.5 text-[0.8rem] font-medium bg-white/80 dark:bg-gray-800/80 border border-black/10 dark:border-white/10 hover:bg-white dark:hover:bg-gray-700 transition-colors text-black dark:text-white"
    onclick={toggle}
    data-testid="open-with-menu-trigger"
  >
    Open With ▾
  </button>

  {#if menuOpen}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 z-9998"
      onclick={close}
      onkeydown={undefined}
    ></div>

    <div
      class="absolute right-0 top-full mt-1 z-9999 min-w-[200px] py-1 rounded-xl bg-white/95 dark:bg-gray-800/95 backdrop-blur-xl border border-black/10 dark:border-white/10 shadow-xl shadow-black/20"
      data-testid="open-with-menu"
    >
      {#each items as item}
        {#if 'divider' in item && item.divider}
          <div class="h-px mx-2 my-1 bg-black/8 dark:bg-white/8"></div>
        {:else if 'action' in item}
          <button
            class="flex items-center gap-2.5 w-full px-3 py-1.5 text-[0.8rem] text-left bg-transparent border-none cursor-default text-gray-800 dark:text-gray-200 hover:bg-blue-500/15 dark:hover:bg-blue-500/20 transition-colors"
            onclick={item.action}
          >
            <span class="text-sm w-5 text-center">{item.icon}</span>
            {item.label}
          </button>
        {/if}
      {/each}
    </div>
  {/if}
</div>
