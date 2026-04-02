<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Terminal } from '@xterm/xterm';
  import { FitAddon } from '@xterm/addon-fit';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import '@xterm/xterm/css/xterm.css';

  let terminalElement: HTMLElement;
  let term: Terminal;
  let fitAddon: FitAddon;
  let unlisten: (() => void) | null = null;
  const terminalId = crypto.randomUUID();

  export function fit() {
    if (fitAddon) {
      requestAnimationFrame(() => fitAddon.fit());
    }
  }

  let resizeObserver: ResizeObserver | null = null;

  onMount(async () => {
    term = new Terminal({
      theme: { background: '#1e1e1e', foreground: '#cccccc' },
      fontFamily: 'Consolas, "Courier New", monospace',
      fontSize: 14,
      cursorBlink: true,
      scrollback: 5000,
      allowTransparency: true
    });
    fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(terminalElement);

    setTimeout(() => {
      fitAddon.fit();
    }, 100);

    term.onData(async (data) => {
      await invoke("write_pty", { id: terminalId, data });
    });

    unlisten = await listen<number[]>(`pty_read_${terminalId}`, (event) => {
      term.write(new Uint8Array(event.payload));
    });

    let cwd = "";
    try {
      cwd = await invoke<string>("get_home_dir");
    } catch {}

    await invoke("spawn_pty", { id: terminalId, cwd });

    resizeObserver = new ResizeObserver(() => {
      fitAddon.fit();
    });
    resizeObserver.observe(terminalElement);
  });

  onDestroy(() => {
    if (unlisten) unlisten();
    if (term) term.dispose();
    if (resizeObserver) resizeObserver.disconnect();
  });
</script>

<div class="absolute inset-0 bg-[#1e1e1e] pl-2 pt-2" bind:this={terminalElement}>
</div>

<style>
  /* Force xterm viewport to respect standard DOM scroll behavior and show the native overlay scrollbar */
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
