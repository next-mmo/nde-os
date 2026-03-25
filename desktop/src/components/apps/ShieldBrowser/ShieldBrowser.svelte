<svelte:options runes={true} />

<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import type { UnlistenFn } from "@tauri-apps/api/event";

  interface ShieldProfile {
    id: string;
    name: string;
    engine: string;
    engine_version: string;
    is_running: boolean;
    last_launch: number | null;
    created_at: number;
    tags: string[];
    note: string | null;
    has_proxy: boolean;
    fingerprint_os: string | null;
  }

  interface ShieldStatus {
    total_profiles: number;
    running_profiles: number;
    installed_engines: { engine: string; version: string }[];
  }

  interface AvailableEngine {
    engine: string;
    name: string;
    description: string;
    available: boolean;
    icon: string;
  }

  let profiles = $state<ShieldProfile[]>([]);
  let status = $state<ShieldStatus | null>(null);
  let availableEngines = $state<AvailableEngine[]>([]);
  let loading = $state(true);
  let view = $state<"setup" | "profiles" | "create" | "settings">("profiles");
  let selectedProfile = $state<ShieldProfile | null>(null);

  // Create form
  let newName = $state("");
  let newEngine = $state("camoufox");
  let newVersion = $state("");
  let resolvingVersion = $state(false);

  // Setup state
  let setupStep = $state<"choose" | "installing">("choose");
  let setupEngine = $state("camoufox");
  let setupVersion = $state("");
  let setupError = $state("");

  let downloading = $state(false);
  let downloadProgress = $state("");
  let downloadPercent = $state(0);
  let launching = $state(false);
  let settingsLatestVersion = $state("");
  let resolvingLatest = $state(false);

  // Track listener cleanup
  let stopListenerCleanup: UnlistenFn | null = null;

  $effect(() => { init(); });

  // Listen for browser process exit events (e.g. user closes camoufox)
  $effect(() => {
    let cancelled = false;
    listen<string>("shield-profile-stopped", (_event) => {
      if (!cancelled) {
        console.log("Browser process exited, refreshing...");
        refresh();
      }
    }).then((unlisten) => {
      if (cancelled) {
        unlisten();
      } else {
        stopListenerCleanup = unlisten;
      }
    });

    return () => {
      cancelled = true;
      stopListenerCleanup?.();
      stopListenerCleanup = null;
    };
  });

  async function init() {
    loading = true;
    try {
      // Load engines list
      availableEngines = await invoke<AvailableEngine[]>("get_available_engines");

      // Load data
      profiles = await invoke<ShieldProfile[]>("list_shield_profiles");
      status = await invoke<ShieldStatus>("get_shield_status");

      // Show setup if no engines installed, otherwise profiles
      if (status!.installed_engines.length === 0) {
        view = "setup";
        resolveSetupVersion();
      } else {
        view = "profiles";
      }
      // Pre-resolve version for create form in background
      resolveCreateVersion();
    } catch (e) {
      console.error("Failed to init shield:", e);
      profiles = [];
      status = { total_profiles: 0, running_profiles: 0, installed_engines: [] };
      view = "setup";
    } finally {
      loading = false;
    }
  }

  async function refresh() {
    try {
      profiles = await invoke<ShieldProfile[]>("list_shield_profiles");
      status = await invoke<ShieldStatus>("get_shield_status");
      if (selectedProfile) {
        const updated = profiles.find(p => p.id === selectedProfile!.id);
        if (updated) selectedProfile = updated;
      }
    } catch (e) {
      console.error("Failed to refresh:", e);
    }
  }

  async function resolveSetupVersion() {
    try {
      setupVersion = await invoke<string>("resolve_engine_version", { engine: setupEngine });
    } catch (e) {
      setupVersion = "";
      setupError = `Failed to resolve version: ${e}`;
    }
  }

  async function resolveCreateVersion() {
    resolvingVersion = true;
    try {
      newVersion = await invoke<string>("resolve_engine_version", { engine: newEngine });
    } catch (e) {
      newVersion = "";
    } finally {
      resolvingVersion = false;
    }
  }

  async function installEngine() {
    if (!setupVersion) {
      setupError = "No version resolved. Check your internet connection.";
      return;
    }
    setupStep = "installing";
    setupError = "";
    downloading = true;
    downloadPercent = 0;
    downloadProgress = "Downloading Camoufox binary…";

    let unlisten: UnlistenFn | null = null;
    try {
      unlisten = await listen<{ downloaded: number; total: number; percent: number }>("shield-download-progress", (event) => {
        const { downloaded, total, percent } = event.payload;
        downloadPercent = percent;
        const dlMB = (downloaded / 1048576).toFixed(1);
        const totalMB = total > 0 ? (total / 1048576).toFixed(0) : "?";
        downloadProgress = `Downloading — ${percent}%  (${dlMB} MB / ${totalMB} MB)`;
      });

      await invoke("download_shield_engine", {
        engine: setupEngine,
        version: setupVersion,
      });
      downloadPercent = 100;
      downloadProgress = "Extracting & installing…";
      await refresh();
      downloadProgress = "Installation complete!";
      // Transition to profiles after short delay
      setTimeout(() => {
        view = "profiles";
        downloading = false;
        downloadProgress = "";
        downloadPercent = 0;
        resolveCreateVersion();
      }, 800);
    } catch (e: unknown) {
      setupError = `Download failed: ${e}`;
      setupStep = "choose";
      downloading = false;
      downloadProgress = "";
      downloadPercent = 0;
    } finally {
      unlisten?.();
    }
  }

  async function createProfile() {
    if (!newName.trim() || !newVersion) return;
    try {
      await invoke("create_shield_profile", {
        name: newName.trim(),
        engine: newEngine,
        engineVersion: newVersion,
      });
      newName = "";
      view = "profiles";
      await refresh();
    } catch (e: unknown) {
      alert(`Failed to create profile: ${e}`);
    }
  }

  async function deleteProfile(id: string) {
    if (!confirm("Delete this profile and all its data?")) return;
    try {
      await invoke("delete_shield_profile", { id });
      if (selectedProfile?.id === id) selectedProfile = null;
      await refresh();
    } catch (e: unknown) {
      alert(`Failed to delete: ${e}`);
    }
  }

  async function downloadEngine(engine: string, version: string) {
    downloading = true;
    downloadPercent = 0;
    downloadProgress = "Downloading engine binary…";

    let unlisten: UnlistenFn | null = null;
    try {
      unlisten = await listen<{ downloaded: number; total: number; percent: number }>("shield-download-progress", (event) => {
        const { downloaded, total, percent } = event.payload;
        downloadPercent = percent;
        const dlMB = (downloaded / 1048576).toFixed(1);
        const totalMB = total > 0 ? (total / 1048576).toFixed(0) : "?";
        downloadProgress = `Downloading — ${percent}%  (${dlMB} MB / ${totalMB} MB)`;
      });

      await invoke("download_shield_engine", { engine, version });
      await refresh();
    } catch (e: unknown) {
      alert(`Download failed: ${e}`);
    } finally {
      unlisten?.();
      downloading = false;
      downloadProgress = "";
      downloadPercent = 0;
    }
  }

  async function launchProfile(id: string) {
    launching = true;
    try {
      const cdpPort = await invoke<number>("launch_shield_profile", { id, url: "https://browserleaks.com/canvas" });
      console.log(`Profile launched on CDP port: ${cdpPort}`);
      // Optimistically update the UI so the Stop button shows immediately
      if (selectedProfile && selectedProfile.id === id) {
        selectedProfile = { ...selectedProfile, is_running: true };
      }
      const idx = profiles.findIndex(p => p.id === id);
      if (idx >= 0) {
        profiles[idx] = { ...profiles[idx], is_running: true };
        profiles = profiles; // trigger reactivity
      }
      await refresh();
    } catch (e: unknown) {
      alert(`Failed to launch: ${e}`);
    } finally {
      launching = false;
    }
  }

  async function stopProfile(id: string) {
    try {
      await invoke("stop_shield_profile", { id });
      await refresh();
    } catch (e: unknown) {
      alert(`Failed to stop: ${e}`);
    }
  }

  function isEngineInstalled(engine: string, version: string): boolean {
    if (!status) return false;
    return status.installed_engines.some(e => e.engine === engine && e.version === version);
  }

  function engineIcon(engine: string) { return engine === "wayfern" ? "🌐" : "🦊"; }
  function engineLabel(engine: string) { return engine === "wayfern" ? "Wayfern (Chromium)" : "Camoufox (Firefox)"; }
  function formatDate(epoch: number) { return new Date(epoch * 1000).toLocaleDateString(); }

  async function removeEngine(engine: string, version: string) {
    if (!confirm(`Remove ${engineLabel(engine)} v${version}? You can reinstall it later.`)) return;
    try {
      await invoke("remove_shield_engine", { engine, version });
      await refresh();
    } catch (e: unknown) {
      alert(`Failed to remove engine: ${e}`);
    }
  }

  async function reinstallEngine(engine: string, version: string) {
    await removeEngine(engine, version);
    // After removal, trigger install
    await downloadEngine(engine, version);
    await refresh();
  }

  async function openSettings() {
    view = "settings";
    resolvingLatest = true;
    settingsLatestVersion = "";
    try {
      settingsLatestVersion = await invoke<string>("resolve_engine_version", { engine: "camoufox" });
    } catch { settingsLatestVersion = ""; }
    finally { resolvingLatest = false; }
  }
</script>

<section class="shield-app">
  {#if loading}
    <div class="loading-screen">
      <div class="spinner"></div>
      <p>Initializing Shield Browser...</p>
    </div>

  {:else if view === "setup"}
    <!-- ═══ Onboarding Setup ═══ -->
    <div class="setup-screen">
      <div class="setup-hero">
        <span class="setup-icon">🛡️</span>
        <h2>Set Up Shield Browser</h2>
        <p class="setup-subtitle">Install a browser engine to create anti-detect profiles with unique fingerprints.</p>
      </div>

      {#if setupStep === "choose"}
        <div class="engine-cards">
          {#each availableEngines as eng (eng.engine)}
            <button
              class="engine-card"
              class:selected={setupEngine === eng.engine}
              class:disabled={!eng.available}
              onclick={() => { if (eng.available) { setupEngine = eng.engine; resolveSetupVersion(); } }}
              disabled={!eng.available}
            >
              <span class="engine-card-icon">{eng.icon}</span>
              <div class="engine-card-body">
                <h4>{eng.name} {#if !eng.available}<span class="coming-soon">Coming Soon</span>{/if}</h4>
                <p>{eng.description}</p>
              </div>
              {#if eng.available && setupEngine === eng.engine}
                <span class="checkmark">✓</span>
              {/if}
            </button>
          {/each}
        </div>

        {#if setupVersion}
          <div class="setup-meta">
            <p>Latest version: <strong>v{setupVersion}</strong></p>
          </div>
        {/if}

        {#if setupError}
          <div class="setup-error">{setupError}</div>
        {/if}

        <button
          class="install-btn"
          onclick={installEngine}
          disabled={!setupVersion || downloading}
        >
          {setupVersion ? `Install Camoufox v${setupVersion}` : "Resolving latest version..."}
        </button>

        <button class="skip-btn" onclick={() => { view = "profiles"; resolveCreateVersion(); }}>
          Skip setup →
        </button>

      {:else if setupStep === "installing"}
        <div class="install-progress">
          <div class="spinner large"></div>
          <p class="progress-text">{downloadProgress}</p>
          {#if downloading && downloadPercent > 0}
            <div class="progress-bar-track">
              <div class="progress-bar-fill" style="width: {downloadPercent}%"></div>
            </div>
            <p class="progress-percent">{downloadPercent}%</p>
          {/if}
          {#if setupError}
            <div class="setup-error">{setupError}</div>
            <button class="action-btn" onclick={() => { setupStep = "choose"; setupError = ""; }}>← Try Again</button>
          {/if}
        </div>
      {/if}
    </div>

  {:else}
    <!-- ═══ Header ═══ -->
    <div class="header">
      <div>
        <p class="eyebrow">Anti-Detect Browser</p>
        <h2>🛡️ Shield Browser</h2>
      </div>
      <div class="header-actions">
        {#if view === "create" || view === "settings"}
          <button class="action-btn" onclick={() => view = "profiles"}>← Back</button>
        {:else}
          {#if status && status.installed_engines.length === 0}
            <button class="action-btn" onclick={() => { resolveSetupVersion(); view = "setup"; }}>🔧 Set Up Engine</button>
          {/if}
          <button class="action-btn" onclick={openSettings}>⚙️</button>
          <button class="action-btn primary" onclick={() => { resolveCreateVersion(); view = "create"; }}>+ New Profile</button>
          <button class="action-btn" onclick={refresh}>↻</button>
        {/if}
      </div>
    </div>

    <!-- ═══ Status Bar ═══ -->
    {#if status}
      <div class="status-bar">
        <div class="status-item">
          <span class="status-value">{status.total_profiles}</span>
          <span class="status-label">Profiles</span>
        </div>
        <div class="status-item">
          <span class="status-value running">{status.running_profiles}</span>
          <span class="status-label">Running</span>
        </div>
        <div class="status-item">
          <span class="status-value">{status.installed_engines.length}</span>
          <span class="status-label">Engines</span>
        </div>
      </div>
    {/if}

    <!-- ═══ Main Content ═══ -->
    <div class="main-content">
      {#if view === "settings"}
        <!-- ═══ Settings / Engine Management ═══ -->
        <div class="settings-panel">
          <h3>⚙️ Engine Management</h3>
          <p class="settings-subtitle">Manage installed browser engines, check for updates, or reinstall.</p>

          {#if status && status.installed_engines.length > 0}
            <div class="engine-list">
              {#each status.installed_engines as eng}
                <div class="engine-row">
                  <div class="engine-row-icon">{engineIcon(eng.engine)}</div>
                  <div class="engine-row-info">
                    <span class="engine-row-name">{engineLabel(eng.engine)}</span>
                    <span class="engine-row-version">v{eng.version}</span>
                    {#if settingsLatestVersion && eng.version !== settingsLatestVersion}
                      <span class="update-badge">Update available: v{settingsLatestVersion}</span>
                    {:else if settingsLatestVersion && eng.version === settingsLatestVersion}
                      <span class="up-to-date-badge">✓ Up to date</span>
                    {:else if resolvingLatest}
                      <span class="engine-row-version">Checking…</span>
                    {/if}
                  </div>
                  <div class="engine-row-actions">
                    {#if settingsLatestVersion && eng.version !== settingsLatestVersion}
                      <button class="action-btn primary" onclick={() => downloadEngine(eng.engine, settingsLatestVersion)} disabled={downloading}>
                        {downloading ? downloadProgress : `⬆ Update to v${settingsLatestVersion}`}
                      </button>
                    {/if}
                    <button class="action-btn" onclick={() => reinstallEngine(eng.engine, eng.version)} disabled={downloading}>
                      🔄 Reinstall
                    </button>
                    <button class="action-btn danger" onclick={() => removeEngine(eng.engine, eng.version)} disabled={downloading}>
                      🗑 Remove
                    </button>
                  </div>
                </div>
              {/each}
            </div>

            {#if downloading && downloadPercent > 0}
              <div class="settings-progress">
                <p class="progress-text">{downloadProgress}</p>
                <div class="progress-bar-track">
                  <div class="progress-bar-fill" style="width: {downloadPercent}%"></div>
                </div>
              </div>
            {/if}
          {:else}
            <div class="settings-empty">
              <p>No engines installed.</p>
              <button class="action-btn primary" onclick={() => { resolveSetupVersion(); view = "setup"; }}>🔧 Install Engine</button>
            </div>
          {/if}

          <div class="settings-section">
            <h4>Add Another Engine</h4>
            <button class="action-btn primary" onclick={() => { resolveSetupVersion(); view = "setup"; }}>🔧 Set Up New Engine</button>
          </div>
        </div>

      {:else if view === "create"}
        <div class="create-form">
          <h3>Create New Profile</h3>
          <div class="form-field">
            <label for="profile-name">Profile Name</label>
            <input id="profile-name" type="text" placeholder="e.g., US Firefox Business" bind:value={newName} />
          </div>
          <div class="form-field">
            <label for="engine-select">Browser Engine</label>
            <select id="engine-select" bind:value={newEngine} onchange={resolveCreateVersion}>
              {#each availableEngines.filter(e => e.available) as eng (eng.engine)}
                <option value={eng.engine}>{eng.icon} {eng.name}</option>
              {/each}
            </select>
          </div>
          <div class="form-field">
            <label for="version-input">Engine Version</label>
            <input id="version-input" type="text" bind:value={newVersion}
              placeholder={resolvingVersion ? "Resolving..." : "Version"}
              readonly={resolvingVersion} />
          </div>
          <div class="engine-info">
            {#each availableEngines.filter(e => e.engine === newEngine) as eng}
              <p>{eng.icon} <strong>{eng.name}</strong> — {eng.description}</p>
            {/each}
          </div>
          <button class="create-btn" onclick={createProfile} disabled={!newName.trim() || !newVersion || resolvingVersion}>
            {resolvingVersion ? "Resolving version..." : "Create Profile"}
          </button>
        </div>

      {:else}
        <div class="profiles-layout">
          <div class="profile-list">
            {#if profiles.length === 0}
              <div class="empty-state">
                <span class="empty-icon">🛡️</span>
                {#if status && status.installed_engines.length === 0}
                  <h3>Welcome to Shield Browser</h3>
                  <p>Set up a browser engine to create anti-detect profiles with unique fingerprints.</p>
                  <button class="action-btn primary" onclick={() => { resolveSetupVersion(); view = "setup"; }}>🔧 Set Up Engine</button>
                {:else}
                  <h3>No Profiles Yet</h3>
                  <p>Create your first anti-detect browser profile to get started.</p>
                  <button class="action-btn primary" onclick={() => { resolveCreateVersion(); view = "create"; }}>+ Create Profile</button>
                {/if}
              </div>
            {:else}
              {#each profiles as profile (profile.id)}
                <button
                  class="profile-card"
                  class:selected={selectedProfile?.id === profile.id}
                  class:running={profile.is_running}
                  onclick={() => selectedProfile = profile}
                >
                  <div class="profile-icon">{engineIcon(profile.engine)}</div>
                  <div class="profile-info">
                    <span class="profile-name">{profile.name}</span>
                    <span class="profile-engine">{engineLabel(profile.engine)} v{profile.engine_version}</span>
                  </div>
                  <div class="profile-status">
                    {#if profile.is_running}
                      <span class="status-dot running" title="Running"></span>
                    {:else}
                      <span class="status-dot idle" title="Idle"></span>
                    {/if}
                  </div>
                </button>
              {/each}
            {/if}
          </div>

          <!-- Detail Panel -->
          <div class="detail-panel">
            {#if selectedProfile}
              <div class="detail-header">
                <h3>{engineIcon(selectedProfile.engine)} {selectedProfile.name}</h3>
                {#if selectedProfile.is_running}
                  <span class="badge running">Running</span>
                {:else}
                  <span class="badge idle">Idle</span>
                {/if}
              </div>
              <div class="detail-grid">
                <div class="detail-row">
                  <span class="detail-key">Engine</span>
                  <span class="detail-val">{engineLabel(selectedProfile.engine)}</span>
                </div>
                <div class="detail-row">
                  <span class="detail-key">Version</span>
                  <span class="detail-val">{selectedProfile.engine_version}</span>
                </div>
                <div class="detail-row">
                  <span class="detail-key">Created</span>
                  <span class="detail-val">{formatDate(selectedProfile.created_at)}</span>
                </div>
                {#if selectedProfile.last_launch}
                  <div class="detail-row">
                    <span class="detail-key">Last Launch</span>
                    <span class="detail-val">{formatDate(selectedProfile.last_launch)}</span>
                  </div>
                {/if}
                <div class="detail-row">
                  <span class="detail-key">Proxy</span>
                  <span class="detail-val">{selectedProfile.has_proxy ? "✅ Configured" : "⚠️ None"}</span>
                </div>
                <div class="detail-row">
                  <span class="detail-key">Fingerprint OS</span>
                  <span class="detail-val">{selectedProfile.fingerprint_os ?? "Auto"}</span>
                </div>
              </div>
              {#if selectedProfile.tags.length > 0}
                <div class="detail-tags">
                  {#each selectedProfile.tags as tag}
                    <span class="tag">{tag}</span>
                  {/each}
                </div>
              {/if}
              <div class="detail-actions">
                {#if selectedProfile.is_running}
                  <button class="action-btn danger" onclick={() => stopProfile(selectedProfile!.id)}>⏹ Stop</button>
                {:else}
                  {#if isEngineInstalled(selectedProfile.engine, selectedProfile.engine_version)}
                    <button class="action-btn primary" onclick={() => launchProfile(selectedProfile!.id)} disabled={launching}>
                      {launching ? "Launching..." : "▶ Launch"}
                    </button>
                  {:else}
                    <button class="action-btn primary" onclick={() => downloadEngine(selectedProfile!.engine, selectedProfile!.engine_version)} disabled={downloading}>
                      {downloading ? downloadProgress : "⬇ Download Engine"}
                    </button>
                  {/if}
                  <button class="action-btn danger" onclick={() => deleteProfile(selectedProfile!.id)}>Delete</button>
                {/if}
              </div>
            {:else}
              <div class="no-selection">
                <span>🛡️</span>
                <p>Select a profile to view details and manage fingerprint, proxy, and engine settings.</p>
              </div>
            {/if}
          </div>
        </div>
      {/if}
    </div>
  {/if}
</section>

<style>
  .shield-app { height: 100%; overflow: hidden; display: grid; grid-template-rows: 1fr; }

  /* ─── Loading ─── */
  .loading-screen { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 1rem; color: var(--system-color-text-muted); }

  .spinner { width: 24px; height: 24px; border: 3px solid var(--system-color-border); border-top-color: var(--system-color-primary); border-radius: 50%; animation: spin 0.8s linear infinite; }
  .spinner.large { width: 40px; height: 40px; border-width: 4px; }
  @keyframes spin { to { transform: rotate(360deg); } }

  /* ─── Setup/Onboarding ─── */
  .setup-screen { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 1.5rem; padding: 2rem; max-width: 560px; margin: 0 auto; }
  .setup-hero { text-align: center; }
  .setup-icon { font-size: 3.5rem; display: block; margin-bottom: 0.5rem; }
  .setup-hero h2 { margin: 0; font-size: 1.5rem; }
  .setup-subtitle { margin: 0.5rem 0 0; color: var(--system-color-text-muted); font-size: 0.88rem; line-height: 1.5; }

  .engine-cards { display: flex; flex-direction: column; gap: 0.6rem; width: 100%; }
  .engine-card {
    display: flex; align-items: flex-start; gap: 0.8rem; padding: 0.9rem 1rem;
    border-radius: 0.85rem; border: 2px solid var(--system-color-border);
    background: var(--system-color-panel); color: var(--system-color-text);
    text-align: left; cursor: pointer; transition: all 0.15s; position: relative;
  }
  .engine-card:hover:not(:disabled) { border-color: hsla(var(--system-color-primary-hsl) / 0.4); }
  .engine-card.selected { border-color: var(--system-color-primary); background: hsla(var(--system-color-primary-hsl) / 0.06); }
  .engine-card.disabled { opacity: 0.5; cursor: not-allowed; }
  .engine-card-icon { font-size: 1.8rem; flex-shrink: 0; margin-top: 0.1rem; }
  .engine-card-body h4 { margin: 0 0 0.25rem; font-size: 0.92rem; }
  .engine-card-body p { margin: 0; font-size: 0.78rem; color: var(--system-color-text-muted); line-height: 1.45; }
  .coming-soon { font-size: 0.65rem; font-weight: 500; padding: 0.1rem 0.45rem; border-radius: 999px; background: hsla(var(--system-color-dark-hsl) / 0.1); color: var(--system-color-text-muted); vertical-align: middle; margin-left: 0.3rem; }
  .checkmark { position: absolute; top: 0.7rem; right: 0.8rem; color: var(--system-color-primary); font-size: 1.1rem; font-weight: 700; }

  .setup-meta { font-size: 0.82rem; color: var(--system-color-text-muted); }
  .setup-meta p { margin: 0; }
  .setup-error { padding: 0.6rem 0.9rem; border-radius: 0.6rem; background: hsl(0 85% 95%); color: hsl(0 75% 40%); font-size: 0.82rem; width: 100%; }

  .install-btn {
    width: 100%; padding: 0.75rem; border-radius: 0.75rem; border: none;
    background: var(--system-color-primary); color: white; font-size: 0.92rem; font-weight: 600;
    cursor: pointer; transition: filter 0.12s;
  }
  .install-btn:hover:not(:disabled) { filter: brightness(1.1); }
  .install-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .skip-btn { background: none; border: none; color: var(--system-color-text-muted); font-size: 0.8rem; cursor: pointer; padding: 0.3rem; }
  .skip-btn:hover { color: var(--system-color-text); }

  .install-progress { display: flex; flex-direction: column; align-items: center; gap: 1rem; padding: 2rem; }
  .progress-text {
    font-size: 0.88rem; color: var(--system-color-text-muted);
    font-variant-numeric: tabular-nums;
    min-width: 280px; text-align: center;
    white-space: nowrap;
  }
  .progress-bar-track {
    width: 100%; max-width: 320px; height: 6px; border-radius: 999px;
    background: hsla(var(--system-color-dark-hsl) / 0.15); overflow: hidden;
  }
  .progress-bar-fill {
    height: 100%; border-radius: 999px;
    background: var(--system-color-primary);
    transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  }
  .progress-percent {
    font-size: 1.4rem; font-weight: 700; color: var(--system-color-text); margin: 0;
    font-variant-numeric: tabular-nums;
    min-width: 4ch; text-align: center;
  }

  /* ─── Header + Status (profiles view) ─── */
  .shield-app:has(.header) { grid-template-rows: auto auto 1fr; gap: 0.85rem; padding: 1.1rem; }
  .header { display: flex; justify-content: space-between; align-items: center; }
  .eyebrow { margin: 0; font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.14em; color: var(--system-color-text-muted); }
  h2 { margin: 0.3rem 0 0; }
  .header-actions { display: flex; gap: 0.5rem; }
  .action-btn {
    border-radius: 999px; padding: 0.5rem 0.9rem; font-size: 0.82rem; cursor: pointer;
    border: 1px solid var(--system-color-border); background: var(--system-color-panel); color: var(--system-color-text);
    transition: background 0.15s;
  }
  .action-btn:hover { background: hsla(var(--system-color-dark-hsl) / 0.08); }
  .action-btn.primary { background: var(--system-color-primary); color: white; border-color: transparent; }
  .action-btn.primary:hover { filter: brightness(1.1); }
  .action-btn.danger { background: hsl(0 75% 55%); color: white; border-color: transparent; }
  .action-btn.danger:hover { filter: brightness(1.1); }

  .status-bar {
    display: flex; gap: 1rem; padding: 0.7rem 1rem; border-radius: 0.8rem;
    border: 1px solid var(--system-color-border); background: var(--system-color-panel);
  }
  .status-item { display: flex; align-items: baseline; gap: 0.35rem; }
  .status-value { font-size: 1.1rem; font-weight: 700; color: var(--system-color-text); }
  .status-value.running { color: hsl(142 70% 45%); }
  .status-label { font-size: 0.72rem; color: var(--system-color-text-muted); text-transform: uppercase; letter-spacing: 0.1em; }

  .main-content { overflow: hidden; min-height: 0; }

  /* ─── Profile List ─── */
  .profiles-layout { display: grid; grid-template-columns: 1fr 1.4fr; gap: 0.85rem; height: 100%; overflow: hidden; }
  .profile-list { overflow-y: auto; display: flex; flex-direction: column; gap: 0.35rem; padding-right: 0.3rem; }
  .profile-card {
    display: flex; align-items: center; gap: 0.7rem; padding: 0.65rem 0.75rem;
    border-radius: 0.7rem; border: 1px solid transparent; background: transparent; color: var(--system-color-text);
    text-align: left; cursor: pointer; transition: all 0.12s;
  }
  .profile-card:hover { background: hsla(var(--system-color-dark-hsl) / 0.06); }
  .profile-card.selected { background: hsla(var(--system-color-primary-hsl) / 0.1); border-color: hsla(var(--system-color-primary-hsl) / 0.25); }
  .profile-card.running { border-left: 3px solid hsl(142 70% 45%); }
  .profile-icon { font-size: 1.3rem; }
  .profile-info { display: flex; flex-direction: column; flex: 1; min-width: 0; }
  .profile-name { font-size: 0.85rem; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .profile-engine { font-size: 0.7rem; color: var(--system-color-text-muted); }
  .status-dot { width: 8px; height: 8px; border-radius: 50%; }
  .status-dot.running { background: hsl(142 70% 45%); box-shadow: 0 0 6px hsl(142 70% 45% / 0.5); }
  .status-dot.idle { background: var(--system-color-text-muted); opacity: 0.4; }

  /* ─── Detail Panel ─── */
  .detail-panel {
    overflow-y: auto; border-radius: 1rem; padding: 1rem;
    border: 1px solid var(--system-color-border); background: var(--system-color-panel);
  }
  .detail-header { display: flex; justify-content: space-between; align-items: center; }
  .detail-header h3 { margin: 0; font-size: 1rem; }
  .badge { padding: 0.15rem 0.55rem; border-radius: 999px; font-size: 0.7rem; font-weight: 600; }
  .badge.running { background: hsla(142 70% 45% / 0.15); color: hsl(142 70% 35%); }
  .badge.idle { background: hsla(var(--system-color-dark-hsl) / 0.08); color: var(--system-color-text-muted); }
  .detail-grid { margin-top: 1rem; display: flex; flex-direction: column; gap: 0.5rem; }
  .detail-row { display: flex; justify-content: space-between; align-items: center; padding: 0.3rem 0; border-bottom: 1px solid hsla(var(--system-color-dark-hsl) / 0.06); }
  .detail-key { font-size: 0.78rem; color: var(--system-color-text-muted); }
  .detail-val { font-size: 0.82rem; font-weight: 500; }
  .detail-tags { display: flex; flex-wrap: wrap; gap: 0.3rem; margin-top: 0.7rem; }
  .tag { padding: 0.15rem 0.5rem; font-size: 0.7rem; border-radius: 999px; background: hsla(var(--system-color-primary-hsl) / 0.1); color: var(--system-color-primary); }
  .detail-actions { margin-top: 1rem; display: flex; gap: 0.5rem; justify-content: flex-end; }

  .no-selection { display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100%; gap: 0.5rem; color: var(--system-color-text-muted); }
  .no-selection span { font-size: 2.5rem; }
  .no-selection p { font-size: 0.85rem; text-align: center; max-width: 20rem; }

  /* ─── Create Form ─── */
  .create-form { max-width: 480px; margin: 0 auto; }
  .create-form h3 { margin: 0 0 1rem; }
  .form-field { display: flex; flex-direction: column; gap: 0.3rem; margin-bottom: 0.85rem; }
  .form-field label { font-size: 0.78rem; font-weight: 600; color: var(--system-color-text-muted); text-transform: uppercase; letter-spacing: 0.08em; }
  .form-field input, .form-field select {
    padding: 0.55rem 0.8rem; border-radius: 0.6rem; border: 1px solid var(--system-color-border);
    background: var(--system-color-panel); color: var(--system-color-text); font-size: 0.85rem;
  }
  .form-field input:focus, .form-field select:focus { border-color: var(--system-color-primary); outline: none; }
  .engine-info {
    padding: 0.75rem; border-radius: 0.7rem; margin-bottom: 1rem;
    background: hsla(var(--system-color-primary-hsl) / 0.06); font-size: 0.82rem; line-height: 1.5;
  }
  .engine-info p { margin: 0; }
  .create-btn {
    width: 100%; padding: 0.65rem; border-radius: 0.7rem; border: none;
    background: var(--system-color-primary); color: white; font-size: 0.88rem; font-weight: 600; cursor: pointer;
  }
  .create-btn:hover:not(:disabled) { filter: brightness(1.1); }
  .create-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .empty-state { text-align: center; display: flex; flex-direction: column; align-items: center; gap: 0.5rem; padding: 3rem; color: var(--system-color-text-muted); }
  .empty-icon { font-size: 2.5rem; }
  .empty-state h3 { margin: 0; color: var(--system-color-text); font-size: 1rem; }
  .empty-state p { margin: 0; font-size: 0.82rem; max-width: 18rem; }

  /* ─── Settings Panel ─── */
  .settings-panel { max-width: 600px; margin: 0 auto; overflow-y: auto; }
  .settings-panel h3 { margin: 0 0 0.25rem; font-size: 1.1rem; }
  .settings-subtitle { margin: 0 0 1.2rem; font-size: 0.82rem; color: var(--system-color-text-muted); }
  .engine-list { display: flex; flex-direction: column; gap: 0.6rem; }
  .engine-row {
    display: flex; align-items: center; gap: 0.8rem; padding: 0.85rem 1rem;
    border-radius: 0.85rem; border: 1px solid var(--system-color-border);
    background: var(--system-color-panel);
  }
  .engine-row-icon { font-size: 1.6rem; flex-shrink: 0; }
  .engine-row-info { flex: 1; display: flex; flex-direction: column; gap: 0.15rem; min-width: 0; }
  .engine-row-name { font-size: 0.88rem; font-weight: 600; }
  .engine-row-version { font-size: 0.75rem; color: var(--system-color-text-muted); }
  .update-badge {
    font-size: 0.7rem; font-weight: 600; padding: 0.1rem 0.45rem;
    border-radius: 999px; background: hsla(38 90% 50% / 0.15); color: hsl(38 85% 40%);
    width: fit-content;
  }
  .up-to-date-badge {
    font-size: 0.7rem; font-weight: 600; padding: 0.1rem 0.45rem;
    border-radius: 999px; background: hsla(142 70% 45% / 0.12); color: hsl(142 70% 35%);
    width: fit-content;
  }
  .engine-row-actions { display: flex; gap: 0.35rem; flex-shrink: 0; }
  .settings-progress { margin-top: 1rem; display: flex; flex-direction: column; gap: 0.5rem; align-items: center; }
  .settings-empty { text-align: center; padding: 2rem; color: var(--system-color-text-muted); }
  .settings-empty p { margin: 0 0 0.8rem; font-size: 0.85rem; }
  .settings-section { margin-top: 1.5rem; padding-top: 1rem; border-top: 1px solid var(--system-color-border); }
  .settings-section h4 { margin: 0 0 0.6rem; font-size: 0.88rem; }
</style>
