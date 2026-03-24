<svelte:options runes={true} />

<script lang="ts">
  import * as api from "$lib/api/backend";
  import type { ProviderStatus, PluginStatus, ChannelStatus } from "$lib/api/types";
  import { healthStatus, runningCount, catalogCount, resourceUsage, systemInfo } from "$lib/stores/state";
  import { selectLauncherSection } from "🍎/state/desktop.svelte";

  let providers = $state<ProviderStatus[]>([]);
  let plugins = $state<PluginStatus[]>([]);
  let channels = $state<ChannelStatus[]>([]);
  let activeModel = $state("");
  let agentTools = $state<string[]>([]);
  let loading = $state(true);

  $effect(() => { refresh(); });

  async function refresh() {
    loading = true;
    try {
      const [provs, active, plugs, config] = await Promise.all([
        api.listModels().catch(() => []),
        api.activeModel().catch(() => "unknown"),
        api.listPlugins().catch(() => []),
        api.agentConfig().catch(() => ({ tools: [], name: "", provider: "", model: "", max_iterations: 0, workspace: "" })),
      ]);
      providers = provs as ProviderStatus[];
      activeModel = active as string;
      plugins = plugs as PluginStatus[];
      agentTools = config.tools;
      channels = await api.listChannels().catch(() => [
        { name: "rest-api", channel_type: "rest" as const, is_running: true, messages_received: 0, messages_sent: 0 },
      ]);
    } catch {}
    finally { loading = false; }
  }

  const runningPlugins = $derived(plugins.filter(p => p.state === "running").length);
  const activeChannels = $derived(channels.filter(c => c.is_running).length);

  function usageTone(pct: number) { return pct >= 85 ? "danger" : pct >= 70 ? "warning" : "safe"; }
  function formatBytes(b: number) {
    if (b >= 1073741824) return `${(b / 1073741824).toFixed(1)} GB`;
    if (b >= 1048576) return `${(b / 1048576).toFixed(1)} MB`;
    return `${Math.round(b / 1024)} KB`;
  }
</script>

<section class="h-full overflow-auto p-[1.1rem] grid gap-4 content-start">
  <div class="flex justify-between items-center">
    <div>
      <p class="m-0 text-[0.72rem] uppercase tracking-[0.14em] text-gray-500">Overview</p>
      <h2 class="mt-[0.3rem] mb-0 text-xl font-bold">Command Center</h2>
    </div>
    <button class="rounded-full px-[0.9rem] py-[0.5rem] text-[0.82rem] cursor-pointer border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 text-black dark:text-white" onclick={refresh} disabled={loading}>{loading ? "..." : "↻ Refresh"}</button>
  </div>

  <!-- Status Cards Row -->
  <div class="grid grid-cols-[repeat(auto-fit,minmax(10rem,1fr))] gap-[0.6rem]">
    <div class="flex items-center gap-[0.55rem] px-[0.85rem] py-[0.75rem] rounded-2xl border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 text-left" class:online={$healthStatus === "online"}>
      <span class="w-[0.55rem] h-[0.55rem] rounded-full shrink-0 transition-colors duration-300 {$healthStatus === 'online' ? 'bg-green-500 shadow-[0_0_6px_rgba(34,197,94,0.5)]' : 'bg-red-500'}"></span>
      <div class="flex flex-col">
        <strong class="text-[0.82rem]">{$healthStatus === "online" ? "Server Online" : "Offline"}</strong>
        <span class="text-[0.7rem] text-gray-500">localhost:8080</span>
      </div>
    </div>
    <button class="flex items-center gap-[0.55rem] px-[0.85rem] py-[0.75rem] rounded-2xl border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 text-left cursor-pointer transition-all duration-150 hover:border-blue-500/30 hover:-translate-y-px" onclick={() => selectLauncherSection("model-settings")}>
      <span class="text-[1.3rem]">🤖</span>
      <div class="flex flex-col">
        <strong class="text-[0.82rem]">{activeModel || "No Model"}</strong>
        <span class="text-[0.7rem] text-gray-500">{providers.length} provider{providers.length !== 1 ? "s" : ""}</span>
      </div>
    </button>
    <button class="flex items-center gap-[0.55rem] px-[0.85rem] py-[0.75rem] rounded-2xl border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 text-left cursor-pointer transition-all duration-150 hover:border-blue-500/30 hover:-translate-y-px" onclick={() => selectLauncherSection("plugins")}>
      <span class="text-[1.3rem]">🧩</span>
      <div class="flex flex-col">
        <strong class="text-[0.82rem]">{runningPlugins} Running</strong>
        <span class="text-[0.7rem] text-gray-500">{plugins.length} plugin{plugins.length !== 1 ? "s" : ""}</span>
      </div>
    </button>
    <button class="flex items-center gap-[0.55rem] px-[0.85rem] py-[0.75rem] rounded-2xl border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 text-left cursor-pointer transition-all duration-150 hover:border-blue-500/30 hover:-translate-y-px" onclick={() => selectLauncherSection("channels")}>
      <span class="text-[1.3rem]">📡</span>
      <div class="flex flex-col">
        <strong class="text-[0.82rem]">{activeChannels} Active</strong>
        <span class="text-[0.7rem] text-gray-500">{channels.length} channel{channels.length !== 1 ? "s" : ""}</span>
      </div>
    </button>
  </div>

  <!-- Metrics Row -->
  <div class="grid grid-cols-[repeat(auto-fit,minmax(8rem,1fr))] gap-[0.6rem]">
    <div class="text-center p-[0.7rem] rounded-2xl border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50">
      <span class="block text-[1.5rem] font-bold text-black dark:text-white">{$catalogCount}</span>
      <span class="block text-[0.68rem] uppercase tracking-[0.1em] text-gray-500 mt-[0.15rem]">Catalog Apps</span>
    </div>
    <div class="text-center p-[0.7rem] rounded-2xl border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50">
      <span class="block text-[1.5rem] font-bold text-black dark:text-white">{$runningCount}</span>
      <span class="block text-[0.68rem] uppercase tracking-[0.1em] text-gray-500 mt-[0.15rem]">Running Apps</span>
    </div>
    <div class="text-center p-[0.7rem] rounded-2xl border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50">
      <span class="block text-[1.5rem] font-bold text-black dark:text-white">{agentTools.length}</span>
      <span class="block text-[0.68rem] uppercase tracking-[0.1em] text-gray-500 mt-[0.15rem]">Agent Tools</span>
    </div>
    <div class="text-center p-[0.7rem] rounded-2xl border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50">
      <span class="block text-[1.5rem] font-bold text-black dark:text-white">{providers.length}</span>
      <span class="block text-[0.68rem] uppercase tracking-[0.1em] text-gray-500 mt-[0.15rem]">LLM Providers</span>
    </div>
  </div>

  <!-- Resources -->
  {#if $resourceUsage}
    <div class="grid grid-cols-2 gap-[0.6rem]">
      <div class="p-[0.85rem] rounded-2xl border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 grid gap-[0.35rem]">
        <div class="flex justify-between items-center">
          <span class="text-[0.82rem] font-semibold">Memory</span>
          <strong class="text-[1.1rem]">{$resourceUsage.memory_percent}%</strong>
        </div>
        <div class="h-[0.45rem] rounded-full bg-black/10 dark:bg-white/10 overflow-hidden" style:--fill={`${$resourceUsage.memory_percent}%`}>
          <span class="block h-full rounded-[inherit] {usageTone($resourceUsage.memory_percent) === 'danger' ? 'bg-linear-to-r from-yellow-500 to-red-500' : usageTone($resourceUsage.memory_percent) === 'warning' ? 'bg-linear-to-r from-yellow-500 to-blue-500' : 'bg-linear-to-r from-green-500 to-blue-500'}" style="width: var(--fill)"></span>
        </div>
        <span class="text-[0.72rem] text-gray-500">{formatBytes($resourceUsage.memory_used_bytes)} / {formatBytes($resourceUsage.memory_total_bytes)}</span>
      </div>
      <div class="p-[0.85rem] rounded-2xl border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 grid gap-[0.35rem]">
        <div class="flex justify-between items-center">
          <span class="text-[0.82rem] font-semibold">Disk</span>
          <strong class="text-[1.1rem]">{$resourceUsage.disk_percent}%</strong>
        </div>
        <div class="h-[0.45rem] rounded-full bg-black/10 dark:bg-white/10 overflow-hidden" style:--fill={`${$resourceUsage.disk_percent}%`}>
          <span class="block h-full rounded-[inherit] {usageTone($resourceUsage.disk_percent) === 'danger' ? 'bg-linear-to-r from-yellow-500 to-red-500' : usageTone($resourceUsage.disk_percent) === 'warning' ? 'bg-linear-to-r from-yellow-500 to-blue-500' : 'bg-linear-to-r from-green-500 to-blue-500'}" style="width: var(--fill)"></span>
        </div>
        <span class="text-[0.72rem] text-gray-500">{formatBytes($resourceUsage.disk_used_bytes)} / {formatBytes($resourceUsage.disk_total_bytes)}</span>
      </div>
    </div>
  {/if}

  <!-- Quick Actions -->
  <div class="grid gap-[0.6rem]">
    <h3 class="mt-[0.3rem] mb-0 text-lg font-bold">Quick Actions</h3>
    <div class="grid grid-cols-[repeat(auto-fill,minmax(8rem,1fr))] gap-[0.5rem]">
      <button class="flex flex-col items-center gap-[0.25rem] py-[0.85rem] px-[0.5rem] rounded-2xl border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 cursor-pointer transition-all duration-150 text-center text-black dark:text-white hover:border-blue-500/35 hover:-translate-y-[2px] hover:shadow-[0_4px_12px_rgba(0,0,0,0.1)]" onclick={() => selectLauncherSection("chat")}>
        <span class="text-[1.5rem]">💬</span><strong class="text-[0.78rem]">Chat</strong><span class="text-[0.65rem] text-gray-500">Talk to the agent</span>
      </button>
      <button class="flex flex-col items-center gap-[0.25rem] py-[0.85rem] px-[0.5rem] rounded-2xl border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 cursor-pointer transition-all duration-150 text-center text-black dark:text-white hover:border-blue-500/35 hover:-translate-y-[2px] hover:shadow-[0_4px_12px_rgba(0,0,0,0.1)]" onclick={() => selectLauncherSection("model-settings")}>
        <span class="text-[1.5rem]">🤖</span><strong class="text-[0.78rem]">Models</strong><span class="text-[0.65rem] text-gray-500">Configure LLM</span>
      </button>
      <button class="flex flex-col items-center gap-[0.25rem] py-[0.85rem] px-[0.5rem] rounded-2xl border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 cursor-pointer transition-all duration-150 text-center text-black dark:text-white hover:border-blue-500/35 hover:-translate-y-[2px] hover:shadow-[0_4px_12px_rgba(0,0,0,0.1)]" onclick={() => selectLauncherSection("plugins")}>
        <span class="text-[1.5rem]">🧩</span><strong class="text-[0.78rem]">Plugins</strong><span class="text-[0.65rem] text-gray-500">Manage extensions</span>
      </button>
      <button class="flex flex-col items-center gap-[0.25rem] py-[0.85rem] px-[0.5rem] rounded-2xl border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 cursor-pointer transition-all duration-150 text-center text-black dark:text-white hover:border-blue-500/35 hover:-translate-y-[2px] hover:shadow-[0_4px_12px_rgba(0,0,0,0.1)]" onclick={() => selectLauncherSection("channels")}>
        <span class="text-[1.5rem]">📡</span><strong class="text-[0.78rem]">Channels</strong><span class="text-[0.65rem] text-gray-500">Gateway status</span>
      </button>
      <button class="flex flex-col items-center gap-[0.25rem] py-[0.85rem] px-[0.5rem] rounded-2xl border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 cursor-pointer transition-all duration-150 text-center text-black dark:text-white hover:border-blue-500/35 hover:-translate-y-[2px] hover:shadow-[0_4px_12px_rgba(0,0,0,0.1)]" onclick={() => selectLauncherSection("mcp-tools")}>
        <span class="text-[1.5rem]">🔧</span><strong class="text-[0.78rem]">MCP Tools</strong><span class="text-[0.65rem] text-gray-500">Browse tools</span>
      </button>
      <button class="flex flex-col items-center gap-[0.25rem] py-[0.85rem] px-[0.5rem] rounded-2xl border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 cursor-pointer transition-all duration-150 text-center text-black dark:text-white hover:border-blue-500/35 hover:-translate-y-[2px] hover:shadow-[0_4px_12px_rgba(0,0,0,0.1)]" onclick={() => selectLauncherSection("skills")}>
        <span class="text-[1.5rem]">📘</span><strong class="text-[0.78rem]">Skills</strong><span class="text-[0.65rem] text-gray-500">Skill library</span>
      </button>
      <button class="flex flex-col items-center gap-[0.25rem] py-[0.85rem] px-[0.5rem] rounded-2xl border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 cursor-pointer transition-all duration-150 text-center text-black dark:text-white hover:border-blue-500/35 hover:-translate-y-[2px] hover:shadow-[0_4px_12px_rgba(0,0,0,0.1)]" onclick={() => selectLauncherSection("knowledge")}>
        <span class="text-[1.5rem]">🧠</span><strong class="text-[0.78rem]">Knowledge</strong><span class="text-[0.65rem] text-gray-500">Agent memory</span>
      </button>
      <button class="flex flex-col items-center gap-[0.25rem] py-[0.85rem] px-[0.5rem] rounded-2xl border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 cursor-pointer transition-all duration-150 text-center text-black dark:text-white hover:border-blue-500/35 hover:-translate-y-[2px] hover:shadow-[0_4px_12px_rgba(0,0,0,0.1)]" onclick={() => selectLauncherSection("code-editor")}>
        <span class="text-[1.5rem]">💻</span><strong class="text-[0.78rem]">IDE</strong><span class="text-[0.65rem] text-gray-500">Code editor</span>
      </button>
    </div>
  </div>

  <!-- System Info -->
  {#if $systemInfo}
    <div class="flex gap-[0.5rem] items-center justify-center text-[0.72rem] text-gray-500 pt-[0.5rem] border-t border-black/10 dark:border-white/10">
      <span>NDE-OS v0.2.0</span>
      <span>·</span>
      <span>{$systemInfo.os}/{$systemInfo.arch}</span>
      <span>·</span>
      <span>GPU: {$systemInfo.gpu_detected ? "✓" : "✗"}</span>
      <span>·</span>
      <span>Python: {$systemInfo.python_version ?? "N/A"}</span>
    </div>
  {/if}
</section>id var(--system-color-border);
  }
</style>
