<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  let { isOpen = $bindable(false), onSelect } = $props<{
    isOpen: boolean;
    onSelect: (path: string) => void;
  }>();

  type FileEntry = { name: string; path: string; is_dir: boolean; size: number; modified: string | null };

  let currentPath = $state("data");
  let entries = $state<FileEntry[]>([]);
  let selectedFolder = $state<string | null>(null);

  async function loadDirectory(path: string) {
    try {
      const all: FileEntry[] = await invoke("list_directory", { path });
      // Only show directories for folder picker
      entries = all.filter(e => e.is_dir).sort((a,b) => a.name.localeCompare(b.name));
      currentPath = path;
      selectedFolder = null; // reset selection when navigating
    } catch (e) {
      console.error(e);
      alert("Cannot read directory: " + e);
    }
  }

  onMount(() => {
    if (isOpen) {
      loadDirectory("data");
    }
  });

  $effect(() => {
    if (isOpen && currentPath === "data" && entries.length === 0) {
       loadDirectory("data");
    }
  });

  function handleFolderClick(entry: FileEntry) {
    selectedFolder = entry.path;
  }

  function handleFolderDoubleClick(entry: FileEntry) {
    loadDirectory(entry.path);
  }

  function goUp() {
    if (currentPath === "" || currentPath === "data") return;
    let parts = currentPath.replace(/\\/g, '/').split('/');
    if (parts.length > 0) {
      parts.pop();
      loadDirectory(parts.join('/'));
    }
  }

  function confirm() {
    if (selectedFolder) {
      onSelect(selectedFolder);
      isOpen = false;
    } else {
      // If none selected, assume they want to open the currentPath
      onSelect(currentPath);
      isOpen = false;
    }
  }

  function cancel() {
    isOpen = false;
  }
</script>

{#if isOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" onclick={cancel}>
    <div 
      class="bg-[#1e1e1e] w-[600px] h-[450px] rounded-xl border border-white/10 shadow-2xl flex flex-col overflow-hidden" 
      onclick={(e) => e.stopPropagation()}
    >
      <!-- Header -->
      <div class="flex items-center justify-between px-4 py-3 border-b border-white/10 bg-white/5">
        <h3 class="text-white/80 font-medium">Open Folder</h3>
        <button onclick={cancel} aria-label="Close" class="text-white/40 hover:text-white transition-colors">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path></svg>
        </button>
      </div>

      <!-- Controls & Path -->
      <div class="flex items-center gap-3 px-4 py-2 bg-black/20 border-b border-white/5">
        <button 
          onclick={goUp} 
          aria-label="Go Up Directory"
          disabled={currentPath === "" || currentPath === "data"}
          class="p-1.5 rounded hover:bg-white/10 text-white/70 disabled:opacity-30 disabled:hover:bg-transparent transition-colors"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18"></path></svg>
        </button>
        <div class="flex items-center text-sm font-mono text-white/60 bg-black/40 px-3 py-1.5 rounded-lg flex-1 overflow-x-auto">
          {currentPath || "/"}
        </div>
      </div>

      <!-- Folder List -->
      <div class="flex-1 overflow-y-auto p-2">
        <div class="grid grid-cols-4 gap-2">
          {#each entries as entry}
             <!-- svelte-ignore a11y_click_events_have_key_events -->
             <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div 
              class="flex flex-col items-center gap-2 p-3 rounded-xl cursor-pointer transition-colors border {selectedFolder === entry.path ? 'bg-indigo-500/20 border-indigo-500/50' : 'border-transparent hover:bg-white/5'}"
              onclick={() => handleFolderClick(entry)}
              ondblclick={() => handleFolderDoubleClick(entry)}
            >
              <svg class="w-10 h-10 text-blue-400 drop-shadow-md" fill="currentColor" viewBox="0 0 20 20"><path d="M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z"></path></svg>
              <span class="text-xs text-white/80 font-medium truncate w-full text-center">{entry.name}</span>
            </div>
          {/each}
          {#if entries.length === 0}
            <div class="col-span-4 py-12 flex flex-col items-center justify-center text-white/30 italic text-sm">
              <svg class="w-12 h-12 mb-3 opacity-20" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z"></path></svg>
              Empty directory
            </div>
          {/if}
        </div>
      </div>

      <!-- Footer -->
      <div class="px-5 py-3 border-t border-white/10 bg-white/5 flex items-center justify-between">
        <div class="text-xs text-white/40">
           {selectedFolder ? selectedFolder.split(/[/\\]/).pop() : "Current Directory"}
        </div>
        <div class="flex gap-3">
          <button onclick={cancel} class="px-4 py-2 rounded-lg text-sm text-white/70 hover:text-white hover:bg-white/10 transition-colors">
            Cancel
          </button>
          <button onclick={confirm} class="px-5 py-2 rounded-lg text-sm font-medium bg-indigo-500 hover:bg-indigo-600 text-white transition-colors shadow-lg">
            Open
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
