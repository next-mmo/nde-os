<svelte:options runes={true} />

<script lang="ts">
  import MenuBar from "🍎/components/TopBar/MenuBar.svelte";
  import ActionCenter from "🍎/components/TopBar/ActionCenter.svelte";
  import TopBarMetrics from "./TopBarMetrics.svelte";
  import TopBarTime from "🍎/components/TopBar/TopBarTime.svelte";

  let isTauri = $state(false);
  let tauriWindow: any = null;

  $effect(() => {
    if ("__TAURI_INTERNALS__" in window) {
      isTauri = true;
      import("@tauri-apps/api/window").then((mod) => {
        tauriWindow = mod.getCurrentWindow();
      });
    }
  });

  function handleClose() {
    tauriWindow?.close();
  }

  function handleMinimize() {
    tauriWindow?.minimize();
  }

  function handleMaximize() {
    tauriWindow?.toggleMaximize();
  }
</script>

<header data-tauri-drag-region class="flex items-center relative w-full h-[1.65rem] bg-white/30 dark:bg-black/30 text-black dark:text-white fill-current z-[9990] backdrop-blur-md before:content-[''] before:fixed before:inset-x-0 before:top-0 before:h-inherit before:-z-10 before:backdrop-blur-md">
  {#if isTauri}
    <div class="flex items-center gap-[0.45rem] ml-[0.65rem] mr-1 group/tl">
      <button
        class="grid place-items-center w-[0.75rem] h-[0.75rem] rounded-full bg-[#ff5f57] border-[0.5px] border-black/10 shadow-[inset_0_1px_1px_rgba(255,255,255,0.2)] p-0 [-webkit-app-region:no-drag] hover:brightness-90 transition-[filter]"
        aria-label="Close window"
        onclick={handleClose}
      >
        <svg class="w-[6px] h-[6px] opacity-0 group-hover/tl:opacity-100 transition-opacity" viewBox="0 0 12 12" fill="none" stroke="rgba(0,0,0,0.5)" stroke-width="2" stroke-linecap="round"><line x1="2" y1="2" x2="10" y2="10"/><line x1="10" y1="2" x2="2" y2="10"/></svg>
      </button>
      <button
        class="grid place-items-center w-[0.75rem] h-[0.75rem] rounded-full bg-[#febc2e] border-[0.5px] border-black/10 shadow-[inset_0_1px_1px_rgba(255,255,255,0.2)] p-0 [-webkit-app-region:no-drag] hover:brightness-90 transition-[filter]"
        aria-label="Minimize window"
        onclick={handleMinimize}
      >
        <svg class="w-[6px] h-[6px] opacity-0 group-hover/tl:opacity-100 transition-opacity" viewBox="0 0 12 12" fill="none" stroke="rgba(0,0,0,0.5)" stroke-width="2" stroke-linecap="round"><line x1="1" y1="6" x2="11" y2="6"/></svg>
      </button>
      <button
        class="grid place-items-center w-[0.75rem] h-[0.75rem] rounded-full bg-[#28c840] border-[0.5px] border-black/10 shadow-[inset_0_1px_1px_rgba(255,255,255,0.2)] p-0 [-webkit-app-region:no-drag] hover:brightness-90 transition-[filter]"
        aria-label="Toggle fullscreen"
        onclick={handleMaximize}
      >
        <svg class="w-[6px] h-[6px] opacity-0 group-hover/tl:opacity-100 transition-opacity" viewBox="0 0 12 12" fill="none" stroke="rgba(0,0,0,0.5)" stroke-width="1.5"><path d="M2 3.5V2h1.5M10 3.5V2H8.5M2 8.5V10h1.5M10 8.5V10H8.5"/></svg>
      </button>
    </div>
  {/if}

  <MenuBar />

  <span class="flex-1"></span>

  <ActionCenter />
  <TopBarMetrics />

  <button class="font-medium text-[0.78rem] tracking-wide relative h-full px-2.5 text-black dark:text-white whitespace-nowrap rounded hover:bg-black/10 dark:hover:bg-white/10" aria-label="Current time">
    <TopBarTime />
  </button>
</header>
