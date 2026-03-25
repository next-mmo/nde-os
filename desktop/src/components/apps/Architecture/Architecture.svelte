<svelte:options runes={true} />

<script lang="ts">
  import * as api from "$lib/api/backend";
  import type {
    PluginStatus,
    ChannelStatus,
    McpServerInfo,
    McpTool,
    SkillInfo,
    KnowledgeEntry,
    ProviderStatus,
  } from "$lib/api/types";
  import {
    catalog,
    installed,
    runningCount,
    catalogCount,
    healthStatus,
    systemInfo,
  } from "$lib/stores/state";

  // ── Tabs State ──
  let activeTab = $state<"architecture" | "compare">("architecture");

  // ── Live data from backend ──
  let plugins = $state<PluginStatus[]>([]);
  let channels = $state<ChannelStatus[]>([]);
  let mcpServers = $state<McpServerInfo[]>([]);
  let mcpTools = $state<McpTool[]>([]);
  let skills = $state<SkillInfo[]>([]);
  let knowledge = $state<KnowledgeEntry[]>([]);
  let providers = $state<ProviderStatus[]>([]);
  let loading = $state(true);

  $effect(() => {
    refresh();
  });

  async function refresh() {
    loading = true;
    try {
      const [p, ch, ms, mt, sk, kn, pr] = await Promise.allSettled([
        api.listPlugins(),
        api.listChannels(),
        api.listMcpServers(),
        api.listMcpTools(),
        api.listSkills(),
        api.listKnowledge(),
        api.listModels(),
      ]);
      plugins = p.status === "fulfilled" ? p.value : [];
      channels = ch.status === "fulfilled" ? ch.value : [];
      mcpServers = ms.status === "fulfilled" ? ms.value : [];
      mcpTools = mt.status === "fulfilled" ? mt.value : [];
      skills = sk.status === "fulfilled" ? sk.value : [];
      knowledge = kn.status === "fulfilled" ? kn.value : [];
      providers = pr.status === "fulfilled" ? pr.value : [];
    } finally {
      loading = false;
    }
  }

  // ── Derived live counts ──
  const runningPlugins = $derived(plugins.filter((p) => p.state === "running").length);
  const connectedChannels = $derived(channels.filter((c) => c.is_running).length);
  const connectedServers = $derived(mcpServers.filter((s) => s.is_connected).length);
  const activeProvider = $derived(providers.find((p) => p.is_active)?.name ?? "None");

  // ── Layer data ──
  const desktopApps = [
    { id: "ai-launcher", label: "AI Launcher", emoji: "🚀", color: "blue" },
    { id: "chat", label: "NDE Chat", emoji: "💬", color: "purple" },
    { id: "browser", label: "Browser", emoji: "🌐", color: "cyan" },
    { id: "shield-browser", label: "Shield Browser", emoji: "🛡️", color: "green" },
    { id: "code-editor", label: "Code Editor", emoji: "📝", color: "orange" },
    { id: "settings", label: "Settings", emoji: "⚙️", color: "gray" },
    { id: "terminal", label: "Terminal", emoji: "⌨️", color: "teal" },
    { id: "file-explorer", label: "File Explorer", emoji: "📂", color: "yellow" },
    { id: "launchpad", label: "Launchpad", emoji: "🔲", color: "pink" },
    { id: "command-center", label: "Command Center", emoji: "🎯", color: "purple" },
    { id: "app-store", label: "App Store", emoji: "🛒", color: "red" },
    { id: "logs", label: "Logs", emoji: "📊", color: "cyan" },
  ];

  const messageBusTypes = [
    { label: "DesktopWindow", kind: "window" },
    { label: "RunningSession", kind: "session" },
    { label: "SystemEvent", kind: "event" },
  ];

  const llmBackends = [
    { id: "claude", label: "Claude SDK", emoji: "🟠", default: true },
    { id: "openai", label: "OpenAI Agents", emoji: "🟢", default: false },
    { id: "google", label: "Google ADK", emoji: "🔵", default: false },
    { id: "codex", label: "Codex CLI", emoji: "🟦", default: false },
  ];

  const subsystems = [
    { icon: "🧠", title: "Memory", desc: "File Store + NewO Vector DB, Session History, Auto-Learn, Venv Isolation" },
    { icon: "🔧", title: "Tools", desc: "38+ Built-in Tools, Policy Engine (deny > allow > profile), Tool Registry" },
    { icon: "🔌", title: "MCP", desc: "stdio/HTTP/SSE Transport, OAuth, Dynamic Discovery" },
    { icon: "🛡️", title: "Security", desc: "Guardian AI, Injection Scanner, Audit Log, Rate Limiter" },
    { icon: "📦", title: "Sandbox", desc: "Filesystem Jail, Path Canonicalization, Symlink Defense, Env-Var Jailing" },
    { icon: "🧩", title: "Plugins", desc: "Manifest Discovery, Hook System, Daemon Lifecycle, Hot Reload" },
    { icon: "📡", title: "Channels", desc: "Telegram, Discord, Slack, REST API, Web Chat, CLI Normalizer" },
    { icon: "⚡", title: "uv Environment", desc: "Auto-Bootstrap uv, Per-App .venv, 10-100× Faster, Python Lock" },
    { icon: "🗂️", title: "Knowledge", desc: "Key-Value Store, Category Tagging, Full-Text Search, Auto-Index" },
    { icon: "🎯", title: "Skills", desc: "YAML Frontmatter Discovery, Trigger Matching, Prompt Builder" },
    { icon: "🤖", title: "LLM Providers", desc: "Multi-Provider Switch, API Key Mgmt, GGUF Local, Codex OAuth" },
    { icon: "📊", title: "System Metrics", desc: "Memory/Disk Usage, App Status Polling, Health Check, Resource Monitor" },
  ];

  // ── Comparison Data ──
  const competitors = ["OpenClaw", "OpenFang", "IronClaw", "ZeroClaw", "Moltis", "PocketPaw", "NDE-OS"];
  
  const comparisonData = [
    {
      category: "Identity",
      rows: [
        ["Language", "TypeScript", "Rust", "Rust", "Rust", "Rust", "Python", "Rust ✨"],
        ["Type", "AI assistant", "Agent OS", "Security agent", "Lightweight", "Runtime", "Ext platform", "AI Desktop OS 🌟"],
        ["Binary size", "~200 MB", "32 MB", "~20 MB", "🥇 3.4 MB", "44 MB", "~50 MB", "Target <30MB ⏳"],
        ["RAM usage", "~300 MB", "~50 MB", "~80 MB", "🥇 <5 MB", "~60 MB", "~120 MB", "Target <50MB ⏳"],
        ["Boot time", "~2s", "<200ms", "unknown", "🥇 <10ms", "unknown", "~3s", "Target <500ms ⏳"],
      ]
    },
    {
      category: "Agent Runtime",
      rows: [
        ["Agent loop", "🥇 ✅", "✅", "✅", "✅", "✅", "❌", "✅ verified"],
        ["Multi-provider LLM", "✅", "🥇 26", "✅", "✅ trait", "✅", "✅", "⭐ ✅ 6 + hot-swap"],
        ["Local LLM", "✅", "✅", "✅", "✅", "✅", "✅ ext", "⭐ ✅ auto-launch"],
        ["Streaming", "✅", "✅", "✅", "✅", "✅", "✅ SSE", "✅ SSE streaming"],
        ["LLM hot-swap", "❌", "partial", "❌", "❌", "❌", "❌", "⭐ ✅ runtime switch"],
        ["Auto-scheduling", "❌", "🥇 Hands", "❌", "✅", "❌", "❌", "🔨 building"],
      ]
    },
    {
      category: "Sandbox & Security",
      rows: [
        ["Sandbox type", "❌ none", "WASM dual", "🥇 WASM cap", "WASM", "WASM+Apple", "uv venv", "⭐ OS jail"],
        ["Path traversal defense", "❌", "via WASM", "via WASM", "via WASM", "via WASM", "❌", "⭐ ✅ canonicalize"],
        ["Symlink defense", "❌", "via WASM", "via WASM", "via WASM", "via WASM", "❌", "⭐ ✅ resolve+verify"],
        ["Env-var jailing", "❌", "❌", "partial", "❌", "❌", "❌", "⭐ ✅ full (12 vars)"],
        ["Prompt injection scan", "❌", "✅", "🥇 ✅", "❌", "❌", "❌", "✅ verified"],
        ["Credential isolation", "❌ plaintext", "❌", "🥇 encrypted", "❌", "❌", "✅ JWT", "✅ env-per-sandbox"],
        ["Compute metering", "❌", "✅ fuel", "❌", "✅ fuel", "✅", "❌", "✅ token+time"],
      ]
    },
    {
      category: "Tools & MCP",
      rows: [
        ["Built-in tools", "~10", "🥇 38", "~10", "~15", "~15", "0", "✅ 6 (jailed)"],
        ["MCP client", "✅", "✅", "✅", "✅", "✅", "❌", "✅ stdio JSON-RPC"],
        ["MCP server", "❌", "🥇 ✅", "❌", "❌", "❌", "❌", "✅ expose tools"],
        ["Dynamic tools", "❌", "❌", "✅ WASM", "✅", "❌", "❌", "⭐ ✅ plugin engine"],
        ["Sandbox enforcement", "❌", "✅", "✅", "✅", "✅", "partial", "⭐ ✅ filesystem jail"],
      ]
    },
    {
      category: "Plugins (Phase 2)",
      rows: [
        ["Plugin engine", "❌", "❌", "❌", "❌", "❌", "✅ ext", "⭐ ✅ manifest v2"],
        ["Plugin lifecycle", "❌", "❌", "❌", "❌", "❌", "partial", "⭐ ✅ discover→run"],
        ["Hook system", "❌", "❌", "❌", "❌", "❌", "❌", "⭐ ✅ 9 hook types"],
        ["Plugin tool merge", "❌", "❌", "❌", "❌", "❌", "❌", "⭐ ✅ auto-merge"],
      ]
    },
    {
      category: "Skills & Knowledge",
      rows: [
        ["Skill format", "🥇 SKILL.md", "HAND+SKILL", "❌", "SKILL.md", "SKILL.md", "ext.json", "✅ SKILL.md"],
        ["Knowledge graph", "❌", "🥇 ✅", "❌", "❌", "❌", "❌", "✅ SQLite"],
        ["Conversation persist", "✅ MD files", "🥇 cross-ch", "✅ encrypt", "✅ log", "✅", "✅ JSON", "✅ SQLite"],
      ]
    },
    {
      category: "Channels & CLI",
      rows: [
        ["Desktop chat UI", "❌", "❌", "❌", "❌", "❌", "❌", "⭐ ✅ REST+UI"],
        ["Telegram", "✅", "✅", "✅", "✅", "✅", "❌", "✅ long-poll"],
        ["Total channels", "🥇 10+", "40+", "5", "4", "4", "1", "✅ 2 (REST+Tele)"],
        ["CLI binary", "✅", "✅", "✅", "✅", "✅", "❌", "⭐ ✅ `nde`"],
        ["Streaming REPL", "✅", "❌", "❌", "❌", "❌", "❌", "⭐ ✅ stream repl"],
      ]
    },
    {
      category: "Desktop OS Experience (NDE-OS Exclusive)",
      rows: [
        ["macOS-style desktop", "❌", "❌", "❌", "❌", "❌", "❌", "⭐ ✅ Tauri+Svelte"],
        ["Window management", "❌", "❌", "❌", "❌", "❌", "❌", "⭐ ✅ drag/resize/min"],
        ["Animated dock", "❌", "❌", "❌", "❌", "❌", "❌", "⭐ ✅ macOS-style"],
        ["App catalog", "❌", "❌", "❌", "❌", "❌", "✅ ext", "⭐ ✅ manifest+uv"],
        ["Per-app sandbox venv", "❌", "❌", "❌", "❌", "❌", "✅ uv", "⭐ ✅ uv+jail"],
        ["Built-in Code Editor", "❌", "❌", "❌", "❌", "❌", "❌", "⭐ ✅ editor+term+AI"],
      ]
    }
  ];

  function formatTableCell(text: string) {
    if (text.includes("⭐")) {
      return text.replace("⭐", '<span class="star-badge">⭐</span>');
    }
    if (text.includes("🥇")) {
      return text.replace("🥇", '<span class="gold-badge">🥇</span>');
    }
    if (text.includes("❌")) return `<span class="cross-text">${text}</span>`;
    if (text.includes("✅")) return `<span class="check-text">${text}</span>`;
    if (text.includes("⚠️")) return `<span class="warn-text">${text}</span>`;
    return text;
  }
</script>

<section class="arch-app">
  <div class="arch-header">
    <div class="header-left">
      <div>
        <p class="eyebrow">System</p>
        <h2>Architecture & Analysis</h2>
      </div>
      
      <div class="tabs">
        <button class="tab-btn" class:active={activeTab === 'architecture'} onclick={() => activeTab = 'architecture'}>
          <span class="tab-icon">🖥️</span> Architecture
        </button>
        <button class="tab-btn" class:active={activeTab === 'compare'} onclick={() => activeTab = 'compare'}>
          <span class="tab-icon">📊</span> Ecosystem Compare
        </button>
      </div>
    </div>
    
    <div class="arch-header-right">
      <div class="live-badge" class:online={!loading}>
        <span class="live-dot"></span>
        {loading ? "Loading..." : "Live"}
      </div>
      <button class="action-btn" onclick={refresh} disabled={loading}>↻ Refresh</button>
    </div>
  </div>

  {#if activeTab === 'architecture'}
    <div class="tab-content architecture-tab">
      <p class="intro">Interactive system architecture diagram — all counters reflect real-time backend state.</p>

      <!-- ═══ LIVE STATS BAR ═══ -->
      <div class="stats-bar">
        <div class="stat-chip"><span class="stat-val">{$catalogCount}</span><span class="stat-lbl">Catalog</span></div>
        <div class="stat-chip"><span class="stat-val">{$runningCount}</span><span class="stat-lbl">Running</span></div>
        <div class="stat-chip"><span class="stat-val">{$installed.length}</span><span class="stat-lbl">Installed</span></div>
        <div class="stat-chip accent"><span class="stat-val">{$healthStatus}</span><span class="stat-lbl">Server</span></div>
        <div class="stat-chip"><span class="stat-val">{activeProvider}</span><span class="stat-lbl">LLM</span></div>
      </div>

      <!-- ═══ LAYER 1: DESKTOP SHELL ═══ -->
      <div class="layer">
        <span class="layer-tag">Layer 1 · Desktop Shell</span>
        <div class="app-row">
          {#each desktopApps as app (app.id)}
            <div class="app-chip c-{app.color}">
              <span class="ic">{app.emoji}</span>
              {app.label}
            </div>
          {/each}
        </div>
      </div>

      <!-- ═══ LAYER 2: SESSION BUS ═══ -->
      <div class="layer">
        <span class="layer-tag">Layer 2 · Event-Driven Session Bus</span>
        <div class="bus-row">
          <div class="bus-pipe"></div>
          {#each messageBusTypes as msg (msg.label)}
            <span class="msg-pill {msg.kind}">{msg.label}</span>
          {/each}
          <div class="bus-pipe"></div>
        </div>
        <div class="bus-detail">
          <div class="bus-box">Window Manager<br><small>z-index · fullscreen · drag · resize</small></div>
          <span class="bus-arrow">⇄</span>
          <div class="bus-box">Session Router<br><small>embedded · windowed · drawer · fullscreen</small></div>
          <span class="bus-arrow">⇄</span>
          <div class="bus-box">Tauri 2 IPC Bridge<br><small>invoke + emit · async commands</small></div>
        </div>
      </div>

      <!-- ═══ LAYER 3: AGENT PIPELINE ═══ -->
      <div class="layer">
        <span class="layer-tag">Layer 3 · Agent Pipeline</span>
        <div class="pipeline-grid">
          <div class="security-badges">
            <div class="badge">🔒 Sandbox Jail</div>
            <div class="badge">🛣️ Path Canon.</div>
            <div class="badge">🔗 Symlink Defense</div>
            <div class="badge">🌍 Env Isolation</div>
          </div>

          <div class="pipeline-center">
            <div class="pipeline-box main">Agent Loop (Orchestrator)</div>
            <span class="pipeline-arrow">⬇</span>
            <div class="pipeline-box">Agent Router (Dispatcher)</div>
            <span class="registry-label">Backend Registry</span>
            <div class="sdk-row">
              {#each llmBackends as sdk (sdk.id)}
                <div class="sdk-card {sdk.id}">
                  {#if sdk.default}<span class="default-badge">Default ★</span>{/if}
                  <strong>{sdk.emoji} {sdk.label}</strong>
                  <div class="sdk-features">
                    <span>Streaming</span><span>Tools</span><span>MCP</span><span>Multi-Turn</span>
                  </div>
                </div>
              {/each}
            </div>
          </div>

          <div class="sidebar-cards">
            <div class="notifier-card">🔔 Notifier</div>
            <div class="api-card">
              <h4>🖥️ REST API + Tauri IPC</h4>
              <ul>
                <li><span class="dot"></span>Smart Invoke (IPC/HTTP)</li>
                <li><span class="dot"></span>WebSocket Streaming</li>
                <li><span class="dot"></span>Session Management</li>
                <li><span class="dot"></span>App Lifecycle CRUD</li>
                <li><span class="dot"></span>Plugin Discovery</li>
                <li><span class="dot"></span>Health Monitor</li>
              </ul>
            </div>
          </div>
        </div>
      </div>

      <!-- ═══ LAYER 4: SUBSYSTEMS ═══ -->
      <div class="layer">
        <span class="layer-tag">Layer 4 · Core Subsystems</span>
        <div class="live-counters">
          <div class="lc-chip"><span class="lc-val">{runningPlugins}/{plugins.length}</span><span class="lc-lbl">Plugins Running</span></div>
          <div class="lc-chip"><span class="lc-val">{connectedChannels}/{channels.length}</span><span class="lc-lbl">Channels Online</span></div>
          <div class="lc-chip"><span class="lc-val">{connectedServers}/{mcpServers.length}</span><span class="lc-lbl">MCP Servers</span></div>
          <div class="lc-chip"><span class="lc-val">{mcpTools.length}</span><span class="lc-lbl">MCP Tools</span></div>
          <div class="lc-chip"><span class="lc-val">{skills.length}</span><span class="lc-lbl">Skills</span></div>
          <div class="lc-chip"><span class="lc-val">{knowledge.length}</span><span class="lc-lbl">Knowledge</span></div>
          <div class="lc-chip"><span class="lc-val">{providers.length}</span><span class="lc-lbl">Providers</span></div>
        </div>
        <div class="sub-grid">
          {#each subsystems as sub (sub.title)}
            <div class="sub-card">
              <h4><span class="sub-icon">{sub.icon}</span>{sub.title}</h4>
              <p>{sub.desc}</p>
            </div>
          {/each}
        </div>
      </div>

      <!-- ═══ LEGEND ═══ -->
      <div class="legend">
        <div class="legend-item"><div class="legend-line data"></div>Data Flow</div>
        <div class="legend-item"><div class="legend-line control"></div>Control Flow</div>
        <span class="legend-stack">Stack: Rust Core → Tauri 2 → Svelte 5 · TanStack Query</span>
      </div>

      <!-- ═══ SYSTEM INFO ═══ -->
      {#if $systemInfo}
        <div class="sys-bar">
          <span>{$systemInfo.os} / {$systemInfo.arch}</span>
          <span>Python {$systemInfo.python_version ?? "N/A"}</span>
          <span>uv {$systemInfo.uv.uv_version || "N/A"}</span>
          <span>GPU: {$systemInfo.gpu_detected ? "✅" : "❌"}</span>
        </div>
      {/if}
    </div>
  {:else}
    <div class="tab-content compare-tab">
      <div class="compare-intro">
        <div class="compare-legend">
          <span><span class="check-text">✅ Built</span></span>
          <span><span class="warn-text">🔨 Building</span></span>
          <span><span class="cross-text">❌ Missing</span></span>
          <span><span class="star-badge">⭐</span> NDE-OS Unique</span>
          <span><span class="gold-badge">🥇</span> Best in class</span>
        </div>
        <p>Comparison of NDE-OS against Rust agents, TypeScript agents, and Python platforms based on the R&D Agent Looping analysis.</p>
      </div>

      <div class="compare-tables">
        {#each comparisonData as table}
          <div class="compare-section">
            <h3 class="compare-heading">{table.category}</h3>
            <div class="table-container">
              <table>
                <thead>
                  <tr>
                    <th class="feature-col">Feature</th>
                    {#each competitors as vendor}
                      <th class:is-nde={vendor === "NDE-OS"}>{vendor}</th>
                    {/each}
                  </tr>
                </thead>
                <tbody>
                  {#each table.rows as row}
                    <tr>
                      <td class="feature-name">{row[0]}</td>
                      {#each row.slice(1) as cell, ix}
                        <td class:is-nde={ix === competitors.length - 1}>
                          {@html formatTableCell(cell as string)}
                        </td>
                      {/each}
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</section>

<style>
  .arch-app { height: 100%; overflow: auto; padding: 1.2rem; display: flex; flex-direction: column; gap: 1rem; }
  
  /* ── Header & Tabs ── */
  .arch-header { display: flex; justify-content: space-between; align-items: flex-end; gap: 1rem; flex-wrap: wrap; margin-bottom: 0.5rem; }
  .header-left { display: flex; flex-direction: column; gap: 0.8rem; }
  .eyebrow { margin: 0; font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.14em; color: var(--system-color-text-muted); }
  h2 { margin: 0; font-size: 1.6rem; letter-spacing: -0.02em; }
  
  .tabs { display: flex; gap: 0.5rem; background: hsla(var(--system-color-dark-hsl, 220 40% 8%) / 0.5); padding: 0.3rem; border-radius: 0.8rem; border: 1px solid var(--system-color-border); }
  .tab-btn { 
    background: transparent; border: none; color: var(--system-color-text-muted); padding: 0.5rem 1rem; 
    border-radius: 0.5rem; font-size: 0.82rem; font-weight: 600; cursor: pointer; transition: all 0.2s;
    display: flex; align-items: center; gap: 0.4rem;
  }
  .tab-btn:hover { color: var(--system-color-text); background: hsla(0 0% 100% / 0.05); }
  .tab-btn.active { background: var(--system-color-primary, hsl(215 90% 60%)); color: #fff; box-shadow: 0 2px 10px hsla(215 90% 60% / 0.3); }
  
  .arch-header-right { display: flex; align-items: center; gap: .55rem; padding-bottom: 0.4rem; }
  .action-btn { border-radius: 999px; padding: 0.5rem 0.9rem; font-size: 0.8rem; cursor: pointer; border: 1px solid var(--system-color-border); background: var(--system-color-panel); color: var(--system-color-text); }
  .live-badge { display: flex; align-items: center; gap: .3rem; padding: .3rem .65rem; border-radius: 999px; font-size: .72rem; font-weight: 700; text-transform: uppercase; letter-spacing: .08em; background: hsla(0 0% 50% / 0.1); border: 1px solid hsla(0 0% 50% / 0.15); color: var(--system-color-text-muted); }
  .live-badge.online { background: hsla(160 60% 50% / 0.1); border-color: hsla(160 60% 50% / 0.25); color: var(--system-color-success); }
  .live-dot { width: .45rem; height: .45rem; border-radius: 50%; background: currentColor; }
  .live-badge.online .live-dot { animation: pulse 2s infinite; }
  @keyframes pulse { 0%,100%{opacity:1} 50%{opacity:.3} }

  .intro { margin: 0 0 1rem; font-size: 0.85rem; color: var(--system-color-text-muted); max-width: 620px; }

  .tab-content { display: flex; flex-direction: column; gap: 1rem; animation: fade-in 0.2s ease-out; }
  @keyframes fade-in { from { opacity: 0; transform: translateY(5px); } to { opacity: 1; transform: translateY(0); } }

  /* ── Stats Bar ── */
  .stats-bar { display: flex; gap: .5rem; flex-wrap: wrap; }
  .stat-chip { padding: .45rem .75rem; border-radius: .65rem; display: flex; flex-direction: column; align-items: center; gap: .1rem; background: var(--system-color-panel); border: 1px solid var(--system-color-border); min-width: 5rem; }
  .stat-chip.accent { border-color: hsla(160 60% 50% / 0.25); }
  .stat-val { font-size: .95rem; font-weight: 700; }
  .stat-lbl { font-size: .62rem; color: var(--system-color-text-muted); text-transform: uppercase; letter-spacing: .08em; }

  /* ── Layer wrapper ── */
  .layer { position: relative; border-radius: 1rem; border: 1px solid var(--system-color-border); background: var(--system-color-panel); padding: 1.1rem 1.2rem; }
  .layer-tag { position: absolute; top: -.5rem; left: 1rem; background: var(--system-color-bg, #181c2a); padding: 0 .45rem; font-size: .65rem; font-weight: 700; text-transform: uppercase; letter-spacing: .12em; color: var(--system-color-primary, hsl(215 90% 60%)); }

  /* ── L1: App chips ── */
  .app-row { display: flex; flex-wrap: wrap; gap: .5rem; justify-content: center; margin-top: .3rem; }
  .app-chip { display: flex; align-items: center; gap: .35rem; padding: .4rem .7rem .4rem .4rem; border-radius: .7rem; font-size: .78rem; font-weight: 600; border: 1px solid; white-space: nowrap; cursor: default; transition: all .15s; }
  .ic { font-size: .95rem; }
  .c-blue { background: hsla(215 80% 55% / 0.08); border-color: hsla(215 80% 55% / 0.2); }
  .c-purple { background: hsla(262 70% 60% / 0.08); border-color: hsla(262 70% 60% / 0.2); }
  .c-cyan { background: hsla(188 80% 55% / 0.08); border-color: hsla(188 80% 55% / 0.2); }
  .c-green { background: hsla(160 60% 50% / 0.08); border-color: hsla(160 60% 50% / 0.2); }
  .c-orange { background: hsla(25 90% 55% / 0.08); border-color: hsla(25 90% 55% / 0.2); }
  .c-gray { background: hsla(215 15% 55% / 0.08); border-color: hsla(215 15% 55% / 0.15); }
  .c-teal { background: hsla(170 60% 45% / 0.08); border-color: hsla(170 60% 45% / 0.2); }
  .c-yellow { background: hsla(48 90% 55% / 0.08); border-color: hsla(48 90% 55% / 0.15); }
  .c-pink { background: hsla(330 70% 60% / 0.08); border-color: hsla(330 70% 60% / 0.2); }
  .c-red { background: hsla(0 75% 55% / 0.08); border-color: hsla(0 75% 55% / 0.2); }

  /* ── L2: Bus ── */
  .bus-row { display: flex; align-items: center; justify-content: center; gap: .8rem; margin: .5rem 0; flex-wrap: wrap; }
  .bus-pipe { flex: 1; height: 2px; max-width: 50px; background: linear-gradient(90deg, transparent, var(--system-color-primary, #3898ff), transparent); }
  .msg-pill { padding: .3rem .7rem; border-radius: 999px; font-size: .72rem; font-weight: 700; border: 1px solid; }
  .msg-pill.window { background: hsla(188 80% 55% / 0.1); border-color: hsla(188 80% 55% / 0.25); color: hsl(188 80% 60%); }
  .msg-pill.session { background: hsla(160 60% 50% / 0.1); border-color: hsla(160 60% 50% / 0.25); color: hsl(160 60% 55%); }
  .msg-pill.event { background: hsla(25 90% 55% / 0.1); border-color: hsla(25 90% 55% / 0.25); color: hsl(25 90% 60%); }
  .bus-detail { display: flex; align-items: center; gap: .6rem; justify-content: center; margin-top: .5rem; }
  .bus-box { padding: .45rem .75rem; border-radius: .6rem; background: hsla(var(--system-color-dark-hsl, 220 40% 8%) / 0.3); border: 1px solid var(--system-color-border); font-size: .75rem; font-weight: 600; text-align: center; }
  .bus-box small { color: var(--system-color-text-muted); font-weight: 400; font-size: .65rem; }
  .bus-arrow { color: var(--system-color-text-muted); font-size: .85rem; }

  /* ── L3: Pipeline ── */
  .pipeline-grid { display: grid; grid-template-columns: auto 1fr auto; gap: .8rem; align-items: start; }
  .pipeline-center { display: flex; flex-direction: column; align-items: center; gap: .45rem; }
  .pipeline-box { padding: .55rem 1.2rem; border-radius: .7rem; font-weight: 700; font-size: .84rem; background: hsla(var(--system-color-dark-hsl, 220 40% 8%) / 0.3); border: 1px solid var(--system-color-border); text-align: center; width: 100%; max-width: 300px; }
  .pipeline-box.main { border-color: hsla(215 80% 55% / 0.3); box-shadow: 0 0 16px hsla(215 80% 55% / 0.06); }
  .pipeline-arrow { color: var(--system-color-primary, hsl(215 90% 60%)); font-size: 1rem; }
  .registry-label { font-size: .6rem; color: var(--system-color-text-muted); text-transform: uppercase; letter-spacing: .1em; }
  .security-badges { display: flex; flex-direction: column; gap: .3rem; }
  .badge { padding: .3rem .55rem; border-radius: .45rem; font-size: .68rem; font-weight: 600; background: hsla(0 70% 55% / 0.05); border: 1px solid hsla(0 70% 55% / 0.12); color: hsla(0 70% 70% / 0.9); }
  .sidebar-cards { display: flex; flex-direction: column; gap: .45rem; }
  .notifier-card { padding: .45rem .65rem; border-radius: .55rem; font-size: .72rem; font-weight: 600; background: hsla(48 80% 55% / 0.06); border: 1px solid hsla(48 80% 55% / 0.15); color: hsla(48 80% 65% / 0.9); }
  .api-card { padding: .65rem; border-radius: .7rem; background: hsla(var(--system-color-dark-hsl, 220 40% 8%) / 0.3); border: 1px solid var(--system-color-border); min-width: 150px; }
  .api-card h4 { font-size: .75rem; margin-bottom: .35rem; color: var(--system-color-primary, hsl(215 90% 60%)); }
  .api-card ul { list-style: none; display: flex; flex-direction: column; gap: .2rem; padding: 0;}
  .api-card li { font-size: .68rem; color: var(--system-color-text-muted); display: flex; align-items: center; gap: .3rem; }
  .dot { width: .3rem; height: .3rem; border-radius: 50%; background: var(--system-color-primary, hsl(215 90% 60%)); flex-shrink: 0; }
  .sdk-row { display: flex; flex-wrap: wrap; gap: .5rem; justify-content: center; margin-top: .5rem; }
  .sdk-card { padding: .5rem .7rem; border-radius: .7rem; border: 1px solid; min-width: 130px; text-align: center; position: relative; }
  .sdk-card strong { font-size: .75rem; display: flex; align-items: center; gap: .2rem; justify-content: center; margin-bottom: .2rem; }
  .sdk-features { display: flex; flex-wrap: wrap; gap: .2rem; justify-content: center; }
  .sdk-features span { font-size: .58rem; padding: .1rem .3rem; border-radius: 999px; background: hsla(0 0% 100% / 0.05); color: var(--system-color-text-muted); }
  .default-badge { position: absolute; top: -.35rem; right: -.15rem; font-size: .55rem; padding: .08rem .3rem; border-radius: 999px; background: var(--system-color-warning, hsl(48 95% 55%)); color: #000; font-weight: 800; }
  .sdk-card.claude { background: hsla(25 90% 55% / 0.05); border-color: hsla(25 90% 55% / 0.18); }
  .sdk-card.openai { background: hsla(160 60% 50% / 0.05); border-color: hsla(160 60% 50% / 0.18); }
  .sdk-card.google { background: hsla(215 80% 55% / 0.05); border-color: hsla(215 80% 55% / 0.18); }
  .sdk-card.codex { background: hsla(170 60% 45% / 0.05); border-color: hsla(170 60% 45% / 0.18); }

  /* ── L4: Subsystems ── */
  .live-counters { display: flex; flex-wrap: wrap; gap: .4rem; margin-bottom: .6rem; }
  .lc-chip { display: flex; align-items: center; gap: .35rem; padding: .3rem .6rem; border-radius: 999px; background: hsla(var(--system-color-primary-hsl, 215 90% 60%) / 0.06); border: 1px solid hsla(var(--system-color-primary-hsl, 215 90% 60%) / 0.12); font-size: .68rem; }
  .lc-val { font-weight: 700; color: var(--system-color-primary, hsl(215 90% 60%)); }
  .lc-lbl { color: var(--system-color-text-muted); }
  .sub-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: .6rem; }
  .sub-card { padding: .7rem; border-radius: .75rem; background: hsla(var(--system-color-dark-hsl, 220 40% 8%) / 0.3); border: 1px solid var(--system-color-border); display: flex; flex-direction: column; gap: .3rem; transition: all .15s; }
  .sub-card:hover { border-color: hsla(var(--system-color-primary-hsl, 215 90% 60%) / 0.25); }
  .sub-card h4 { font-size: .78rem; font-weight: 700; display: flex; align-items: center; gap: .35rem; margin: 0; }
  .sub-icon { font-size: .9rem; }
  .sub-card p { font-size: .68rem; color: var(--system-color-text-muted); line-height: 1.5; margin: 0; }

  /* ── Legend ── */
  .legend { display: flex; gap: 1.2rem; justify-content: flex-end; align-items: center; flex-wrap: wrap; }
  .legend-item { display: flex; align-items: center; gap: .35rem; font-size: .7rem; color: var(--system-color-text-muted); }
  .legend-line { width: 1.6rem; height: 2px; border-radius: 1px; }
  .legend-line.data { background: var(--system-color-primary, hsl(215 90% 60%)); }
  .legend-line.control { background-image: repeating-linear-gradient(90deg, var(--system-color-warning, hsl(48 95% 55%)) 0, var(--system-color-warning, hsl(48 95% 55%)) 4px, transparent 4px, transparent 8px); }
  .legend-stack { font-size: .62rem; color: hsla(215 15% 55% / 0.5); }

  /* ── System bar ── */
  .sys-bar { display: flex; gap: .8rem; flex-wrap: wrap; padding: .5rem .8rem; border-radius: .6rem; background: hsla(var(--system-color-dark-hsl, 220 40% 8%) / 0.3); border: 1px solid var(--system-color-border); font-size: .7rem; color: var(--system-color-text-muted); }

  /* ── Compare Tab Styles ── */
  .compare-intro { display: flex; flex-direction: column; gap: 0.8rem; margin-bottom: 1.5rem; }
  .compare-intro p { margin: 0; font-size: 0.9rem; color: var(--system-color-text-muted); }
  
  .compare-legend {
    display: flex; gap: 1rem; flex-wrap: wrap; font-size: 0.75rem;
    padding: 0.6rem 1rem; background: hsla(0 0% 100% / 0.03); border: 1px solid var(--system-color-border);
    border-radius: 0.5rem; align-items: center;
  }

  .compare-tables { display: flex; flex-direction: column; gap: 2rem; }
  .compare-section { display: flex; flex-direction: column; gap: 0.8rem; }
  .compare-heading { margin: 0; font-size: 1rem; color: var(--system-color-primary, hsl(215 90% 60%)); display: flex; align-items: center; gap: 0.5rem; }
  .compare-heading::after { content: ""; flex: 1; height: 1px; background: linear-gradient(90deg, var(--system-color-border), transparent); }

  .table-container { overflow-x: auto; border-radius: 0.8rem; border: 1px solid var(--system-color-border); background: var(--system-color-panel); }
  table { width: 100%; border-collapse: collapse; text-align: left; font-size: 0.8rem; }
  th, td { padding: 0.75rem 1rem; border-bottom: 1px solid var(--system-color-border); white-space: nowrap; }
  th { font-weight: 600; color: var(--system-color-text-muted); text-transform: uppercase; letter-spacing: 0.05em; font-size: 0.7rem; background: hsla(0 0% 0% / 0.2); }
  td:not(.feature-name) { color: var(--system-color-text-muted); }
  tr:last-child td { border-bottom: none; }
  tr:hover td { background: hsla(0 0% 100% / 0.02); }
  
  .feature-col { min-width: 180px; }
  .feature-name { font-weight: 600; color: var(--system-color-text); }
  
  /* Highlight NDE-OS column */
  th.is-nde { background: hsla(215 90% 60% / 0.15); color: var(--system-color-primary, hsl(215 90% 60%)); border-left: 1px solid hsla(215 90% 60% / 0.3); border-right: 1px solid hsla(215 90% 60% / 0.3); }
  td.is-nde { background: hsla(215 90% 60% / 0.05); color: var(--system-color-text); border-left: 1px solid hsla(215 90% 60% / 0.3); border-right: 1px solid hsla(215 90% 60% / 0.3); font-weight: 500; }
  
  :global(.check-text) { color: var(--system-color-success, #34d399); font-weight: 600; }
  :global(.cross-text) { color: var(--system-color-danger, #f87171); opacity: 0.7; }
  :global(.warn-text) { color: var(--system-color-warning, #fbbf24); }
  :global(.star-badge) { display: inline-flex; align-items: center; justify-content: center; background: hsla(215 90% 60% / 0.2); border-radius: 0.3rem; padding: 0.1rem 0.25rem; font-size: 0.65rem; border: 1px solid hsla(215 90% 60% / 0.4); margin-right: 0.3rem; }
  :global(.gold-badge) { display: inline-flex; align-items: center; justify-content: center; background: hsla(43 100% 50% / 0.2); border-radius: 0.3rem; padding: 0.1rem 0.25rem; font-size: 0.65rem; border: 1px solid hsla(43 100% 50% / 0.4); margin-right: 0.3rem; }

  @media (max-width: 900px) {
    .pipeline-grid { grid-template-columns: 1fr; }
    .pipeline-grid > * { justify-self: center; }
    .security-badges { flex-direction: row; flex-wrap: wrap; }
    .arch-header { flex-direction: column; align-items: flex-start; }
    .arch-header-right { align-self: flex-end; }
  }
</style>
