<svelte:options runes={true} />

<script lang="ts">
  import { elevation } from "🍎/actions";
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

<section class="dock-shell" class:auto-hide={desktop.dock_auto_hide} use:elevation={"dock"}>
  <div
    class="dock"
    role="toolbar"
    aria-label="Dock"
    tabindex="-1"
    onmousemove={(event) => (mouseX = event.clientX)}
    onmouseleave={() => (mouseX = null)}
  >
    {#each dockApps as { app_id } (app_id)}
      {#if apps_config[app_id as keyof typeof apps_config]?.dock_breaks_before}
        <div class="divider" aria-hidden="true"></div>
      {/if}
      <DockItem {app_id} mouse_x={mouseX} />
    {/each}
  </div>
</section>

<style>
  .dock-shell {
    display: flex;
    justify-content: center;
    padding: 0.55rem 0 0.95rem;
    pointer-events: none;
    transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .dock-shell.auto-hide {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    z-index: 1000;
    transform: translateY(calc(100% - 10px));
  }

  .dock-shell.auto-hide:hover, .dock-shell.auto-hide:focus-within {
    transform: translateY(0);
  }

  .dock {
    pointer-events: auto;
    display: flex;
    align-items: flex-end;
    gap: 0.3rem;
    padding: 0.4rem 0.55rem;
    border-radius: 1.2rem;
    background: hsla(var(--system-color-light-hsl) / 0.38);
    border: 1px solid hsla(0 0% 100% / 0.26);
    box-shadow:
      inset 0 0 0 0.5px hsla(0 0% 100% / 0.5),
      0 18px 50px hsla(220 40% 10% / 0.28);
    backdrop-filter: blur(14px);
  }

  .divider {
    width: 1px;
    align-self: stretch;
    background: hsla(220 12% 20% / 0.18);
    margin: 0 0.2rem;
  }
</style>
