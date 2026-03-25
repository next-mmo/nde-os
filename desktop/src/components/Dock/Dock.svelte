<svelte:options runes={true} />

<script lang="ts">
  import { apps_config } from "🍎/configs/apps/apps-config";
  import DockItem from "🍎/components/Dock/DockItem.svelte";
  import { desktop } from "🍎/state/desktop.svelte";

  let mouseX = $state<number | null>(null);

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
  class="flex justify-center pt-2 pb-4 pointer-events-none transition-transform duration-300 ease-[cubic-bezier(0.4,0,0.2,1)] {desktop.dock_auto_hide ? 'translate-y-[calc(100%-10px)] hover:translate-y-0 focus-within:translate-y-0' : ''}" 
>
  <div
    class="pointer-events-auto flex items-end gap-1 px-2.5 py-1.5 rounded-[1.25rem] bg-white/40 dark:bg-black/30 backdrop-blur-xl border border-white/40 dark:border-white/10 shadow-[inset_0_1px_1px_rgba(255,255,255,0.4),0_18px_50px_rgba(0,0,0,0.28)]"
    role="toolbar"
    aria-label="Dock"
    tabindex="-1"
    onmousemove={(event) => (mouseX = event.clientX)}
    onmouseleave={() => (mouseX = null)}
  >
    {#each dockApps as { app_id } (app_id)}
      {#if apps_config[app_id as keyof typeof apps_config]?.dock_breaks_before}
        <div class="w-[1px] self-stretch bg-black/10 dark:bg-white/10 mx-1" aria-hidden="true"></div>
      {/if}
      <DockItem {app_id} mouse_x={mouseX} />
    {/each}
  </div>
</section>
