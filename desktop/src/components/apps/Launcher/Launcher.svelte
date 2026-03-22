<svelte:options runes={true} />

<script lang="ts">
  import {
    installAndFocusCatalog,
    openSessionExternally,
    revealOrLaunchManifest,
    stopManifest,
  } from "$lib/session-actions";
  import type { AppManifest, InstalledApp } from "$lib/api/types";
  import {
    catalog,
    catalogCount,
    healthStatus,
    installed,
    installedMap,
    lastRefreshAt,
    refreshAll,
    runningCount,
    systemInfo,
    uninstallApp,
  } from "$lib/stores/state";
  import SessionSurface from "🍎/components/apps/Launcher/SessionSurface.svelte";
  import {
    desktop,
    focusSessionDetails,
    getSessionById,
    launcherRunningSessions,
    openGenericBrowserWindow,
    openSessionInDashboard,
    openSessionInWindow,
    openStaticApp,
    selectLauncherSection,
    selectManifest,
    syncSessionsFromInstalled,
    type LauncherSection,
  } from "🍎/state/desktop.svelte";

  let workingAppId = $state<string | null>(null);
  let visibleLimit = $state(12);

  const sections: { id: LauncherSection; label: string }[] = [
    { id: "overview", label: "Overview" },
    { id: "catalog", label: "Catalog" },
    { id: "installed", label: "Installed" },
    { id: "running", label: "Running" },
    { id: "server", label: "Server & System" },
  ];

  $effect(() => {
    syncSessionsFromInstalled($installed);
  });

  $effect(() => {
    desktop.launcher_query;
    visibleLimit = 12;
  });

  const filteredCatalog = $derived(
    $catalog.filter((app) => {
      const query = desktop.launcher_query.trim().toLowerCase();
      if (!query) return true;
      return (
        app.name.toLowerCase().includes(query) ||
        app.description.toLowerCase().includes(query) ||
        app.tags.some((tag) => tag.toLowerCase().includes(query))
      );
    }),
  );

  const visibleCatalog = $derived(filteredCatalog.slice(0, visibleLimit));
  const currentSession = $derived(
    desktop.workspace_view.kind === "session" ? getSessionById(desktop.workspace_view.session_id) : null,
  );
  const selectedManifest = $derived(
    $catalog.find((app) => app.id === desktop.selected_app_id) ??
      (currentSession ? $catalog.find((app) => app.id === currentSession.app_id) ?? null : null),
  );
  const selectedInstalled = $derived(selectedManifest ? $installedMap[selectedManifest.id] ?? null : null);
  const sessions = $derived(launcherRunningSessions());
  const quickCatalog = $derived($catalog.slice(0, 4));

  function appStatus(app: AppManifest) {
    const installedApp = $installedMap[app.id];
    if (!installedApp) return "available";
    if (installedApp.status.state === "Running") return "running";
    return "installed";
  }

  async function refreshWorkspace() {
    await refreshAll();
  }

  async function launchManifest(app: AppManifest, target: "embedded" | "windowed") {
    const installedApp = $installedMap[app.id] ?? null;
    workingAppId = app.id;
    try {
      await revealOrLaunchManifest(app, installedApp, target);
      selectManifest(app.id);
    } finally {
      workingAppId = null;
    }
  }

  async function installManifest(app: AppManifest) {
    workingAppId = app.id;
    try {
      await installAndFocusCatalog(app);
    } finally {
      workingAppId = null;
    }
  }

  async function stopInstalledApp(installedApp: InstalledApp) {
    workingAppId = installedApp.manifest.id;
    try {
      await stopManifest(installedApp);
    } finally {
      workingAppId = null;
    }
  }

  async function uninstallManifest(app: AppManifest) {
    workingAppId = app.id;
    try {
      await uninstallApp(app.id);
    } finally {
      workingAppId = null;
    }
  }

  function selectSection(section: LauncherSection) {
    selectLauncherSection(section);
  }

  function showManifestDetails(appId: string) {
    selectManifest(appId);
  }

  function showSession(sessionId: string) {
    focusSessionDetails(sessionId);
    openSessionInDashboard(sessionId);
  }

  async function defaultTileAction(app: AppManifest) {
    const installedApp = $installedMap[app.id] ?? null;
    if (!installedApp) {
      selectSection("catalog");
      selectManifest(app.id);
      return;
    }
    await launchManifest(app, "embedded");
  }

  function currentLoadLabel() {
    if (!currentSession) return "";
    if (currentSession.load_state === "fallback") {
      return "Embedding looked unreliable, so the app was moved to a separate browser window.";
    }
    if (currentSession.load_state === "loading") {
      return "Preparing preview...";
    }
    return "";
  }
</script>

<section class="launcher">
  <aside class="rail">
    <div class="brand">
      <img src="/app-icons/ai-launcher/256.webp" alt="" />
      <div>
        <strong>AI Launcher</strong>
        <span>workspace</span>
      </div>
    </div>

    <nav>
      {#each sections as section}
        <button
          class="rail-link"
          class:active={desktop.launcher_section === section.id && desktop.workspace_view.kind === "dashboard"}
          onclick={() => selectSection(section.id)}
        >
          <span>{section.label}</span>
          {#if section.id === "catalog"}
            <small>{$catalogCount}</small>
          {:else if section.id === "running"}
            <small>{$runningCount}</small>
          {/if}
        </button>
      {/each}
    </nav>

    <div class="rail-card">
      <span class="eyebrow">Default open</span>
      <strong>Dashboard first</strong>
      <p>Primary actions focus the main workspace. Use the secondary action to pop out into a browser window.</p>
    </div>
  </aside>

  <div class="workspace-shell">
    <header class="toolbar">
      <div class="toolbar-left">
        <input
          bind:value={desktop.launcher_query}
          placeholder="Search apps, ids, tags"
          aria-label="Search AI apps"
        />
        <button onclick={refreshWorkspace}>Refresh</button>
      </div>
      <div class="toolbar-right">
        <button onclick={() => openStaticApp("settings")}>Settings</button>
        <button onclick={openGenericBrowserWindow}>New Browser</button>
      </div>
    </header>

    <div class="session-strip">
      <button
        class="session-pill"
        class:active={desktop.workspace_view.kind === "dashboard"}
        onclick={() => selectSection(desktop.launcher_section)}
      >
        Dashboard
      </button>
      {#each sessions as session (session.id)}
        <button
          class="session-pill"
          class:active={desktop.workspace_view.kind === "session" && desktop.workspace_view.session_id === session.id}
          onclick={() => showSession(session.id)}
        >
          <span>{session.title}</span>
          <small>{session.mode === "windowed" ? "Window" : "Embedded"}</small>
        </button>
      {/each}
    </div>

    <div class="workspace">
      <div class="center">
        {#if desktop.workspace_view.kind === "session" && currentSession}
          <div class="session-headline">
            <div>
              <p class="eyebrow">Active preview</p>
              <h2>{currentSession.title}</h2>
            </div>
            <div class="session-actions">
              <button onclick={() => openSessionInWindow(currentSession.id)}>Open in Window</button>
              <button onclick={() => openSessionExternally(currentSession.port)}>Open External</button>
            </div>
          </div>

          {#if currentLoadLabel()}
            <div class="notice">{currentLoadLabel()}</div>
          {/if}

          <div class="session-surface">
            <SessionSurface session={currentSession} />
          </div>
        {:else if desktop.launcher_section === "overview"}
          <div class="overview-grid">
            <article class="hero-card">
              <p class="eyebrow">Overview</p>
              <h2>Manage local AI apps like browser workspaces.</h2>
              <p class="hero-copy">
                Launch apps into the dashboard by default, pop them into separate browser windows when needed,
                and keep server-backed lifecycle controls inside one desktop shell.
              </p>
              <div class="hero-actions">
                <button onclick={() => selectSection("catalog")}>Browse Catalog</button>
                <button onclick={() => selectSection("running")}>View Running Apps</button>
              </div>
            </article>

            <div class="stat-grid">
              <article class="stat-card">
                <span>Catalog</span>
                <strong>{$catalogCount}</strong>
              </article>
              <article class="stat-card">
                <span>Running</span>
                <strong>{$runningCount}</strong>
              </article>
              <article class="stat-card">
                <span>Server</span>
                <strong>{$healthStatus}</strong>
              </article>
              <article class="stat-card">
                <span>Last refresh</span>
                <strong>{$lastRefreshAt ? new Date($lastRefreshAt).toLocaleTimeString() : "Never"}</strong>
              </article>
            </div>

            <article class="panel">
              <div class="panel-header">
                <div>
                  <p class="eyebrow">Quick launch</p>
                  <h3>Popular apps</h3>
                </div>
              </div>
              <div class="row-list">
                {#each quickCatalog as app (app.id)}
                  <button class="row-button" onclick={() => defaultTileAction(app)}>
                    <div>
                      <strong>{app.name}</strong>
                      <p>{app.description}</p>
                    </div>
                    <small>{appStatus(app)}</small>
                  </button>
                {/each}
              </div>
            </article>

            <article class="panel">
              <div class="panel-header">
                <div>
                  <p class="eyebrow">Sessions</p>
                  <h3>Recent running apps</h3>
                </div>
              </div>
              <div class="row-list">
                {#each sessions as session (session.id)}
                  <button class="row-button" onclick={() => showSession(session.id)}>
                    <div>
                      <strong>{session.title}</strong>
                      <p>{session.url}</p>
                    </div>
                    <small>{session.mode}</small>
                  </button>
                {:else}
                  <div class="empty-panel">No running app sessions yet.</div>
                {/each}
              </div>
            </article>
          </div>
        {:else if desktop.launcher_section === "catalog"}
          <div class="panel">
            <div class="panel-header">
              <div>
                <p class="eyebrow">Catalog</p>
                <h3>Available AI apps</h3>
              </div>
              <span>{filteredCatalog.length} result(s)</span>
            </div>
            <div class="row-list">
              {#each visibleCatalog as app (app.id)}
                <article class="app-row" data-app-id={app.id}>
                  <button class="app-main" onclick={() => showManifestDetails(app.id)}>
                    <div class="icon">{app.icon ?? app.name.slice(0, 2).toUpperCase()}</div>
                    <div>
                      <strong>{app.name}</strong>
                      <p>{app.description}</p>
                    </div>
                  </button>
                  <div class="row-actions">
                    {#if !$installedMap[app.id]}
                      <button disabled={workingAppId === app.id} onclick={() => installManifest(app)}>Install</button>
                    {:else}
                      <button disabled={workingAppId === app.id} onclick={() => launchManifest(app, "embedded")}>Open in Dashboard</button>
                      <button disabled={workingAppId === app.id} onclick={() => launchManifest(app, "windowed")}>Open in Window</button>
                    {/if}
                  </div>
                </article>
              {/each}
            </div>
            {#if visibleCatalog.length < filteredCatalog.length}
              <div class="show-more">
                <button onclick={() => (visibleLimit += 12)}>Show more</button>
              </div>
            {/if}
          </div>
        {:else if desktop.launcher_section === "installed"}
          <div class="panel">
            <div class="panel-header">
              <div>
                <p class="eyebrow">Installed</p>
                <h3>Ready to run</h3>
              </div>
              <span>{$installed.length} installed</span>
            </div>
            <div class="row-list">
              {#each $installed as app (app.manifest.id)}
                <article class="app-row" data-app-id={app.manifest.id}>
                  <button class="app-main" onclick={() => showManifestDetails(app.manifest.id)}>
                    <div class="icon">{app.manifest.icon ?? app.manifest.name.slice(0, 2).toUpperCase()}</div>
                    <div>
                      <strong>{app.manifest.name}</strong>
                      <p>{app.workspace}</p>
                    </div>
                  </button>
                  <div class="row-actions">
                    <button disabled={workingAppId === app.manifest.id} onclick={() => launchManifest(app.manifest, "embedded")}>Open in Dashboard</button>
                    <button disabled={workingAppId === app.manifest.id} onclick={() => launchManifest(app.manifest, "windowed")}>Open in Window</button>
                    <button disabled={workingAppId === app.manifest.id} onclick={() => uninstallManifest(app.manifest)}>Uninstall</button>
                  </div>
                </article>
              {:else}
                <div class="empty-panel">Install an app from the catalog to see it here.</div>
              {/each}
            </div>
          </div>
        {:else if desktop.launcher_section === "running"}
          <div class="panel">
            <div class="panel-header">
              <div>
                <p class="eyebrow">Running</p>
                <h3>Live sessions</h3>
              </div>
              <span>{sessions.length} session(s)</span>
            </div>
            <div class="row-list">
              {#each sessions as session (session.id)}
                <article class="app-row" data-session-id={session.id}>
                  <button class="app-main" onclick={() => showSession(session.id)}>
                    <div class="icon">{session.title.slice(0, 2).toUpperCase()}</div>
                    <div>
                      <strong>{session.title}</strong>
                      <p>{session.url}</p>
                    </div>
                  </button>
                  <div class="row-actions">
                    <button onclick={() => openSessionInDashboard(session.id)}>Open in Dashboard</button>
                    <button onclick={() => openSessionInWindow(session.id)}>Open in Window</button>
                  </div>
                </article>
              {:else}
                <div class="empty-panel">No running sessions yet.</div>
              {/each}
            </div>
          </div>
        {:else}
          <div class="panel">
            <div class="panel-header">
              <div>
                <p class="eyebrow">Server & System</p>
                <h3>Runtime status</h3>
              </div>
            </div>
            {#if $systemInfo}
              <div class="detail-grid">
                <div class="detail-card">
                  <span>Server health</span>
                  <strong>{$healthStatus}</strong>
                </div>
                <div class="detail-card">
                  <span>OS</span>
                  <strong>{$systemInfo.os} / {$systemInfo.arch}</strong>
                </div>
                <div class="detail-card">
                  <span>Python</span>
                  <strong>{$systemInfo.python_version ?? "Not detected"}</strong>
                </div>
                <div class="detail-card">
                  <span>uv</span>
                  <strong>{$systemInfo.uv.uv_version}</strong>
                </div>
              </div>
            {/if}
          </div>
        {/if}
      </div>

      <aside class="detail-pane">
        {#if selectedManifest}
          <div class="detail-panel">
            <p class="eyebrow">Inspector</p>
            <h3>{selectedManifest.name}</h3>
            <p class="detail-copy">{selectedManifest.description}</p>

            <div class="detail-meta">
              <div><span>Author</span><strong>{selectedManifest.author}</strong></div>
              <div><span>Port</span><strong>{selectedManifest.port}</strong></div>
              <div><span>Disk</span><strong>{selectedManifest.disk_size}</strong></div>
              <div><span>Python</span><strong>{selectedManifest.python_version}</strong></div>
            </div>

            <div class="tag-list">
              {#each selectedManifest.tags as tag}
                <span>{tag}</span>
              {/each}
              {#if selectedManifest.needs_gpu}
                <span>gpu</span>
              {/if}
            </div>

            <div class="detail-actions">
              {#if !selectedInstalled}
                <button onclick={() => installManifest(selectedManifest)}>Install</button>
              {:else if selectedInstalled.status.state === "Running"}
                <button onclick={() => launchManifest(selectedManifest, "embedded")}>Open in Dashboard</button>
                <button onclick={() => launchManifest(selectedManifest, "windowed")}>Open in Window</button>
                <button onclick={() => stopInstalledApp(selectedInstalled)}>Stop</button>
                <button onclick={() => uninstallManifest(selectedManifest)}>Uninstall</button>
                {#if selectedInstalled.status.port}
                  <button onclick={() => openSessionExternally(selectedInstalled.status.port!)}>Open External</button>
                {/if}
              {:else}
                <button onclick={() => launchManifest(selectedManifest, "embedded")}>Open in Dashboard</button>
                <button onclick={() => launchManifest(selectedManifest, "windowed")}>Open in Window</button>
                <button onclick={() => uninstallManifest(selectedManifest)}>Uninstall</button>
              {/if}
            </div>
          </div>
        {:else}
          <div class="detail-panel empty-detail">
            <p class="eyebrow">Inspector</p>
            <h3>Select an app</h3>
            <p class="detail-copy">Choose an app or running session to see details and actions here.</p>
          </div>
        {/if}

        <div class="detail-panel">
          <p class="eyebrow">Utilities</p>
          <div class="utility-list">
            <button onclick={() => openStaticApp("logs")}>Open Logs</button>
            <button onclick={() => openStaticApp("settings")}>Open Settings</button>
            <button onclick={() => openStaticApp("app-store")}>Open App Store</button>
            <button onclick={() => openStaticApp("terminal")}>Open Terminal</button>
          </div>
        </div>
      </aside>
    </div>
  </div>
</section>

<style>
  .launcher {
    display: grid;
    grid-template-columns: 15.5rem 1fr;
    height: 100%;
    min-height: 0;
  }

  .rail {
    padding: 1rem;
    border-right: 1px solid var(--system-color-border);
    background: linear-gradient(180deg, hsla(var(--system-color-light-hsl) / 0.56), hsla(var(--system-color-light-hsl) / 0.72));
    display: grid;
    grid-template-rows: auto auto 1fr;
    gap: 1rem;
  }

  :global(body.dark) .rail {
    background: linear-gradient(180deg, hsla(225 16% 18% / 0.94), hsla(225 16% 14% / 0.96));
  }

  .brand {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.8rem;
    align-items: center;
  }

  .brand img {
    width: 2.9rem;
    height: 2.9rem;
  }

  .brand strong {
    display: block;
  }

  .brand span {
    color: var(--system-color-text-muted);
    font-size: 0.8rem;
  }

  nav {
    display: grid;
    gap: 0.35rem;
  }

  .rail-link {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 0.85rem;
    border-radius: 0.95rem;
    text-align: left;
    color: var(--system-color-text-muted);
  }

  .rail-link.active {
    background: linear-gradient(180deg, hsla(var(--system-color-primary-hsl) / 0.18), hsla(var(--system-color-primary-hsl) / 0.1));
    color: var(--system-color-primary);
  }

  .rail-link small {
    padding: 0.15rem 0.45rem;
    border-radius: 999px;
    background: var(--system-color-chip);
    color: inherit;
  }

  .rail-card,
  .panel,
  .detail-panel,
  .hero-card,
  .stat-card {
    border-radius: 1.15rem;
    border: 1px solid var(--system-color-border);
    background: var(--system-color-panel);
  }

  .rail-card {
    padding: 1rem;
    align-self: end;
  }

  .rail-card strong {
    display: block;
    margin-top: 0.4rem;
  }

  .rail-card p {
    margin: 0.5rem 0 0;
    font-size: 0.84rem;
    color: var(--system-color-text-muted);
  }

  .workspace-shell {
    display: grid;
    grid-template-rows: auto auto 1fr;
    min-height: 0;
  }

  .toolbar,
  .session-strip {
    padding: 0.9rem 1rem;
    display: flex;
    gap: 0.75rem;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--system-color-border);
  }

  .toolbar-left,
  .toolbar-right,
  .session-strip {
    display: flex;
    gap: 0.65rem;
    align-items: center;
  }

  .toolbar input {
    width: min(26rem, 100%);
    border-radius: 999px;
    border: 1px solid var(--system-color-border);
    padding: 0.75rem 1rem;
    background: hsla(var(--system-color-light-hsl) / 0.84);
    color: var(--system-color-text);
  }

  .toolbar button,
  .session-pill,
  .row-actions button,
  .hero-actions button,
  .detail-actions button,
  .utility-list button,
  .show-more button,
  .session-actions button {
    border-radius: 999px;
    padding: 0.65rem 0.95rem;
    border: 1px solid var(--system-color-border);
    background: hsla(var(--system-color-light-hsl) / 0.82);
  }

  .session-strip {
    justify-content: flex-start;
    overflow: auto;
  }

  .session-pill {
    display: flex;
    gap: 0.45rem;
    align-items: center;
    white-space: nowrap;
  }

  .session-pill small {
    color: var(--system-color-text-muted);
  }

  .session-pill.active {
    background: linear-gradient(180deg, hsla(var(--system-color-primary-hsl) / 0.18), hsla(var(--system-color-primary-hsl) / 0.08));
    border-color: hsla(var(--system-color-primary-hsl) / 0.22);
    color: var(--system-color-primary);
  }

  .workspace {
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) 20rem;
    gap: 1rem;
    padding: 1rem;
  }

  .center,
  .detail-pane {
    min-height: 0;
  }

  .center {
    overflow: auto;
    display: grid;
    align-content: start;
    gap: 1rem;
  }

  .detail-pane {
    overflow: auto;
    display: grid;
    align-content: start;
    gap: 1rem;
  }

  .overview-grid {
    display: grid;
    gap: 1rem;
  }

  .hero-card {
    padding: 1.3rem;
    background:
      radial-gradient(circle at top right, hsla(var(--system-color-primary-hsl) / 0.18), transparent 36%),
      var(--system-color-panel);
  }

  .hero-card h2,
  .session-headline h2 {
    margin: 0.35rem 0 0.55rem;
    font-size: 1.9rem;
    line-height: 1.02;
  }

  .hero-copy,
  .detail-copy {
    margin: 0;
    color: var(--system-color-text-muted);
  }

  .hero-actions,
  .detail-actions,
  .utility-list,
  .session-actions {
    display: flex;
    gap: 0.65rem;
    flex-wrap: wrap;
    margin-top: 1rem;
  }

  .stat-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.8rem;
  }

  .stat-card {
    padding: 1rem;
  }

  .stat-card span,
  .detail-meta span,
  .detail-card span {
    display: block;
    font-size: 0.74rem;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    color: var(--system-color-text-muted);
  }

  .stat-card strong,
  .detail-card strong {
    display: block;
    margin-top: 0.45rem;
    font-size: 1.15rem;
  }

  .panel,
  .detail-panel {
    padding: 1rem;
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-end;
    margin-bottom: 0.85rem;
  }

  .panel-header h3,
  .detail-panel h3 {
    margin: 0.32rem 0 0;
    font-size: 1.15rem;
  }

  .row-list {
    display: grid;
    gap: 0.7rem;
  }

  .row-button,
  .app-row {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: 0.8rem;
    padding: 0.9rem;
    border-radius: 1rem;
    border: 1px solid var(--system-color-border);
    background: hsla(var(--system-color-light-hsl) / 0.58);
  }

  .app-main {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.9rem;
    align-items: center;
    text-align: left;
  }

  .icon {
    width: 2.65rem;
    height: 2.65rem;
    border-radius: 0.8rem;
    background: linear-gradient(180deg, hsla(var(--system-color-primary-hsl) / 0.16), hsla(var(--system-color-primary-hsl) / 0.06));
    display: grid;
    place-items: center;
    font-weight: 700;
    color: var(--system-color-primary);
  }

  .app-main strong,
  .row-button strong {
    display: block;
    margin-bottom: 0.25rem;
  }

  .app-main p,
  .row-button p {
    margin: 0;
    color: var(--system-color-text-muted);
    font-size: 0.82rem;
  }

  .row-actions {
    display: flex;
    gap: 0.55rem;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .detail-meta {
    display: grid;
    gap: 0.7rem;
    margin-top: 1rem;
  }

  .detail-meta div {
    display: grid;
    gap: 0.2rem;
  }

  .tag-list {
    display: flex;
    gap: 0.45rem;
    flex-wrap: wrap;
    margin-top: 1rem;
  }

  .tag-list span {
    border-radius: 999px;
    padding: 0.35rem 0.65rem;
    background: var(--system-color-chip);
    font-size: 0.76rem;
    color: var(--system-color-text-muted);
  }

  .detail-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.8rem;
  }

  .detail-card {
    padding: 1rem;
    border-radius: 1rem;
    background: hsla(var(--system-color-light-hsl) / 0.58);
    border: 1px solid var(--system-color-border);
  }

  .session-headline {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
  }

  .session-surface {
    height: min(64vh, 42rem);
  }

  .notice,
  .empty-panel {
    padding: 0.9rem 1rem;
    border-radius: 1rem;
    border: 1px solid hsla(var(--system-color-warning) / 0.22);
    background: hsla(36 92% 52% / 0.08);
    color: var(--system-color-text-muted);
  }

  .empty-detail {
    min-height: 13rem;
  }

  .eyebrow {
    margin: 0;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.14em;
    color: var(--system-color-text-muted);
  }

  @media (max-width: 1200px) {
    .workspace {
      grid-template-columns: 1fr;
    }

    .detail-pane {
      grid-template-columns: repeat(2, minmax(0, 1fr));
      display: grid;
    }
  }

  @media (max-width: 980px) {
    .launcher {
      grid-template-columns: 1fr;
    }

    .rail {
      grid-template-columns: 1fr;
    }

    .stat-grid,
    .detail-grid,
    .detail-pane {
      grid-template-columns: 1fr;
    }
  }
</style>
