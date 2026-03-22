<script lang="ts">
  import type { AppManifest, InstalledApp } from "$lib/api/types";
  import { installApp, launchApp, stopApp, uninstallApp } from "$lib/stores/state";
  import * as api from "$lib/api/backend";

  const ICONS: Record<string, string> = {
    "sample-gradio": "🔢", "stable-diffusion-webui": "🎨", "ollama": "🦙",
    "comfyui": "🧩", "open-webui": "💬", "whisper-web": "🎙️",
    "fooocus": "🔮", "kohya-ss": "⚡",
  };

  interface Props {
    app: AppManifest;
    installed: InstalledApp | null;
  }

  let { app, installed }: Props = $props();

  let loading = $state(false);
  let error = $state("");

  const icon = $derived(app.icon || ICONS[app.id] || "📦");
  const status = $derived(installed?.status.state ?? "NotInstalled");
  const isRunning = $derived(status === "Running");
  const isInstalled = $derived(status === "Installed" || isRunning);

  async function handleInstall() {
    loading = true; error = "";
    try { await installApp(app); }
    catch (e) { error = String(e); }
    finally { loading = false; }
  }

  async function handleLaunch() {
    loading = true; error = "";
    try { await launchApp(app.id); }
    catch (e) { error = String(e); }
    finally { loading = false; }
  }

  async function handleStop() {
    loading = true; error = "";
    try { await stopApp(app.id); }
    catch (e) { error = String(e); }
    finally { loading = false; }
  }

  async function handleUninstall() {
    if (!confirm(`Uninstall "${app.name}"? This removes all data.`)) return;
    loading = true; error = "";
    try { await uninstallApp(app.id); }
    catch (e) { error = String(e); }
    finally { loading = false; }
  }

  async function handleOpen() {
    if (installed?.status.port) {
      await api.openAppBrowser(installed.status.port);
    }
  }
</script>

<div class="card" class:running={isRunning}>
  <div class="card-header">
    <span class="icon">{icon}</span>
    <div class="info">
      <div class="name">{app.name}</div>
      <div class="desc">{app.description}</div>
    </div>
    <div class="status-badge" class:status-running={isRunning} class:status-installed={isInstalled && !isRunning} class:status-none={!isInstalled}>
      {#if isRunning}
        <span class="dot dot-running"></span> Running
      {:else if isInstalled}
        <span class="dot dot-installed"></span> Installed
      {:else}
        Available
      {/if}
    </div>
  </div>

  <div class="meta">
    {#each app.tags as tag}
      <span class="tag">{tag}</span>
    {/each}
    {#if app.needs_gpu}
      <span class="tag tag-gpu">GPU</span>
    {/if}
    <span class="tag tag-size">{app.disk_size}</span>
  </div>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  <div class="actions">
    {#if loading}
      <button class="btn btn-disabled" disabled>
        <span class="animate-spin">⏳</span> Working...
      </button>
    {:else if isRunning}
      <button class="btn btn-open" onclick={handleOpen}>🌐 Open</button>
      <button class="btn btn-stop" onclick={handleStop}>⏹ Stop</button>
    {:else if isInstalled}
      <button class="btn btn-launch" onclick={handleLaunch}>▶ Launch</button>
      <button class="btn btn-danger" onclick={handleUninstall}>🗑 Uninstall</button>
    {:else}
      <button class="btn btn-install" onclick={handleInstall}>⬇ Install</button>
    {/if}
  </div>
</div>

<style>
  .card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    padding: 16px;
    transition: border-color 0.2s, box-shadow 0.2s;
  }
  .card:hover { border-color: var(--color-border-hover); }
  .card.running { border-color: var(--color-success); box-shadow: 0 0 0 1px rgba(34,197,94,0.15); }

  .card-header { display: flex; align-items: flex-start; gap: 12px; }
  .icon { font-size: 28px; line-height: 1; flex-shrink: 0; margin-top: 2px; }
  .info { flex: 1; min-width: 0; }
  .name { font-weight: 600; font-size: 14px; color: var(--color-text); }
  .desc { font-size: 12px; color: var(--color-text-dim); margin-top: 2px; line-height: 1.4; }

  .status-badge {
    font-size: 11px; font-weight: 500; padding: 3px 8px;
    border-radius: 12px; white-space: nowrap; display: flex; align-items: center; gap: 4px;
  }
  .status-running { background: rgba(34,197,94,0.12); color: var(--color-success); }
  .status-installed { background: rgba(59,130,246,0.12); color: var(--color-info); }
  .status-none { background: rgba(161,161,170,0.1); color: var(--color-text-muted); }

  .dot { width: 6px; height: 6px; border-radius: 50%; display: inline-block; }
  .dot-running { background: var(--color-success); box-shadow: 0 0 6px var(--color-success); }
  .dot-installed { background: var(--color-info); }

  .meta { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 10px; }
  .tag {
    font-size: 10px; font-weight: 500; padding: 2px 7px; border-radius: 4px;
    background: rgba(139,92,246,0.1); color: var(--color-accent);
    font-family: var(--font-mono);
  }
  .tag-gpu { background: rgba(245,158,11,0.12); color: var(--color-warning); }
  .tag-size { background: rgba(161,161,170,0.08); color: var(--color-text-muted); }

  .error {
    margin-top: 8px; font-size: 11px; color: var(--color-danger);
    background: rgba(239,68,68,0.08); padding: 6px 10px; border-radius: 4px;
    font-family: var(--font-mono);
  }

  .actions { display: flex; gap: 6px; margin-top: 12px; }
  .btn {
    font-size: 12px; font-weight: 500; padding: 6px 14px; border-radius: 6px;
    border: none; cursor: pointer; display: flex; align-items: center; gap: 4px;
    transition: background 0.15s, transform 0.1s;
  }
  .btn:active { transform: scale(0.97); }
  .btn-install { background: var(--color-accent); color: white; }
  .btn-install:hover { background: var(--color-accent-hover); }
  .btn-launch { background: var(--color-success); color: white; }
  .btn-launch:hover { background: #16a34a; }
  .btn-open { background: var(--color-info); color: white; }
  .btn-open:hover { background: #2563eb; }
  .btn-stop { background: var(--color-warning); color: #000; }
  .btn-stop:hover { background: #d97706; }
  .btn-danger { background: transparent; color: var(--color-text-muted); border: 1px solid var(--color-border); }
  .btn-danger:hover { background: rgba(239,68,68,0.1); color: var(--color-danger); border-color: var(--color-danger); }
  .btn-disabled { background: var(--color-surface-hover); color: var(--color-text-muted); cursor: not-allowed; }
</style>
