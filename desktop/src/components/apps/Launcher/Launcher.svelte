<svelte:options runes={true} />

<script lang="ts">
  import { createMutation } from "@tanstack/svelte-query";
  import {
    installAndFocusCatalog,
    openSessionInNewOsWindow,
    revealOrLaunchManifest,
    stopManifest,
    type OpenTarget,
  } from "$lib/session-actions";
  import type { AppManifest, InstalledApp } from "$lib/api/types";
  import { uploadApp } from "$lib/api/backend";
  import { open } from "@tauri-apps/plugin-dialog";
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
  import CommandCenter from "🍎/components/apps/CommandCenter/CommandCenter.svelte";
  import Chat from "🍎/components/apps/Chat/Chat.svelte";
  import JsonPlayground from "🍎/components/apps/JsonPlayground/JsonPlayground.svelte";
  import ModelSettings from "🍎/components/apps/ModelSettings/ModelSettings.svelte";
  import Plugins from "🍎/components/apps/Plugins/Plugins.svelte";
  import Channels from "🍎/components/apps/Channels/Channels.svelte";
  import McpTools from "🍎/components/apps/McpTools/McpTools.svelte";
  import Skills from "🍎/components/apps/Skills/Skills.svelte";
  import Knowledge from "🍎/components/apps/Knowledge/Knowledge.svelte";
  import CodeEditor from "🍎/components/apps/CodeEditor/CodeEditor.svelte";
  import Architecture from "🍎/components/apps/Architecture/Architecture.svelte";
  import OpenWithMenu from "🍎/components/Desktop/OpenWithMenu.svelte";
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
    toggleDefaultSessionMode,
    type LauncherSection,
  } from "🍎/state/desktop.svelte";

  let visibleLimit = $state(12);
  function getSavedCatalogLayout(): "grid" | "list" {
    try {
      const saved = localStorage.getItem("ai-launcher:catalog-layout");
      if (saved === "grid" || saved === "list") return saved;
    } catch {}
    return "grid";
  }

  let catalogLayout = $state<"grid" | "list">(getSavedCatalogLayout());

  function setCatalogLayout(layout: "grid" | "list") {
    catalogLayout = layout;
    try {
      localStorage.setItem("ai-launcher:catalog-layout", layout);
    } catch {}
  }

  const storeGradients = [
    "linear-gradient(160deg, hsl(215 100% 58%), hsl(191 86% 48%) 58%, hsl(171 77% 42%))",
    "linear-gradient(160deg, hsl(261 82% 61%), hsl(215 92% 58%) 58%, hsl(188 88% 46%))",
    "linear-gradient(160deg, hsl(16 100% 63%), hsl(346 83% 60%) 58%, hsl(281 74% 58%))",
    "linear-gradient(160deg, hsl(148 68% 44%), hsl(191 87% 43%) 58%, hsl(214 100% 55%))",
  ];

  const sections: { id: LauncherSection; label: string; group?: string }[] = [
    { id: "overview", label: "Overview" },
    { id: "command-center", label: "Command Center" },
    { id: "catalog", label: "Catalog" },
    { id: "installed", label: "Installed" },
    { id: "running", label: "Running" },
    { id: "server", label: "Server & System" },
  ];

  const appSections: { id: LauncherSection; label: string }[] = [
    { id: "chat", label: "Chat" },
    { id: "playground", label: "JSON Playground" },
    { id: "model-settings", label: "LLM Providers" },
    { id: "plugins", label: "Plugins" },
    { id: "channels", label: "Channels" },
    { id: "mcp-tools", label: "MCP Tools" },
    { id: "skills", label: "Skills" },
    { id: "knowledge", label: "Knowledge" },
    { id: "code-editor", label: "Code Editor" },
    { id: "architecture", label: "Architecture" },
  ];

  type PendingAction =
    | "install"
    | "launch-dashboard"
    | "launch-window"
    | "stop"
    | "uninstall";

  const installMutation = createMutation(() => ({
    mutationKey: ["launcher", "install"],
    mutationFn: async (app: AppManifest) => {
      await installAndFocusCatalog(app);
      return app.id;
    },
  }));

  const launchMutation = createMutation(() => ({
    mutationKey: ["launcher", "launch"],
    mutationFn: async ({
      app,
      installed,
      target,
    }: {
      app: AppManifest;
      installed: InstalledApp | null;
      target: OpenTarget;
    }) => {
      await revealOrLaunchManifest(app, installed, target);
      return { appId: app.id, target };
    },
  }));

  const stopMutation = createMutation(() => ({
    mutationKey: ["launcher", "stop"],
    mutationFn: async (installedApp: InstalledApp) => {
      await stopManifest(installedApp);
      return installedApp.manifest.id;
    },
  }));

  const uninstallMutation = createMutation(() => ({
    mutationKey: ["launcher", "uninstall"],
    mutationFn: async (app: AppManifest) => {
      await uninstallApp(app.id);
      return app.id;
    },
  }));

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
  const displayVersion = (value: string | null | undefined) => value?.trim() || "Not detected";

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
    try {
      await launchMutation.mutateAsync({
        app,
        installed: $installedMap[app.id] ?? null,
        target,
      });
      selectManifest(app.id);
    } catch {}
  }

  async function installManifest(app: AppManifest) {
    try {
      await installMutation.mutateAsync(app);
    } catch {}
  }

  async function stopInstalledApp(installedApp: InstalledApp) {
    try {
      await stopMutation.mutateAsync(installedApp);
    } catch {}
  }

  async function uninstallManifest(app: AppManifest) {
    try {
      await uninstallMutation.mutateAsync(app);
    } catch {}
  }

  function selectSection(section: LauncherSection) {
    selectLauncherSection(section);
  }

  function showManifestDetails(appId: string) {
    selectManifest(appId);
  }

  function showSession(sessionId: string) {
    focusSessionDetails(sessionId);
    const session = getSessionById(sessionId);
    if (session?.mode === "windowed") {
      openSessionInWindow(sessionId);
    } else {
      openSessionInDashboard(sessionId);
    }
  }

  async function defaultTileAction(app: AppManifest) {
    const installedApp = $installedMap[app.id] ?? null;
    if (!installedApp) {
      selectSection("catalog");
      selectManifest(app.id);
      return;
    }
    await launchManifest(app, desktop.default_session_mode);
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

  function appCategory(app: AppManifest) {
    return app.tags[0]?.replace(/-/g, " ") ?? "AI app";
  }

  function appGradient(app: AppManifest) {
    const seed = Array.from(app.id).reduce((sum, char) => sum + char.charCodeAt(0), 0);
    return storeGradients[seed % storeGradients.length];
  }

  function statusLabel(app: AppManifest) {
    const status = appStatus(app);
    return status.charAt(0).toUpperCase() + status.slice(1);
  }

  function pendingActionFor(appId: string): PendingAction | null {
    const installVariables = installMutation.variables;
    if (installMutation.isPending && installVariables?.id === appId) {
      return "install";
    }

    const launchVariables = launchMutation.variables;
    if (launchMutation.isPending && launchVariables?.app.id === appId) {
      return launchVariables.target === "embedded" ? "launch-dashboard" : "launch-window";
    }

    const stopVariables = stopMutation.variables;
    if (stopMutation.isPending && stopVariables?.manifest.id === appId) {
      return "stop";
    }

    const uninstallVariables = uninstallMutation.variables;
    if (uninstallMutation.isPending && uninstallVariables?.id === appId) {
      return "uninstall";
    }

    return null;
  }

  function isAppPending(appId: string) {
    return pendingActionFor(appId) !== null;
  }

  function installLabel(appId: string) {
    return pendingActionFor(appId) === "install" ? "Installing..." : "Install";
  }

  function launchLabel(appId: string, target: OpenTarget) {
    const expectedAction = target === "embedded" ? "launch-dashboard" : "launch-window";
    if (pendingActionFor(appId) === expectedAction) {
      return "Opening...";
    }
    return target === "embedded" ? "Open in Dashboard" : "Open in Window";
  }

  function stopLabel(appId: string) {
    return pendingActionFor(appId) === "stop" ? "Stopping..." : "Stop";
  }

  function uninstallLabel(appId: string) {
    return pendingActionFor(appId) === "uninstall" ? "Uninstalling..." : "Uninstall";
  }

  let isUploading = $state(false);

  async function handleUploadFolder() {
    try {
      if (!("__TAURI_INTERNALS__" in window)) {
        alert("Upload is only supported in the desktop app.");
        return;
      }
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select App Folder"
      });
      if (selected === null || typeof selected !== "string") return;

      isUploading = true;

      const result = await uploadApp({
        source_type: "folder",
        source_path: selected
      });

      if (!result.accepted) {
        const errorMsg = result.validation_errors.map(e => `* ${e.field}: ${e.message}`).join("\n");
        alert(`Upload Invalid:\n${errorMsg}\n\nLogs:\n${result.install_log.slice(-10).join('\n')}`);
      } else {
        alert(`Successfully uploaded ${result.app_name || "app"} (${result.app_id})`);
        await refreshWorkspace();
      }
    } catch (e: any) {
      alert(`Error uploading: ${e.message}`);
    } finally {
      isUploading = false;
    }
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

      <span class="rail-divider">Apps</span>
      {#each appSections as section}
        <button
          class="rail-link"
          class:active={desktop.launcher_section === section.id && desktop.workspace_view.kind === "dashboard"}
          onclick={() => selectSection(section.id)}
        >
          <span>{section.label}</span>
        </button>
      {/each}
    </nav>

    <div class="rail-card">
      <div style="display: flex; justify-content: space-between; align-items: start;">
        <span class="eyebrow">Default open</span>
        <button class="toggle-btn" tabindex="-1" onclick={toggleDefaultSessionMode} style="background: transparent; border: 1px solid var(--system-color-border); color: var(--system-color-text-muted); padding: 0.15rem 0.45rem; font-size: 0.7rem; border-radius: 999px; cursor: default;">Toggle</button>
      </div>
      <strong>{desktop.default_session_mode === "embedded" ? "Dashboard first" : "Window first"}</strong>
      <p>{desktop.default_session_mode === "embedded" 
        ? "Primary actions focus the main workspace. Use the secondary action to pop out into a browser window." 
        : "Primary actions open in a separate browser window. Use the secondary action to view in the dashboard."}</p>
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
        <button onclick={() => openGenericBrowserWindow()}>New Browser</button>
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
              <OpenWithMenu session_id={currentSession.id} port={currentSession.port} title={currentSession.title} />
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
        {:else if desktop.launcher_section === "command-center"}
          <CommandCenter />
        {:else if desktop.launcher_section === "chat"}
          <div class="embedded-app"><Chat /></div>
        {:else if desktop.launcher_section === "playground"}
          <div class="embedded-app"><JsonPlayground /></div>
        {:else if desktop.launcher_section === "model-settings"}
          <div class="embedded-app"><ModelSettings /></div>
        {:else if desktop.launcher_section === "plugins"}
          <div class="embedded-app"><Plugins /></div>
        {:else if desktop.launcher_section === "channels"}
          <div class="embedded-app"><Channels /></div>
        {:else if desktop.launcher_section === "mcp-tools"}
          <div class="embedded-app"><McpTools /></div>
        {:else if desktop.launcher_section === "skills"}
          <div class="embedded-app"><Skills /></div>
        {:else if desktop.launcher_section === "knowledge"}
          <div class="embedded-app"><Knowledge /></div>
        {:else if desktop.launcher_section === "code-editor"}
          <div class="embedded-app"><CodeEditor /></div>
        {:else if desktop.launcher_section === "architecture"}
          <div class="embedded-app"><Architecture /></div>
        {:else if desktop.launcher_section === "catalog"}
          <div class="panel catalog-panel">
            <div class="panel-header catalog-header">
              <div>
                <p class="eyebrow">Catalog</p>
                <h3>Explore AI apps</h3>
              </div>
              <div class="catalog-header-actions">
                <button 
                  onclick={handleUploadFolder} 
                  disabled={isUploading} 
                  class="action-btn upload-btn"
                >
                  {isUploading ? "Uploading..." : "Upload Folder"}
                </button>
                <span>{filteredCatalog.length} result(s)</span>
                <div class="catalog-layout-toggle" aria-label="Catalog layout">
                  <button
                    type="button"
                    class:active={catalogLayout === "grid"}
                    aria-pressed={catalogLayout === "grid"}
                    data-catalog-layout="grid"
                    onclick={() => setCatalogLayout("grid")}
                  >
                    Grid
                  </button>
                  <button
                    type="button"
                    class:active={catalogLayout === "list"}
                    aria-pressed={catalogLayout === "list"}
                    data-catalog-layout="list"
                    onclick={() => setCatalogLayout("list")}
                  >
                    List
                  </button>
                </div>
              </div>
            </div>
            {#if catalogLayout === "grid"}
              <div class="catalog-grid" data-catalog-surface data-layout="grid">
                {#each visibleCatalog as app (app.id)}
                  <article class="catalog-card" data-app-id={app.id}>
                    <button class="catalog-card-main" onclick={() => showManifestDetails(app.id)}>
                      <div class="catalog-card-art" style={`background:${appGradient(app)}`}>
                        <div class="catalog-card-topline">
                          <span class="catalog-card-category">{appCategory(app)}</span>
                          <span class={`catalog-status ${appStatus(app)}`}>{statusLabel(app)}</span>
                        </div>
                        <div class="catalog-card-copy">
                          <span class="catalog-card-id">{app.id}</span>
                          <strong>{app.name}</strong>
                          <p>{app.description}</p>
                        </div>
                      </div>
                    </button>
                    <div class="catalog-card-footer">
                      <div class="catalog-card-meta">
                        {#each app.tags.slice(0, 3) as tag}
                          <span>{tag}</span>
                        {/each}
                        <span>{app.disk_size}</span>
                      </div>
                      <div class="catalog-card-actions">
                        {#if !$installedMap[app.id]}
                          <button disabled={isAppPending(app.id)} aria-busy={pendingActionFor(app.id) === "install"} onclick={() => installManifest(app)}>
                            {installLabel(app.id)}
                          </button>
                        {:else}
                          <button disabled={isAppPending(app.id)} aria-busy={pendingActionFor(app.id) === "launch-dashboard"} onclick={() => launchManifest(app, "embedded")}>
                            {launchLabel(app.id, "embedded")}
                          </button>
                          <button disabled={isAppPending(app.id)} aria-busy={pendingActionFor(app.id) === "launch-window"} onclick={() => launchManifest(app, "windowed")}>
                            {launchLabel(app.id, "windowed")}
                          </button>
                        {/if}
                      </div>
                    </div>
                  </article>
                {/each}
              </div>
            {:else}
              <div class="catalog-list" data-catalog-surface data-layout="list">
                {#each visibleCatalog as app (app.id)}
                  <article class="catalog-list-row" data-app-id={app.id}>
                    <button class="catalog-list-main" onclick={() => showManifestDetails(app.id)}>
                      <div class="catalog-list-art" style={`background:${appGradient(app)}`}>
                        <span>{app.icon ?? app.name.slice(0, 2).toUpperCase()}</span>
                      </div>
                      <div class="catalog-list-copy">
                        <div class="catalog-list-head">
                          <div>
                            <small>{appCategory(app)}</small>
                            <strong>{app.name}</strong>
                          </div>
                          <span class={`catalog-status ${appStatus(app)}`}>{statusLabel(app)}</span>
                        </div>
                        <p>{app.description}</p>
                        <div class="catalog-card-meta">
                          {#each app.tags.slice(0, 3) as tag}
                            <span>{tag}</span>
                          {/each}
                          <span>{app.disk_size}</span>
                        </div>
                      </div>
                    </button>
                    <div class="catalog-list-actions">
                      {#if !$installedMap[app.id]}
                        <button disabled={isAppPending(app.id)} aria-busy={pendingActionFor(app.id) === "install"} onclick={() => installManifest(app)}>
                          {installLabel(app.id)}
                        </button>
                      {:else}
                        <button disabled={isAppPending(app.id)} aria-busy={pendingActionFor(app.id) === "launch-dashboard"} onclick={() => launchManifest(app, "embedded")}>
                          {launchLabel(app.id, "embedded")}
                        </button>
                        <button disabled={isAppPending(app.id)} aria-busy={pendingActionFor(app.id) === "launch-window"} onclick={() => launchManifest(app, "windowed")}>
                          {launchLabel(app.id, "windowed")}
                        </button>
                      {/if}
                    </div>
                  </article>
                {/each}
              </div>
            {/if}
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
                    <button disabled={isAppPending(app.manifest.id)} aria-busy={pendingActionFor(app.manifest.id) === "launch-dashboard"} onclick={() => launchManifest(app.manifest, "embedded")}>{launchLabel(app.manifest.id, "embedded")}</button>
                    <button disabled={isAppPending(app.manifest.id)} aria-busy={pendingActionFor(app.manifest.id) === "launch-window"} onclick={() => launchManifest(app.manifest, "windowed")}>{launchLabel(app.manifest.id, "windowed")}</button>
                    <button disabled={isAppPending(app.manifest.id)} aria-busy={pendingActionFor(app.manifest.id) === "uninstall"} onclick={() => uninstallManifest(app.manifest)}>{uninstallLabel(app.manifest.id)}</button>
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
                    <OpenWithMenu session_id={session.id} port={session.port} title={session.title} />
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
                  <strong>{displayVersion($systemInfo.uv.uv_version)}</strong>
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
                <button disabled={isAppPending(selectedManifest.id)} aria-busy={pendingActionFor(selectedManifest.id) === "install"} onclick={() => installManifest(selectedManifest)}>{installLabel(selectedManifest.id)}</button>
              {:else if selectedInstalled.status.state === "Running"}
                <button disabled={isAppPending(selectedManifest.id)} aria-busy={pendingActionFor(selectedManifest.id) === "launch-dashboard"} onclick={() => launchManifest(selectedManifest, "embedded")}>{launchLabel(selectedManifest.id, "embedded")}</button>
                <button disabled={isAppPending(selectedManifest.id)} aria-busy={pendingActionFor(selectedManifest.id) === "launch-window"} onclick={() => launchManifest(selectedManifest, "windowed")}>{launchLabel(selectedManifest.id, "windowed")}</button>
                <button disabled={isAppPending(selectedManifest.id)} aria-busy={pendingActionFor(selectedManifest.id) === "stop"} onclick={() => stopInstalledApp(selectedInstalled)}>{stopLabel(selectedManifest.id)}</button>
                <button disabled={isAppPending(selectedManifest.id)} aria-busy={pendingActionFor(selectedManifest.id) === "uninstall"} onclick={() => uninstallManifest(selectedManifest)}>{uninstallLabel(selectedManifest.id)}</button>
                {#if selectedInstalled.status.port}
                  {@const inspSession = desktop.sessions.find(s => s.app_id === selectedManifest.id)}
                  {#if inspSession}
                    <OpenWithMenu session_id={inspSession.id} port={selectedInstalled.status.port!} title={selectedManifest.name} />
                  {/if}
                {/if}
              {:else}
                <button disabled={isAppPending(selectedManifest.id)} aria-busy={pendingActionFor(selectedManifest.id) === "launch-dashboard"} onclick={() => launchManifest(selectedManifest, "embedded")}>{launchLabel(selectedManifest.id, "embedded")}</button>
                <button disabled={isAppPending(selectedManifest.id)} aria-busy={pendingActionFor(selectedManifest.id) === "launch-window"} onclick={() => launchManifest(selectedManifest, "windowed")}>{launchLabel(selectedManifest.id, "windowed")}</button>
                <button disabled={isAppPending(selectedManifest.id)} aria-busy={pendingActionFor(selectedManifest.id) === "uninstall"} onclick={() => uninstallManifest(selectedManifest)}>{uninstallLabel(selectedManifest.id)}</button>
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

  .upload-btn {
    padding: 0.3rem 0.6rem;
    font-size: 0.8rem;
    border-radius: 0.4rem;
    background: var(--system-color-accent);
    color: white;
    border: none;
    cursor: pointer;
  }
  .upload-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .rail {
    padding: 1rem;
    border-right: 1px solid var(--system-color-border);
    background: linear-gradient(180deg, hsla(var(--system-color-light-hsl) / 0.56), hsla(var(--system-color-light-hsl) / 0.72));
    display: grid;
    grid-template-rows: auto 1fr auto;
    gap: 1rem;
    overflow: hidden;
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
    overflow-y: auto;
    align-content: start;
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

  .rail-divider {
    display: block;
    margin-top: 0.6rem;
    padding: 0.4rem 0.85rem 0.25rem;
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.14em;
    color: var(--system-color-text-muted);
  }

  .embedded-app {
    height: 100%;
    min-height: 0;
    overflow: auto;
    border-radius: 1.15rem;
    border: 1px solid var(--system-color-border);
    background: var(--system-color-panel);
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
  .catalog-layout-toggle button,
  .catalog-card-actions button,
  .catalog-list-actions button,
  .hero-actions button,
  .detail-actions button,
  .utility-list button,
  .show-more button {
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

  .catalog-panel {
    background:
      radial-gradient(circle at top left, hsla(var(--system-color-primary-hsl) / 0.16), transparent 30%),
      linear-gradient(180deg, hsla(var(--system-color-light-hsl) / 0.72), hsla(var(--system-color-light-hsl) / 0.92)),
      var(--system-color-panel);
  }

  :global(body.dark) .catalog-panel {
    background:
      radial-gradient(circle at top left, hsla(var(--system-color-primary-hsl) / 0.2), transparent 34%),
      linear-gradient(180deg, hsla(223 18% 18% / 0.96), hsla(224 18% 15% / 0.98)),
      var(--system-color-panel);
  }

  .catalog-header {
    align-items: center;
  }

  .catalog-header-actions {
    display: flex;
    gap: 0.8rem;
    align-items: center;
  }

  .catalog-layout-toggle {
    display: inline-flex;
    gap: 0.25rem;
    padding: 0.24rem;
    border-radius: 999px;
    border: 1px solid var(--system-color-border);
    background: hsla(var(--system-color-light-hsl) / 0.62);
  }

  .catalog-layout-toggle button {
    padding: 0.48rem 0.85rem;
    background: transparent;
    border-color: transparent;
    font-size: 0.8rem;
  }

  .catalog-layout-toggle button.active {
    background: linear-gradient(180deg, hsla(var(--system-color-primary-hsl) / 0.18), hsla(var(--system-color-primary-hsl) / 0.08));
    border-color: hsla(var(--system-color-primary-hsl) / 0.2);
    color: var(--system-color-primary);
  }

  .catalog-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(16.5rem, 1fr));
    gap: 1rem;
  }

  .catalog-card,
  .catalog-list-row {
    border-radius: 1.25rem;
    border: 1px solid var(--system-color-border);
    background: var(--system-color-panel-solid);
    overflow: hidden;
    box-shadow: 0 18px 40px hsla(220 24% 12% / 0.08);
  }

  .catalog-card-main,
  .catalog-list-main {
    text-align: left;
  }

  .catalog-card-art {
    position: relative;
    min-height: 17.5rem;
    padding: 1rem;
    display: grid;
    align-content: space-between;
    gap: 1rem;
    color: white;
    overflow: hidden;
  }

  .catalog-card-art::before,
  .catalog-list-art::before {
    content: "";
    position: absolute;
    inset: auto -12% -22% auto;
    width: 9rem;
    height: 9rem;
    border-radius: 999px;
    background: hsla(var(--system-color-light-hsl) / 0.22);
    filter: blur(10px);
  }

  .catalog-card-topline,
  .catalog-list-head {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    align-items: flex-start;
  }

  .catalog-card-category,
  .catalog-card-id,
  .catalog-list-copy small {
    display: block;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.14em;
  }

  .catalog-card-category,
  .catalog-card-id {
    color: hsla(var(--system-color-light-hsl) / 0.82);
  }

  .catalog-card-copy,
  .catalog-card-topline {
    position: relative;
    z-index: 1;
  }

  .catalog-card-copy strong {
    display: block;
    margin-top: 0.38rem;
    font-size: 1.55rem;
    line-height: 1.02;
  }

  .catalog-card-copy p {
    margin: 0.55rem 0 0;
    max-width: 18rem;
    color: hsla(var(--system-color-light-hsl) / 0.88);
    line-height: 1.45;
  }

  .catalog-status {
    border-radius: 999px;
    padding: 0.35rem 0.65rem;
    font-size: 0.72rem;
    font-weight: 600;
    white-space: nowrap;
    background: hsla(var(--system-color-light-hsl) / 0.18);
    color: white;
    border: 1px solid hsla(var(--system-color-light-hsl) / 0.24);
  }

  .catalog-status.installed {
    background: hsla(211 100% 50% / 0.18);
  }

  .catalog-status.running {
    background: hsla(142 64% 42% / 0.2);
  }

  .catalog-card-footer,
  .catalog-list-actions {
    padding: 0.95rem 1rem 1rem;
  }

  .catalog-card-footer {
    display: grid;
    gap: 0.9rem;
  }

  .catalog-card-meta {
    display: flex;
    gap: 0.45rem;
    flex-wrap: wrap;
  }

  .catalog-card-meta span {
    border-radius: 999px;
    padding: 0.32rem 0.62rem;
    background: var(--system-color-chip);
    color: var(--system-color-text-muted);
    font-size: 0.74rem;
  }

  .catalog-card-actions,
  .catalog-list-actions {
    display: flex;
    gap: 0.55rem;
    flex-wrap: wrap;
  }

  .catalog-list {
    display: grid;
    gap: 0.8rem;
  }

  .catalog-list-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 1rem;
    align-items: center;
  }

  .catalog-list-main {
    display: grid;
    grid-template-columns: 6rem minmax(0, 1fr);
    gap: 1rem;
    align-items: stretch;
    padding: 0.9rem;
  }

  .catalog-list-art {
    position: relative;
    min-height: 7.25rem;
    border-radius: 1rem;
    display: grid;
    place-items: center;
    color: white;
    font-size: 1.2rem;
    font-weight: 700;
    overflow: hidden;
  }

  .catalog-list-copy {
    display: grid;
    gap: 0.55rem;
    align-content: center;
  }

  .catalog-list-copy strong {
    display: block;
    margin-top: 0.22rem;
    font-size: 1.1rem;
  }

  .catalog-list-copy p {
    margin: 0;
    color: var(--system-color-text-muted);
    line-height: 1.45;
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

    .catalog-list-row {
      grid-template-columns: 1fr;
    }

    .catalog-list-actions {
      padding-top: 0;
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

    .catalog-header,
    .catalog-header-actions,
    .catalog-list-head {
      align-items: flex-start;
      flex-direction: column;
    }

    .catalog-grid {
      grid-template-columns: 1fr;
    }

    .catalog-list-main {
      grid-template-columns: 1fr;
    }

    .catalog-list-art {
      min-height: 8.5rem;
    }
  }
</style>
