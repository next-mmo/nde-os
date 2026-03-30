<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  let { onSelectFile, currentFile, onProjectChange, externalRoot = null } = $props<{
    onSelectFile: (path: string, name: string) => void;
    currentFile: string | null;
    onProjectChange?: (selected: boolean) => void;
    externalRoot?: string | null;
  }>();

  type FileEntry = {
    name: string;
    path: string;
    is_dir: boolean;
    size: number;
    modified: string | null;
  };

  let entries = $state<FileEntry[]>([]);
  let currentDir = $state<string>("");
  let projectRoot = $state<string | null>(null);

  // Use the external list command when browsing user-selected folders
  let useExternalCmd = $derived(!!externalRoot);

  async function loadDirectory(path: string) {
    try {
      const cmd = useExternalCmd ? "list_directory_external" : "list_directory";
      entries = await invoke(cmd, { path });
      currentDir = path;
    } catch (e) {
      console.error(e);
    }
  }

  // Go up one directory if we are not at root
  async function goUp() {
    if (currentDir === "" || currentDir === projectRoot) return;
    
    let parts = currentDir.replace(/\\/g, '/').split('/');
    if (parts.length > 0) {
      parts.pop();
      let newDir = parts.join('/');
      await loadDirectory(newDir);
    }
  }

  function closeProject() {
    projectRoot = null;
    currentDir = "";
    onProjectChange?.(false);
  }

  // When externalRoot is provided, auto-load that folder
  onMount(() => {
    if (externalRoot) {
      projectRoot = externalRoot;
      loadDirectory(externalRoot);
    }
  });

  // React to externalRoot changes (e.g., user picks a new folder)
  $effect(() => {
    if (externalRoot && externalRoot !== projectRoot) {
      projectRoot = externalRoot;
      loadDirectory(externalRoot);
    }
  });
</script>

<div class="flex flex-col h-full bg-black/40 text-sm font-sans w-64 border-r border-white/10 shrink-0 select-none overflow-y-auto">
  {#if projectRoot}
    <div class="px-4 py-2 text-xs font-semibold text-white/50 tracking-wider sticky top-0 bg-black/60 backdrop-blur z-10 flex items-center justify-between border-b border-white/5">
      <span class="truncate uppercase max-w-[140px]" title={projectRoot}>{projectRoot.split(/[/\\]/).pop() || 'Explorer'}</span>
      <div class="flex gap-2">
        <button onclick={closeProject} class="text-white/40 hover:text-white transition-colors" title="Close Folder">
          <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path></svg>
        </button>
        {#if currentDir !== projectRoot}
          <button onclick={goUp} class="text-white/40 hover:text-white transition-colors" title="Go Up Directory">↑</button>
          <button onclick={() => loadDirectory(projectRoot!)} class="text-white/40 hover:text-white transition-colors" title="Go to Project Root">🏠</button>
        {/if}
      </div>
    </div>
    
    <div class="flex-1 py-1">
      {#each entries as entry}
        {#if entry.is_dir}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div 
            class="flex items-center gap-2 px-4 py-1.5 hover:bg-white/5 cursor-pointer text-white/70"
            onclick={() => loadDirectory(entry.path)}
          >
            <svg class="w-4 h-4 text-yellow-500/80 shrink-0" fill="currentColor" viewBox="0 0 20 20"><path d="M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z"></path></svg>
            <span class="truncate">{entry.name}</span>
          </div>
        {:else}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div 
            class="flex items-center gap-2 px-4 py-1.5 hover:bg-white/5 cursor-pointer transition-colors {currentFile === entry.path ? 'bg-indigo-500/20 text-indigo-200 border-l-2 border-indigo-400' : 'text-white/60 border-l-2 border-transparent'}"
            onclick={() => onSelectFile(entry.path, entry.name)}
          >
            <svg class="w-4 h-4 text-blue-400 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"></path></svg>
            <span class="truncate">{entry.name}</span>
          </div>
        {/if}
      {/each}
      {#if entries.length === 0}
        <div class="px-4 py-3 text-white/30 italic text-xs text-center">Folder is empty</div>
      {/if}
    </div>
  {:else}
    <div class="flex-1 flex items-center justify-center text-white/30 text-xs italic">
      No folder opened
    </div>
  {/if}
</div>
