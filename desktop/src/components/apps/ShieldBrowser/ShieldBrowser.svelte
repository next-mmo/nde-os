<svelte:options runes={true} />

<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

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

  let profiles = $state<ShieldProfile[]>([]);
  let status = $state<ShieldStatus | null>(null);
  let loading = $state(true);
  let view = $state<"profiles" | "create">("profiles");
  let selectedProfile = $state<ShieldProfile | null>(null);

  // Create form
  let newName = $state("");
  let newEngine = $state("wayfern");
  let newVersion = $state("latest");

  let downloading = $state(false);
  let launching = $state(false);

  $effect(() => { refresh(); });

  async function refresh() {
    loading = true;
    try {
      profiles = await invoke<ShieldProfile[]>("list_shield_profiles");
      status = await invoke<ShieldStatus>("get_shield_status");
      // Refresh selected profile if it exists
      if (selectedProfile) {
        const updated = profiles.find(p => p.id === selectedProfile!.id);
        if (updated) selectedProfile = updated;
      }
    } catch (e) {
      console.error("Failed to load shield data:", e);
      profiles = [];
      status = { total_profiles: 0, running_profiles: 0, installed_engines: [] };
    } finally {
      loading = false;
    }
  }

  async function createProfile() {
    if (!newName.trim()) return;
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
      alert(`Failed to delete profile: ${e}`);
    }
  }

  async function downloadEngine(engine: string, version: string) {
    downloading = true;
    try {
      await invoke("download_shield_engine", { engine, version });
      await refresh();
    } catch (e: unknown) {
      alert(`Failed to download engine: ${e}`);
    } finally {
      downloading = false;
    }
  }

  async function launchProfile(id: string) {
    launching = true;
    try {
      const cdpPort = await invoke<number>("launch_shield_profile", { id, url: "https://browserleaks.com/canvas" });
      console.log(`Profile launched on CDP port: ${cdpPort}`);
      await refresh();
    } catch (e: unknown) {
      alert(`Failed to launch profile: ${e}`);
    } finally {
      launching = false;
    }
  }

  async function stopProfile(id: string) {
    try {
      await invoke("stop_shield_profile", { id });
      await refresh();
    } catch (e: unknown) {
      alert(`Failed to stop profile: ${e}`);
    }
  }

  function isEngineInstalled(engine: string, version: string): boolean {
    if (!status) return false;
    return status.installed_engines.some(e => e.engine === engine && e.version === version);
  }

  function engineIcon(engine: string) {
    return engine === "wayfern" ? "🌐" : "🦊";
  }

  function engineLabel(engine: string) {
    return engine === "wayfern" ? "Wayfern (Chromium)" : "Camoufox (Firefox)";
  }

  function formatDate(epoch: number) {
    return new Date(epoch * 1000).toLocaleDateString();
  }
</script>

<section class="shield-app">
  <!-- Header -->
  <div class="header">
    <div>
      <p class="eyebrow">Anti-Detect Browser</p>
      <h2>🛡️ Shield Browser</h2>
    </div>
    <div class="header-actions">
      {#if view === "create"}
        <button class="action-btn" onclick={() => view = "profiles"}>← Back</button>
      {:else}
        <button class="action-btn primary" onclick={() => view = "create"}>+ New Profile</button>
        <button class="action-btn" onclick={refresh} disabled={loading}>{loading ? "..." : "↻"}</button>
      {/if}
    </div>
  </div>

  <!-- Status Bar -->
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

  <!-- Main Content -->
  <div class="main-content">
    {#if view === "create"}
      <!-- Create Profile Form -->
      <div class="create-form">
        <h3>Create New Profile</h3>
        <div class="form-field">
          <label for="profile-name">Profile Name</label>
          <input id="profile-name" type="text" placeholder="e.g., US Chrome Business" bind:value={newName} />
        </div>
        <div class="form-field">
          <label for="engine-select">Browser Engine</label>
          <select id="engine-select" bind:value={newEngine}>
            <option value="wayfern">🌐 Wayfern — Chromium (C++ fingerprint patches)</option>
            <option value="camoufox">🦊 Camoufox — Firefox (C++ fingerprint patches)</option>
          </select>
        </div>
        <div class="form-field">
          <label for="version-input">Engine Version</label>
          <input id="version-input" type="text" placeholder="latest" bind:value={newVersion} />
        </div>
        <div class="engine-info">
          {#if newEngine === "wayfern"}
            <p>🌐 <strong>Wayfern</strong> — Chromium-based with patched canvas, WebGL, audio, and navigator APIs at the C++ level. Undetectable by most anti-bot systems.</p>
          {:else}
            <p>🦊 <strong>Camoufox</strong> — Firefox-based with native fingerprint generation via Playwright. Includes timezone, locale, WebRTC, and geolocation spoofing.</p>
          {/if}
        </div>
        <button class="create-btn" onclick={createProfile} disabled={!newName.trim()}>Create Profile</button>
      </div>
    {:else}
      <!-- Profile List + Detail -->
      <div class="profiles-layout">
        <div class="profile-list">
          {#if loading}
            <div class="loading-state">Loading profiles...</div>
          {:else if profiles.length === 0}
            <div class="empty-state">
              <span class="empty-icon">🛡️</span>
              <h3>No Profiles Yet</h3>
              <p>Create your first anti-detect browser profile to get started.</p>
              <button class="action-btn primary" onclick={() => view = "create"}>+ Create Profile</button>
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
                    {downloading ? "Downloading..." : "⬇ Download Engine"}
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
</section>

<style>
  .shield-app { height: 100%; overflow: hidden; padding: 1.1rem; display: grid; grid-template-rows: auto auto 1fr; gap: 0.85rem; }
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

  /* Status Bar */
  .status-bar {
    display: flex; gap: 1rem; padding: 0.7rem 1rem; border-radius: 0.8rem;
    border: 1px solid var(--system-color-border); background: var(--system-color-panel);
  }
  .status-item { display: flex; align-items: baseline; gap: 0.35rem; }
  .status-value { font-size: 1.1rem; font-weight: 700; color: var(--system-color-text); }
  .status-value.running { color: hsl(142 70% 45%); }
  .status-label { font-size: 0.72rem; color: var(--system-color-text-muted); text-transform: uppercase; letter-spacing: 0.1em; }

  /* Main Content */
  .main-content { overflow: hidden; min-height: 0; }

  /* Profile List */
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

  /* Detail Panel */
  .detail-panel {
    overflow-y: auto; border-radius: 1rem; padding: 1rem;
    border: 1px solid var(--system-color-border); background: var(--system-color-panel);
  }
  .detail-header { display: flex; justify-content: space-between; align-items: center; }
  .detail-header h3 { margin: 0; font-size: 1rem; }
  .badge {
    padding: 0.15rem 0.55rem; border-radius: 999px; font-size: 0.7rem; font-weight: 600;
  }
  .badge.running { background: hsla(142 70% 45% / 0.15); color: hsl(142 70% 35%); }
  .badge.idle { background: hsla(var(--system-color-dark-hsl) / 0.08); color: var(--system-color-text-muted); }
  .detail-grid { margin-top: 1rem; display: flex; flex-direction: column; gap: 0.5rem; }
  .detail-row { display: flex; justify-content: space-between; align-items: center; padding: 0.3rem 0; border-bottom: 1px solid hsla(var(--system-color-dark-hsl) / 0.06); }
  .detail-key { font-size: 0.78rem; color: var(--system-color-text-muted); }
  .detail-val { font-size: 0.82rem; font-weight: 500; }
  .detail-tags { display: flex; flex-wrap: wrap; gap: 0.3rem; margin-top: 0.7rem; }
  .tag { padding: 0.15rem 0.5rem; font-size: 0.7rem; border-radius: 999px; background: hsla(var(--system-color-primary-hsl) / 0.1); color: var(--system-color-primary); }
  .detail-actions { margin-top: 1rem; display: flex; gap: 0.5rem; justify-content: flex-end; }

  /* No Selection */
  .no-selection { display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100%; gap: 0.5rem; color: var(--system-color-text-muted); }
  .no-selection span { font-size: 2.5rem; }
  .no-selection p { font-size: 0.85rem; text-align: center; max-width: 20rem; }

  /* Create Form */
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

  /* States */
  .loading-state, .empty-state { text-align: center; padding: 2rem; color: var(--system-color-text-muted); }
  .empty-state { display: flex; flex-direction: column; align-items: center; gap: 0.5rem; padding: 3rem; }
  .empty-icon { font-size: 2.5rem; }
  .empty-state h3 { margin: 0; color: var(--system-color-text); font-size: 1rem; }
  .empty-state p { margin: 0; font-size: 0.82rem; max-width: 18rem; }
</style>
