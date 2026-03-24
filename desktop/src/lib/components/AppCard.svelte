<script lang="ts">
  import type { AppManifest, InstalledApp } from "$lib/api/types";
  import { installApp, launchApp, stopApp, uninstallApp } from "$lib/stores/state";
  import { openGenericBrowserWindow } from "../../state/desktop.svelte";

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

  import { createMutation } from "@tanstack/svelte-query";

  const installMutation = createMutation(() => ({ mutationFn: () => installApp(app) }));
  const launchMutation = createMutation(() => ({ mutationFn: () => launchApp(app.id) }));
  const stopMutation = createMutation(() => ({ mutationFn: () => stopApp(app.id) }));
  const uninstallMutation = createMutation(() => ({ mutationFn: () => uninstallApp(app.id) }));

  const loading = $derived(
    installMutation.isPending || 
    launchMutation.isPending || 
    stopMutation.isPending || 
    uninstallMutation.isPending
  );

  const errorMsg = $derived.by(() => {
    const err = installMutation.error || launchMutation.error || stopMutation.error || uninstallMutation.error;
    return err ? String(err) : "";
  });

  const icon = $derived(app.icon || ICONS[app.id] || "📦");
  const status = $derived(installed?.status.state ?? "NotInstalled");
  const isRunning = $derived(status === "Running");
  const isInstalled = $derived(status === "Installed" || isRunning);

  function handleInstall() {
    installMutation.mutate();
  }

  function handleLaunch() {
    launchMutation.mutate();
  }

  function handleStop() {
    stopMutation.mutate();
  }

  function handleUninstall() {
    if (!confirm(`Uninstall "${app.name}"? This removes all data.`)) return;
    uninstallMutation.mutate();
  }

  async function handleOpen() {
    if (installed?.status.port) {
      openGenericBrowserWindow(`http://localhost:${installed.status.port}`, app.name);
    }
  }
</script>

<div class="bg-white/50 dark:bg-black/50 border border-black/10 dark:border-white/10 rounded-2xl p-4 transition-all duration-200 {isRunning ? 'border-green-500 shadow-[0_0_0_1px_rgba(34,197,94,0.15)]' : 'hover:border-black/20 dark:hover:border-white/20'}">
  <div class="flex items-start gap-3">
    <span class="text-[28px] leading-none shrink-0 mt-0.5">{icon}</span>
    <div class="flex-1 min-w-0">
      <div class="font-semibold text-[14px] text-black dark:text-white">{app.name}</div>
      <div class="text-[12px] text-gray-500 dark:text-gray-400 mt-0.5 leading-snug">{app.description}</div>
    </div>
    
    <div class="text-[11px] font-medium px-2 py-[3px] rounded-xl whitespace-nowrap flex items-center gap-1 {isRunning ? 'bg-green-500/15 text-green-600 dark:text-green-400' : isInstalled ? 'bg-blue-500/15 text-blue-600 dark:text-blue-400' : 'bg-black/5 dark:bg-white/5 text-gray-500 dark:text-gray-400'}">
      {#if isRunning}
        <span class="w-1.5 h-1.5 rounded-full inline-block bg-green-500 shadow-[0_0_6px_rgba(34,197,94,0.8)]"></span> Running
      {:else if isInstalled}
        <span class="w-1.5 h-1.5 rounded-full inline-block bg-blue-500"></span> Installed
      {:else}
        Available
      {/if}
    </div>
  </div>

  <div class="flex flex-wrap gap-1.5 mt-2.5">
    {#each app.tags as tag}
      <span class="text-[10px] font-medium px-1.5 py-0.5 rounded bg-purple-500/10 text-purple-600 dark:text-purple-400 font-mono">{tag}</span>
    {/each}
    {#if app.needs_gpu}
      <span class="text-[10px] font-medium px-1.5 py-0.5 rounded bg-amber-500/10 text-amber-600 dark:text-amber-400 font-mono">GPU</span>
    {/if}
    <span class="text-[10px] font-medium px-1.5 py-0.5 rounded bg-black/5 dark:bg-white/5 text-gray-500 font-mono">{app.disk_size}</span>
  </div>

  {#if errorMsg}
    <div class="mt-2 text-[11px] text-red-500 bg-red-500/10 px-2.5 py-1.5 rounded font-mono">{errorMsg}</div>
  {/if}

  <div class="flex gap-1.5 mt-3">
    {#if loading}
      <button class="text-[12px] font-medium px-3.5 py-1.5 rounded bg-black/5 dark:bg-white/5 text-gray-500 cursor-not-allowed flex items-center gap-1" disabled>
        <span class="animate-spin">⏳</span> Working...
      </button>
    {:else if isRunning}
      <button class="text-[12px] font-medium px-3.5 py-1.5 rounded bg-blue-500 text-white flex items-center gap-1 hover:bg-blue-600 transition-colors active:scale-95" onclick={handleOpen}>🌐 Open</button>
      <button class="text-[12px] font-medium px-3.5 py-1.5 rounded bg-amber-500 text-black flex items-center gap-1 hover:bg-amber-600 transition-colors active:scale-95" onclick={handleStop}>⏹ Stop</button>
    {:else if isInstalled}
      <button class="text-[12px] font-medium px-3.5 py-1.5 rounded bg-green-500 text-white flex items-center gap-1 hover:bg-green-600 transition-colors active:scale-95" onclick={handleLaunch}>▶ Launch</button>
      <button class="text-[12px] font-medium px-3.5 py-1.5 rounded bg-transparent border border-black/10 dark:border-white/10 text-gray-500 flex items-center gap-1 hover:bg-red-500/10 hover:text-red-500 hover:border-red-500 transition-colors active:scale-95" onclick={handleUninstall}>🗑 Uninstall</button>
    {:else}
      <button class="text-[12px] font-medium px-3.5 py-1.5 rounded bg-purple-500 text-white flex items-center gap-1 hover:bg-purple-600 transition-colors active:scale-95" onclick={handleInstall}>⬇ Install</button>
    {/if}
  </div>
</div>
