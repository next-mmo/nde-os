<script lang="ts">
  import "../app.css";
  import { page } from "$app/stores";
  import { onMount } from "svelte";
  import { refreshAll, systemInfo, runningCount } from "$lib/stores/state";

  interface Props { children: import("svelte").Snippet; }
  let { children }: Props = $props();

  const NAV = [
    { href: "/catalog", label: "Catalog", icon: "📦" },
    { href: "/installed", label: "Installed", icon: "💾" },
    { href: "/running", label: "Running", icon: "▶️" },
    { href: "/logs", label: "Logs", icon: "📋" },
    { href: "/settings", label: "Settings", icon: "⚙️" },
  ];

  onMount(() => { refreshAll(); });
</script>

<div class="shell">
  <!-- Sidebar -->
  <nav class="sidebar">
    <div class="sidebar-brand">
      <span class="brand-icon">🚀</span>
      <div>
        <div class="brand-name">AI Launcher</div>
        <div class="brand-version">v0.2.0</div>
      </div>
    </div>

    <div class="sidebar-nav">
      {#each NAV as item}
        <a
          href={item.href}
          class="nav-item"
          class:active={$page.url.pathname.startsWith(item.href)}
        >
          <span class="nav-icon">{item.icon}</span>
          <span class="nav-label">{item.label}</span>
          {#if item.href === "/running" && $runningCount > 0}
            <span class="nav-badge">{$runningCount}</span>
          {/if}
        </a>
      {/each}
    </div>

    <div class="sidebar-footer">
      {#if $systemInfo}
        <div class="sys-row">
          <span class="sys-label">OS</span>
          <span class="sys-value">{$systemInfo.os}/{$systemInfo.arch}</span>
        </div>
        <div class="sys-row">
          <span class="sys-label">Python</span>
          <span class="sys-value">{$systemInfo.python_version || "not found"}</span>
        </div>
        <div class="sys-row">
          <span class="sys-label">GPU</span>
          <span class="sys-value">{$systemInfo.gpu_detected ? "✅ Detected" : "❌ None"}</span>
        </div>
        <div class="sys-row">
          <span class="sys-label">Apps</span>
          <span class="sys-value">{$systemInfo.total_apps} installed, {$systemInfo.running_apps} running</span>
        </div>
      {:else}
        <div class="sys-loading">Loading system info...</div>
      {/if}
    </div>
  </nav>

  <!-- Main content -->
  <main class="content">
    {@render children()}
  </main>
</div>

<style>
  .shell { display: flex; height: 100vh; overflow: hidden; }

  /* ── Sidebar ── */
  .sidebar {
    width: 220px; flex-shrink: 0;
    background: var(--color-surface);
    border-right: 1px solid var(--color-border);
    display: flex; flex-direction: column;
    user-select: none;
  }

  .sidebar-brand {
    display: flex; align-items: center; gap: 10px;
    padding: 18px 16px 14px; border-bottom: 1px solid var(--color-border);
  }
  .brand-icon { font-size: 24px; }
  .brand-name { font-weight: 700; font-size: 14px; color: var(--color-text); }
  .brand-version { font-size: 10px; color: var(--color-text-muted); font-family: var(--font-mono); }

  .sidebar-nav { flex: 1; padding: 8px; display: flex; flex-direction: column; gap: 2px; }

  .nav-item {
    display: flex; align-items: center; gap: 10px;
    padding: 9px 12px; border-radius: 6px;
    text-decoration: none; color: var(--color-text-dim);
    font-size: 13px; font-weight: 500;
    transition: background 0.15s, color 0.15s;
  }
  .nav-item:hover { background: var(--color-surface-hover); color: var(--color-text); }
  .nav-item.active { background: rgba(139,92,246,0.12); color: var(--color-accent); }
  .nav-icon { font-size: 16px; width: 20px; text-align: center; }
  .nav-label { flex: 1; }
  .nav-badge {
    font-size: 10px; font-weight: 600; background: var(--color-success);
    color: #000; padding: 1px 6px; border-radius: 10px;
    min-width: 16px; text-align: center;
  }

  .sidebar-footer {
    padding: 12px 16px; border-top: 1px solid var(--color-border);
    font-size: 11px;
  }
  .sys-row { display: flex; justify-content: space-between; padding: 3px 0; }
  .sys-label { color: var(--color-text-muted); }
  .sys-value { color: var(--color-text-dim); font-family: var(--font-mono); font-size: 10px; }
  .sys-loading { color: var(--color-text-muted); text-align: center; padding: 8px 0; }

  /* ── Content ── */
  .content {
    flex: 1; overflow-y: auto; padding: 0;
    background: var(--color-bg);
  }
</style>
