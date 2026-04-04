<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Terminal } from '@xterm/xterm';
  import { FitAddon } from '@xterm/addon-fit';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import '@xterm/xterm/css/xterm.css';

  let { projectPath = "" }: { projectPath?: string } = $props();

  let terminalElement: HTMLElement;
  let term: Terminal;
  let fitAddon: FitAddon;
  let unlisten: (() => void) | null = null;
  const terminalId = crypto.randomUUID();
  let spawned = $state(false);
  let resolvedCwd = $state("");

  export function fit() {
    if (fitAddon) {
      requestAnimationFrame(() => fitAddon.fit());
    }
  }

  let resizeObserver: ResizeObserver | null = null;

  // Resolve to an absolute filesystem path.
  // selectedProjectPath may already be absolute (from list_directory which returns full paths)
  // or relative like "data" / "data/my-project".
  async function resolveAbsPath(relPath: string): Promise<string> {
    if (!relPath) {
      try { return await invoke<string>("get_home_dir"); } catch { return ""; }
    }
    // Already absolute — use as-is (Unix: starts with /, Windows: C:\...)
    if (relPath.startsWith("/") || /^[A-Za-z]:[\\\/]/.test(relPath)) {
      return relPath;
    }
    // Relative sandbox path — join with sandbox root
    try {
      const home = await invoke<string>("get_home_dir");
      return `${home.replace(/\/$/, "")}/${relPath}`;
    } catch {
      return relPath;
    }
  }

  onMount(async () => {
    term = new Terminal({
      theme: {
        background: '#1e1e1e',
        foreground: '#cccccc',
        cursor: '#528bff',
        selectionBackground: 'rgba(82, 139, 255, 0.3)',
      },
      fontFamily: 'Consolas, "Courier New", monospace',
      fontSize: 14,
      cursorBlink: true,
      scrollback: 5000,
      allowTransparency: true
    });
    fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(terminalElement);

    setTimeout(() => { fitAddon.fit(); }, 100);

    term.onData(async (data) => {
      await invoke("write_pty", { id: terminalId, data });
    });

    unlisten = await listen<number[]>(`pty_read_${terminalId}`, (event) => {
      term.write(new Uint8Array(event.payload));
    });

    // Resolve the starting directory
    const cwd = await resolveAbsPath(projectPath);
    resolvedCwd = cwd;

    await invoke("spawn_pty", { id: terminalId, cwd });
    spawned = true;

    resizeObserver = new ResizeObserver(() => { fitAddon.fit(); });
    resizeObserver.observe(terminalElement);
  });

  // React to project changes while terminal is running — cd like VS Code does
  const cdSent = new Set<string>();
  $effect(() => {
    const current = projectPath;   // reactive: re-runs when prop changes
    if (!spawned || cdSent.has(current)) return;
    cdSent.add(current);
    resolveAbsPath(current).then((abs) => {
      if (!abs) return;
      resolvedCwd = abs;
      invoke("write_pty", { id: terminalId, data: `cd "${abs}"\r` });
    });
  });

  onDestroy(() => {
    if (unlisten) unlisten();
    if (term) term.dispose();
    if (resizeObserver) resizeObserver.disconnect();
  });
</script>

<div class="absolute inset-0 flex flex-col bg-[#282c34]">
  <!-- Breadcrumb: shows resolved project path in terminal header -->
  {#if resolvedCwd}
    <div class="flex items-center gap-1.5 px-3 py-1 bg-[#21252b] border-b border-[#181a1f] text-[10px] text-white/30 font-mono shrink-0 select-none">
      <svg class="w-3 h-3 text-white/20 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"></path>
      </svg>
      <span class="truncate">{resolvedCwd}</span>
    </div>
  {/if}
  <div class="flex-1 relative pl-2 pt-1" bind:this={terminalElement}></div>
</div>

<style>
  :global(.xterm .xterm-viewport) {
    overflow-y: scroll !important;
    scrollbar-width: thin !important;
  }
  :global(.xterm-viewport::-webkit-scrollbar) {
    display: block !important;
    width: 8px !important;
  }
  :global(.xterm-viewport::-webkit-scrollbar-thumb) {
    background-color: rgba(255, 255, 255, 0.2) !important;
    border-radius: 4px !important;
  }
</style>
