<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  let { onSelectFile, currentFile } = $props<{
    onSelectFile: (path: string, name: string) => void;
    currentFile: string | null;
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
  let projects = $state<FileEntry[]>([]);

  async function loadProjects() {
    try {
      let allEntries = await invoke<FileEntry[]>("list_directory", { path: "" });
      projects = allEntries.filter(e => e.is_dir);
    } catch (e) {
      console.error(e);
    }
  }

  async function selectProject(path: string) {
    projectRoot = path;
    await loadDirectory(path);
  }

  async function loadDirectory(path: string) {
    try {
      entries = await invoke("list_directory", { path });
      currentDir = path;
    } catch (e) {
      console.error(e);
    }
  }

  // Go up one directory if we are not at root
  async function goUp() {
    if (currentDir === "" || currentDir === projectRoot) return;
    
    // Normalize path separators conceptually 
    let parts = currentDir.replace(/\\/g, '/').split('/');
    if (parts.length > 0) {
      parts.pop();
      let newDir = parts.join('/');
      await loadDirectory(newDir);
    }
  }

  onMount(() => {
    loadProjects();
  });
</script>

<div class="flex flex-col h-full bg-black/40 text-sm font-sans w-64 border-r border-white/10 shrink-0 select-none overflow-y-auto">
  {#if projectRoot === null}
    <div class="px-4 py-2 text-xs font-semibold text-white/50 tracking-wider sticky top-0 bg-black/60 backdrop-blur z-10 border-b border-white/5">
      <span class="uppercase">Select Project</span>
    </div>
    <div class="flex-1 py-1">
      {#each projects as project}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div 
          class="flex items-center gap-2 px-4 py-1.5 hover:bg-white/5 cursor-pointer text-white/70 hover:text-white transition-colors"
          onclick={() => selectProject(project.path)}
        >
          <svg class="w-4 h-4 text-indigo-400 shrink-0" fill="currentColor" viewBox="0 0 20 20"><path d="M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z"></path></svg>
          <span class="truncate">{project.name}</span>
        </div>
      {/each}
      {#if projects.length === 0}
        <div class="px-4 py-3 text-white/30 italic text-xs text-center">No folders found in workspace</div>
      {/if}
    </div>
  {:else}
    <div class="px-4 py-2 text-xs font-semibold text-white/50 tracking-wider sticky top-0 bg-black/60 backdrop-blur z-10 flex items-center justify-between border-b border-white/5">
      <span class="truncate uppercase max-w-[80px]" title={projectRoot}>{projectRoot.split(/[/\\]/).pop() || 'Explorer'}</span>
      <div class="flex gap-2">
        <button onclick={() => { projectRoot = null; currentDir = ""; }} class="text-white/40 hover:text-white transition-colors" title="Change Project">🔙</button>
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
  {/if}
</div>
