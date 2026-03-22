<svelte:options runes={true} />

<script lang="ts">
  import { elevation } from "🍎/actions";
  import { apps_config } from "🍎/configs/apps/apps-config";
  import DockItem from "🍎/components/Dock/DockItem.svelte";

  let mouseX = $state<number | null>(null);
</script>

<section class="dock-shell" use:elevation={"dock"}>
  <div
    class="dock"
    role="toolbar"
    aria-label="Dock"
    tabindex="-1"
    onmousemove={(event) => (mouseX = event.clientX)}
    onmouseleave={() => (mouseX = null)}
  >
    {#each Object.entries(apps_config) as [app_id, config]}
      {#if config.dock_breaks_before}
        <div class="divider" aria-hidden="true"></div>
      {/if}
      <DockItem app_id={app_id} mouse_x={mouseX} />
    {/each}
  </div>
</section>

<style>
  .dock-shell {
    display: flex;
    justify-content: center;
    padding: 0.55rem 0 0.95rem;
    pointer-events: none;
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
