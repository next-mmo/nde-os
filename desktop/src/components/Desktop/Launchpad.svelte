<svelte:options runes={true} />

<script lang="ts">
  import { click_outside } from "🍎/actions";
  import { apps_config } from "🍎/configs/apps/apps-config";
  import { revealOrLaunchManifest } from "$lib/session-actions";
  import { catalog, installedMap } from "$lib/stores/state";
  import { desktop, openStaticApp, selectLauncherSection, selectManifest, toggleLaunchpad } from "🍎/state/desktop.svelte";

  let panelEl = $state<HTMLElement>();
  let filter = $state("");

  const systemApps = (Object.entries(apps_config) as [
    keyof typeof apps_config,
    (typeof apps_config)[keyof typeof apps_config],
  ][]).filter(([app_id]) => app_id !== "ai-launcher");
  const filteredCatalog = $derived(
    $catalog.filter((app) => {
      if (!filter) return true;
      const query = filter.toLowerCase();
      return (
        app.name.toLowerCase().includes(query) ||
        app.id.toLowerCase().includes(query) ||
        app.tags.some((tag) => tag.toLowerCase().includes(query))
      );
    }),
  );

  function openShellApp(app_id: keyof typeof apps_config) {
    openStaticApp(app_id);
    toggleLaunchpad(false);
  }

  async function openManifest(appId: string, target: "embedded" | "windowed") {
    const manifest = $catalog.find((item) => item.id === appId);
    if (!manifest) return;
    const installed = $installedMap[appId] ?? null;
    if (!installed) {
      selectLauncherSection("catalog");
      selectManifest(appId);
      openStaticApp("ai-launcher");
      toggleLaunchpad(false);
      return;
    }
    await revealOrLaunchManifest(manifest, installed, target);
    toggleLaunchpad(false);
  }
</script>

<div class="overlay">
  <section
    data-testid="launchpad"
    class="launchpad"
    bind:this={panelEl}
    use:click_outside={() => toggleLaunchpad(false)}
    aria-label="Launchpad"
  >
    <header class="launchpad-header">
      <div>
        <p class="eyebrow">Launchpad</p>
        <h2>Open apps from the dashboard or a separate window.</h2>
      </div>
      <input bind:value={filter} placeholder="Search apps" aria-label="Search apps" />
    </header>

    <div class="section">
      <p class="section-title">System</p>
      <div class="tile-grid">
        <button class="tile" onclick={() => openShellApp("ai-launcher")}>
          <img src="/app-icons/ai-launcher/256.webp" alt="" />
          <span>AI Launcher</span>
        </button>
        {#each systemApps as [app_id, app_config]}
          <button class="tile" onclick={() => openShellApp(app_id)}>
            <img src="/app-icons/{app_id}/256.webp" alt="" />
            <span>{app_config.title}</span>
          </button>
        {/each}
      </div>
    </div>

    <div class="section">
      <div class="section-bar">
        <p class="section-title">AI Apps</p>
        <span>{filteredCatalog.length} app(s)</span>
      </div>
      <div class="catalog-grid">
        {#each filteredCatalog as app (app.id)}
          <article class="catalog-card">
            <button class="catalog-main" onclick={() => openManifest(app.id, "embedded")}>
              <div class="catalog-icon">{app.icon ?? "AI"}</div>
              <div>
                <strong>{app.name}</strong>
                <p>{app.description}</p>
              </div>
            </button>
            <div class="card-actions">
              <button onclick={() => openManifest(app.id, "embedded")}>Open in Dashboard</button>
              <button onclick={() => openManifest(app.id, "windowed")}>Open in Window</button>
            </div>
          </article>
        {/each}
      </div>
    </div>
  </section>
</div>

<style>
  .overlay {
    position: absolute;
    inset: 0;
    z-index: 100;
    backdrop-filter: blur(26px);
    background: linear-gradient(180deg, hsla(222 66% 8% / 0.35), hsla(222 66% 8% / 0.2));
    padding: 4.8rem 4rem 6.5rem;
  }

  .launchpad {
    width: min(1240px, 100%);
    height: 100%;
    margin: 0 auto;
    border-radius: 2rem;
    background: hsla(var(--system-color-light-hsl) / 0.36);
    border: 1px solid hsla(0 0% 100% / 0.34);
    box-shadow: 0 32px 96px hsla(222 60% 10% / 0.28);
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    padding: 2rem;
  }

  .launchpad-header {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
  }

  .eyebrow,
  .section-title {
    text-transform: uppercase;
    letter-spacing: 0.14em;
    font-size: 0.72rem;
    font-weight: 700;
    margin: 0;
    color: var(--system-color-text-muted);
  }

  h2 {
    margin: 0.3rem 0 0;
    max-width: 34rem;
    font-size: 1.9rem;
    line-height: 1.05;
  }

  input {
    width: 18rem;
    border-radius: 999px;
    border: 1px solid var(--system-color-border);
    background: hsla(var(--system-color-light-hsl) / 0.78);
    padding: 0.95rem 1.2rem;
    color: var(--system-color-text);
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
  }

  .section-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    color: var(--system-color-text-muted);
    font-size: 0.85rem;
  }

  .tile-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(7.8rem, 1fr));
    gap: 1rem;
  }

  .tile {
    display: grid;
    gap: 0.7rem;
    justify-items: center;
    padding: 1rem 0.75rem;
    border-radius: 1.4rem;
    background: hsla(var(--system-color-light-hsl) / 0.5);
    border: 1px solid var(--system-color-border);
  }

  .tile img {
    width: 4.6rem;
    height: 4.6rem;
  }

  .tile span {
    font-size: 0.88rem;
    text-align: center;
  }

  .catalog-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(17rem, 1fr));
    gap: 1rem;
    overflow: auto;
    padding-right: 0.35rem;
  }

  .catalog-card {
    display: grid;
    gap: 0.8rem;
    padding: 1rem;
    border-radius: 1.2rem;
    background: hsla(var(--system-color-light-hsl) / 0.6);
    border: 1px solid var(--system-color-border);
  }

  .catalog-main {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.9rem;
    align-items: start;
    text-align: left;
  }

  .catalog-icon {
    width: 3rem;
    height: 3rem;
    border-radius: 0.95rem;
    background: linear-gradient(180deg, hsla(var(--system-color-primary-hsl) / 0.22), hsla(206 72% 44% / 0.08));
    display: grid;
    place-items: center;
    font-weight: 700;
    color: var(--system-color-primary);
  }

  strong {
    display: block;
    margin-bottom: 0.3rem;
  }

  p {
    margin: 0;
    color: var(--system-color-text-muted);
    font-size: 0.84rem;
  }

  .card-actions {
    display: flex;
    gap: 0.55rem;
  }

  .card-actions button {
    flex: 1;
    border-radius: 999px;
    padding: 0.6rem 0.8rem;
    background: hsla(var(--system-color-light-hsl) / 0.84);
    border: 1px solid var(--system-color-border);
    font-size: 0.8rem;
  }

  @media (max-width: 900px) {
    .overlay {
      padding: 4.4rem 1rem 6rem;
    }

    .launchpad {
      padding: 1rem;
    }

    .launchpad-header {
      flex-direction: column;
    }

    input {
      width: 100%;
    }
  }
</style>
