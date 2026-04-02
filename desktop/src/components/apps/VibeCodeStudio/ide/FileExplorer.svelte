<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  let { onSelectFile, currentFile, onProjectChange, basePath = "data" } = $props<{
    onSelectFile: (path: string, name: string) => void;
    currentFile: string | null;
    onProjectChange?: (selected: boolean) => void;
    basePath?: string;
  }>();

  type FileEntry = {
    name: string;
    path: string;
    is_dir: boolean;
    size: number;
    modified: string | null;
  };

  type TreeFolder = {
    entries: FileEntry[];
    expanded: boolean;
  };

  let treeState = $state<Record<string, TreeFolder>>({});
  let selectedItem = $state<FileEntry | null>(null);
  let projectRoot = $state<string | null>(null);

  async function loadDirectory(path: string) {
    try {
      const items = await invoke<FileEntry[]>("list_directory", { path });
      if (!treeState[path]) {
        treeState[path] = { entries: items, expanded: true };
      } else {
        treeState[path].entries = items;
      }
    } catch (e) {
      console.error(e);
    }
  }

  async function initProject(path: string) {
    projectRoot = path;
    selectedItem = null;
    await loadDirectory(path);
  }

  async function toggleDirectory(entry: FileEntry) {
    selectedItem = entry;
    const p = entry.path;
    if (!treeState[p]) {
      treeState[p] = { entries: [], expanded: true };
      await loadDirectory(p);
    } else {
      treeState[p].expanded = !treeState[p].expanded;
      if (treeState[p].expanded) {
        await loadDirectory(p); // Refresh on expand
      }
    }
  }

  function selectFile(entry: FileEntry) {
    selectedItem = entry;
    onSelectFile(entry.path, entry.name);
  }

  function closeProject() {
    projectRoot = null;
    selectedItem = null;
    treeState = {};
    onProjectChange?.(false);
  }

  onMount(() => {
    initProject(basePath);
  });

  $effect(() => {
    if (basePath && basePath !== projectRoot) {
      initProject(basePath);
    }
  });

  // --- Context Menu & Creation State ---
  let showNewInput = $state<"file" | "folder" | null>(null);
  let newName = $state("");
  
  let ctxMenu = $state<{x: number, y: number, path: string, name: string, isDir: boolean} | null>(null);
  let renameTarget = $state<{path: string, originalName: string, isDir: boolean} | null>(null);
  let renameValue = $state("");

  function getTargetCreationDir() {
    if (!selectedItem) return projectRoot;
    if (selectedItem.is_dir) return selectedItem.path;
    const parts = selectedItem.path.split(/[/\\]/);
    parts.pop();
    return parts.join('/') || projectRoot;
  }

  function handleContextMenu(e: MouseEvent, target: FileEntry | null) {
    e.preventDefault();
    e.stopPropagation();
    if (target) {
      selectedItem = target;
      ctxMenu = { x: e.clientX, y: e.clientY, path: target.path, name: target.name, isDir: target.is_dir };
    } else {
      // Background click
      selectedItem = null;
      ctxMenu = { x: e.clientX, y: e.clientY, path: projectRoot || "", name: "", isDir: true };
    }
  }

  function closeContextMenu() {
    ctxMenu = null;
  }

  async function handleDelete(path: string) {
    if (confirm(`Are you sure you want to delete '${path.split(/[/\\]/).pop()}'?`)) {
       try {
          await invoke("delete_entry", { path });
          // Bruteforce refresh the parent folder
          const parts = path.split(/[/\\]/);
          parts.pop();
          const parent = parts.join('/') || projectRoot;
          if (parent) await loadDirectory(parent);
       } catch (err) { alert("Failed to delete: " + err); }
    }
  }

  async function handleRenameSubmit(e: KeyboardEvent) {
    if (e.key === "Escape") {
      renameTarget = null;
      renameValue = "";
      return;
    }
    if (e.key === "Enter") {
      e.stopPropagation();
      e.preventDefault();
      
      const val = renameValue.trim();
      if (!val || !renameTarget) {
        renameTarget = null;
        return;
      }
      
      const parts = renameTarget.path.split(/[/\\]/);
      parts.pop();
      const parent = parts.join('/') || projectRoot;
      const newPath = (parent ? parent + "/" : "") + val;
      const oldPath = renameTarget.path;
      
      renameTarget = null;
      renameValue = "";
      
      if (oldPath !== newPath) {
        try {
          await invoke("rename_entry", { oldPath, newPath });
          if (parent) await loadDirectory(parent);
        } catch (err) {
          alert("Rename failed: " + err);
        }
      }
    }
  }

  async function handleCreate(e: KeyboardEvent) {
    if (e.key === "Escape") {
      showNewInput = null;
      newName = "";
      return;
    }
    if (e.key === "Enter") {
      e.stopPropagation();
      e.preventDefault();
      
      const localName = newName.trim();
      if (!localName) {
        showNewInput = null;
        return;
      }
      
      const parentDir = getTargetCreationDir();
      if (!parentDir) return;
      
      const targetPath = parentDir + "/" + localName;
      const type = showNewInput;
      
      showNewInput = null;
      newName = "";
      
      try {
         if (type === "folder") {
            await invoke("create_folder", { path: targetPath });
         } else {
            await invoke("write_file_content", { path: targetPath, content: "" });
            onSelectFile(targetPath, localName);
         }
         
         // Ensure parent is expanded so we can see the new item
         if (!treeState[parentDir]) {
           treeState[parentDir] = { entries: [], expanded: true };
         } else {
           treeState[parentDir].expanded = true;
         }
         await loadDirectory(parentDir);
      } catch (err) {
         alert("Failed to create: " + err);
      }
    }
  }
  
  // Start New File/Folder
  function triggerNew(type: "file" | "folder") {
    showNewInput = type;
    const targetDir = getTargetCreationDir();
    if (targetDir && treeState[targetDir]) {
      treeState[targetDir].expanded = true;
    }
  }

  // Focus without scrolling parent containers (prevents top bar from being pushed off-screen)
  function focusNoScroll(node: HTMLInputElement) {
    node.focus({ preventScroll: true });
  }
</script>

<svelte:window onclick={closeContextMenu} oncontextmenu={(e) => { 
  if (!e.defaultPrevented) closeContextMenu(); 
}} />

<!-- Recursive tree snippet -->
{#snippet renderTree(folderPath: string, depth: number)}
  {#if treeState[folderPath] && treeState[folderPath].expanded}
    
    <!-- Render the input box if we are creating right here -->
    {#if showNewInput && getTargetCreationDir() === folderPath}
      <div 
        class="flex items-center gap-2 py-1 bg-indigo-500/10 border-y border-indigo-500/20"
        style="padding-left: {depth * 12 + 16}px; padding-right: 16px;"
      >
        {#if showNewInput === 'folder'}
          <svg class="w-[14px] h-[14px] text-yellow-500/80 shrink-0" fill="currentColor" viewBox="0 0 20 20"><path d="M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z"></path></svg>
        {:else}
          <svg class="w-[14px] h-[14px] text-blue-400 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"></path></svg>
        {/if}
        <input
          use:focusNoScroll
          bind:value={newName}
          onkeydown={handleCreate}
          onblur={() => { showNewInput = null; newName = ''; }}
          class="bg-black/60 border border-indigo-500/50 rounded px-1 text-xs text-white focus:outline-none focus:border-indigo-400 flex-1 min-w-0 placeholder-white/30"
          placeholder={showNewInput === 'folder' ? 'Folder Name' : 'File Name'}
        />
      </div>
    {/if}

    <!-- Render children -->
    {#each treeState[folderPath].entries as entry}
      {#if entry.is_dir}
        <!-- DIRECTORY -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div 
          class="flex items-center py-1 cursor-pointer transition-colors group {selectedItem?.path === entry.path ? 'bg-indigo-500/20 text-indigo-100' : 'text-white/70 hover:bg-white/5'}"
          style="padding-left: {depth * 12 + 6}px; padding-right: 16px;"
          onclick={(e) => { e.stopPropagation(); toggleDirectory(entry); }}
          oncontextmenu={(e) => handleContextMenu(e, entry)}
        >
          <!-- Chevron -->
          <div class="w-4 h-4 flex items-center justify-center shrink-0 opacity-60 group-hover:opacity-100 transition-opacity">
            <svg class="w-2.5 h-2.5 transition-transform {treeState[entry.path]?.expanded ? 'rotate-90' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"></path></svg>
          </div>
          <!-- Icon -->
          <svg class="w-3.5 h-3.5 text-yellow-500/80 mr-1.5 shrink-0" fill="currentColor" viewBox="0 0 20 20"><path d="M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z"></path></svg>
          
          {#if renameTarget?.path === entry.path}
            <input
              use:focusNoScroll
              bind:value={renameValue}
              onkeydown={handleRenameSubmit}
              onblur={() => { renameTarget = null; renameValue = ''; }}
              class="bg-black/80 border border-indigo-500 rounded px-1 text-xs text-white focus:outline-none flex-1 min-w-0"
              onclick={(e) => e.stopPropagation()}
            />
          {:else}
            <span class="truncate">{entry.name}</span>
          {/if}
        </div>
        <!-- Recurse -->
        {@render renderTree(entry.path, depth + 1)}
      {:else}
        <!-- FILE -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div 
          class="flex items-center py-1 cursor-pointer transition-colors group {currentFile === entry.path ? 'bg-indigo-500/20 text-indigo-200 border-l-2 border-indigo-400' : (selectedItem?.path === entry.path ? 'bg-white/10 text-white' : 'text-white/60 hover:bg-white/5 border-l-2 border-transparent')}"
          style="padding-left: {depth * 12 + 22}px; padding-right: 16px;"
          onclick={(e) => { e.stopPropagation(); selectFile(entry); }}
          oncontextmenu={(e) => handleContextMenu(e, entry)}
        >
          <!-- Icon -->
          <svg class="w-3.5 h-3.5 text-blue-400 mr-1.5 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"></path></svg>
          
          {#if renameTarget?.path === entry.path}
            <input
              use:focusNoScroll
              bind:value={renameValue}
              onkeydown={handleRenameSubmit}
              onblur={() => { renameTarget = null; renameValue = ''; }}
              class="bg-black/80 border border-indigo-500 rounded px-1 text-xs text-white focus:outline-none flex-1 min-w-0"
              onclick={(e) => e.stopPropagation()}
            />
          {:else}
            <span class="truncate">{entry.name}</span>
          {/if}
        </div>
      {/if}
    {/each}
    {#if treeState[folderPath].entries.length === 0 && (!showNewInput || getTargetCreationDir() !== folderPath)}
      <div 
        class="py-1 text-white/30 italic text-[11px]"
        style="padding-left: {depth * 12 + 22}px;"
      >Empty</div>
    {/if}
  {/if}
{/snippet}

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div 
  class="flex flex-col h-full bg-black/40 text-[13px] font-sans w-64 border-r border-white/10 shrink-0 select-none overflow-y-auto relative"
  oncontextmenu={(e) => {
    if (!e.defaultPrevented && projectRoot) {
      handleContextMenu(e, null);
    }
  }}
  onclick={(e) => {
    if (!e.defaultPrevented) selectedItem = null;
  }}
>
  {#if projectRoot}
    <!-- Activity Top Bar -->
    <div class="px-4 py-2 text-xs font-semibold text-white/50 tracking-wider sticky top-0 bg-black/60 backdrop-blur z-20 flex items-center justify-between border-b border-white/5">
      <span class="truncate uppercase max-w-[100px]" title={projectRoot}>{projectRoot.split(/[/\\]/).pop() || 'Explorer'}</span>
      <div class="flex items-center gap-1.5 hover:text-white/60">
        <!-- New File -->
        <button onclick={() => triggerNew('file')} class="text-white/40 hover:text-white transition-colors p-0.5 rounded hover:bg-white/10" title="New File">
          <svg class="w-[15px] h-[15px]" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 13h6m-3-3v6m5 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path></svg>
        </button>
        <!-- New Folder -->
        <button onclick={() => triggerNew('folder')} class="text-white/40 hover:text-white transition-colors p-0.5 rounded hover:bg-white/10" title="New Folder">
          <svg class="w-[15px] h-[15px]" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 13h6m-3-3v6m-9 1V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v10a2 2 0 01-2 2H5a2 2 0 01-2-2z"></path></svg>
        </button>
        <!-- Refresh -->
        <button onclick={() => loadDirectory(projectRoot!)} class="text-indigo-400 hover:text-indigo-300 transition-colors p-0.5 rounded hover:bg-indigo-500/20" title="Refresh">
           <svg class="w-[14px] h-[14px]" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path></svg>
        </button>
        <!-- Close Project -->
        <button onclick={closeProject} class="text-white/40 hover:text-rose-400 transition-colors p-0.5 rounded hover:bg-rose-500/20" title="Close Workspace">
          <svg class="w-[15px] h-[15px]" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path></svg>
        </button>
      </div>
    </div>
    
    <!-- Render Root Tree -->
    <div class="flex-1 py-1 pb-10">
      {@render renderTree(projectRoot, 0)}
      
      <!-- Show input at the absolute root if target is strictly root -->
      {#if showNewInput && getTargetCreationDir() === projectRoot && !(treeState[projectRoot]?.expanded)}
        <div class="flex items-center gap-2 py-1 px-4 bg-indigo-500/10 border-y border-indigo-500/20">
          {#if showNewInput === 'folder'}
            <svg class="w-[14px] h-[14px] text-yellow-500/80 shrink-0" fill="currentColor" viewBox="0 0 20 20"><path d="M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z"></path></svg>
          {:else}
            <svg class="w-[14px] h-[14px] text-blue-400 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"></path></svg>
          {/if}
          <input
            use:focusNoScroll
            bind:value={newName}
            onkeydown={handleCreate}
            onblur={() => { showNewInput = null; newName = ''; }}
            class="bg-black/60 border border-indigo-500/50 rounded px-1 text-xs text-white focus:outline-none focus:border-indigo-400 flex-1 min-w-0 placeholder-white/30"
            placeholder={showNewInput === 'folder' ? 'Folder Name' : 'File Name'}
          />
        </div>
      {/if}
    </div>
  {:else}
    <div class="flex-1 flex items-center justify-center text-white/30 text-xs italic">
      No folder opened
    </div>
  {/if}
</div>

{#if ctxMenu}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div 
    class="fixed z-[100] w-48 bg-[#252526] border border-black/50 shadow-2xl rounded-md py-1 text-xs text-white/80 font-sans"
    style="left: {Math.min(ctxMenu.x, window.innerWidth - 192)}px; top: {Math.min(ctxMenu.y, window.innerHeight - 150)}px;"
    onclick={(e) => e.stopPropagation()}
  >
    <!-- Background right-click / Empty space -->
    {#if !ctxMenu.name}
      <button onclick={() => { triggerNew('file'); closeContextMenu(); }} class="w-full text-left px-3 py-1.5 hover:bg-indigo-500 hover:text-white transition-colors flex items-center gap-2">
        <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 13h6m-3-3v6m5 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path></svg>
        New File
      </button>
      <button onclick={() => { triggerNew('folder'); closeContextMenu(); }} class="w-full text-left px-3 py-1.5 hover:bg-indigo-500 hover:text-white transition-colors flex items-center gap-2 mb-1 border-b border-white/5">
        <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 13h6m-3-3v6m-9 1V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v10a2 2 0 01-2 2H5a2 2 0 01-2-2z"></path></svg>
        New Folder
      </button>
      <div class="px-3 py-1 text-white/30 italic truncate">{getTargetCreationDir()}</div>
    {:else}
      <!-- Specific Item right-click -->
      <div class="px-3 py-1.5 text-white/40 font-semibold truncate border-b border-white/5 mb-1 text-[11px] uppercase tracking-wider">{ctxMenu.name}</div>
      <button onclick={() => { triggerNew('file'); closeContextMenu(); }} class="w-full text-left px-3 py-1.5 hover:bg-indigo-500 hover:text-white transition-colors flex items-center gap-2">
        <svg class="w-3.5 h-3.5 opacity-70" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 13h6m-3-3v6m5 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path></svg>
        New File
      </button>
      <button onclick={() => { triggerNew('folder'); closeContextMenu(); }} class="w-full text-left px-3 py-1.5 hover:bg-indigo-500 hover:text-white transition-colors flex items-center gap-2 mb-1 border-b border-white/5">
        <svg class="w-3.5 h-3.5 opacity-70" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 13h6m-3-3v6m-9 1V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v10a2 2 0 01-2 2H5a2 2 0 01-2-2z"></path></svg>
        New Folder
      </button>

      <button 
        onclick={() => { 
          navigator.clipboard.writeText(ctxMenu!.path);
          closeContextMenu(); 
        }} 
        class="w-full text-left px-3 py-1.5 hover:bg-indigo-500 hover:text-white transition-colors flex items-center gap-2"
      >
        <svg class="w-3.5 h-3.5 opacity-70" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3"></path></svg>
        Copy Path
      </button>

      <button 
        onclick={() => { 
          renameTarget = { path: ctxMenu!.path, originalName: ctxMenu!.name, isDir: ctxMenu!.isDir }; 
          renameValue = ctxMenu!.name; 
          closeContextMenu(); 
        }} 
        class="w-full text-left px-3 py-1.5 hover:bg-indigo-500 hover:text-white transition-colors flex items-center gap-2"
      >
        <svg class="w-3.5 h-3.5 opacity-70" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"></path></svg>
        Rename...
      </button>
      <button 
        onclick={() => { handleDelete(ctxMenu!.path); closeContextMenu(); }} 
        class="w-full text-left px-3 py-1.5 hover:bg-rose-500 hover:text-white transition-colors flex items-center gap-2 mt-1 border-t border-white/5"
      >
        <svg class="w-3.5 h-3.5 opacity-70" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path></svg>
        Delete
      </button>
    {/if}
  </div>
{/if}
