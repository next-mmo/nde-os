<svelte:options runes={true} />

<script lang="ts">
  import type { DesktopWindow } from "🍎/state/desktop.svelte";
  import { onMount, onDestroy } from 'svelte';
  import { Terminal } from '@xterm/xterm';
  import { FitAddon } from '@xterm/addon-fit';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import '@xterm/xterm/css/xterm.css';

  interface Props {
    window: DesktopWindow;
  }
  let { window }: Props = $props();

  let terminalElement: HTMLElement;
  let term: Terminal;
  let fitAddon: FitAddon;
  let unlisten: (() => void) | null = null;
  const terminalId = crypto.randomUUID();

  let resizeObserver: ResizeObserver | null = null;

  onMount(async () => {
    term = new Terminal({
      theme: { background: '#1e1e1e', foreground: '#cccccc' },
      fontFamily: 'SFMono-Regular, Consolas, "Courier New", monospace',
      fontSize: 14,
      cursorBlink: true
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

<div class="relative w-full h-full bg-[#1e1e1e] overflow-hidden">
  <div class="absolute inset-2 overflow-hidden" bind:this={terminalElement}></div>
</div>

<style>
  :global(.xterm-viewport) {
    /* Ensures the scrollbar is styled nicely or hidden */
    overflow-y: auto;
  }
</style>
