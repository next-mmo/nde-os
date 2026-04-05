<svelte:options runes={true} />

<script lang="ts">
  import { apps_config } from "🍎/configs/apps/apps-config";
  import DockItem from "🍎/components/Dock/DockItem.svelte";
  import { desktop } from "🍎/state/desktop.svelte";
  import { onMount } from "svelte";

  let mouseX = $state<number | null>(null);
  let dockVisible = $state(true);
  let hideTimeoutId: ReturnType<typeof setTimeout> | null = null;
  let dockEl = $state<HTMLElement>();

  const HIDE_DELAY = 600;   // ms before dock hides after mouse leaves
  const HOT_ZONE_PX = 6;    // invisible pixel strip at screen bottom to trigger reveal

  // --- auto-hide logic ---
  function showDock() {
    if (hideTimeoutId) { clearTimeout(hideTimeoutId); hideTimeoutId = null; }
    dockVisible = true;
  }

  function scheduleHide() {
    if (hideTimeoutId) clearTimeout(hideTimeoutId);
    hideTimeoutId = setTimeout(() => { dockVisible = false; hideTimeoutId = null; }, HIDE_DELAY);
  }

  function handleDockMouseEnter() {
    if (desktop.dock_auto_hide) showDock();
  }

  function handleDockMouseLeave() {
    mouseX = null;
    if (desktop.dock_auto_hide) scheduleHide();
  }

  // Bottom-edge hot zone: reveal dock when cursor hits bottom of screen
  onMount(() => {
    function onGlobalMove(e: MouseEvent) {
      if (!desktop.dock_auto_hide) return;
      const atBottom = e.clientY >= window.innerHeight - HOT_ZONE_PX;
      if (atBottom) showDock();
    }
    window.addEventListener("mousemove", onGlobalMove, { passive: true });
    return () => {
      window.removeEventListener("mousemove", onGlobalMove);
      if (hideTimeoutId) clearTimeout(hideTimeoutId);
    };
  });

  // When auto-hide is toggled off, always show
  $effect(() => {
    if (!desktop.dock_auto_hide) {
      dockVisible = true;
      if (hideTimeoutId) { clearTimeout(hideTimeoutId); hideTimeoutId = null; }
    }
  });

  const isHidden = $derived(desktop.dock_auto_hide && !dockVisible);

  const dockApps = $derived.by(() => {
    const ObjectKeys = Object.keys(apps_config) as (keyof typeof apps_config)[];
    const apps = ObjectKeys.map(id => ({ app_id: id as string, isStatic: true }));
    const existing = new Set(apps.map(a => a.app_id));

    if (desktop.sessions) {
      for (const session of desktop.sessions) {
        if (!existing.has(session.app_id) && session.app_id !== "browser") {
          apps.push({ app_id: session.app_id, isStatic: false });
          existing.add(session.app_id);
        }
      }
    }

    if (desktop.windows) {
      for (const win of desktop.windows) {
        if (!existing.has(win.app_id) && win.app_id !== "browser") {
          apps.push({ app_id: win.app_id as string, isStatic: false });
          existing.add(win.app_id);
        }
      }
    }

    return apps;
  });
</script>

<section
  class="flex justify-center pt-2 pb-4 pointer-events-none"
  style="transition: transform 0.4s cubic-bezier(0.32, 0.72, 0, 1), opacity 0.35s ease;
         transform: translateY({isHidden ? 'calc(100% + 8px)' : '0px'});
         opacity: {isHidden ? 0 : 1};"
  bind:this={dockEl}
>
  <div
    class="pointer-events-auto flex items-end gap-1 px-2.5 py-1.5 rounded-[1.25rem] bg-white/40 dark:bg-black/30 backdrop-blur-xl border border-white/40 dark:border-white/10 shadow-[inset_0_1px_1px_rgba(255,255,255,0.4),0_18px_50px_rgba(0,0,0,0.28)]"
    role="toolbar"
    aria-label="Dock"
    tabindex="-1"
    onmouseenter={handleDockMouseEnter}
    onmouseleave={handleDockMouseLeave}
    onmousemove={(event) => (mouseX = event.clientX)}
  >
    {#each dockApps as { app_id } (app_id)}
      {#if apps_config[app_id as keyof typeof apps_config]?.dock_breaks_before}
        <div class="w-[1px] self-stretch bg-black/10 dark:bg-white/10 mx-1" aria-hidden="true"></div>
      {/if}
      <DockItem {app_id} mouse_x={mouseX} />
    {/each}
  </div>
</section>
