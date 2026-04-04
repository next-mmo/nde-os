<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  import FileExplorer from './FileExplorer.svelte';
  import FolderPicker from './FolderPicker.svelte';
  import SourceControl from './SourceControl.svelte';
  import CodeEditor from './CodeEditor.svelte';
  import TerminalPanel from './TerminalPanel.svelte';
  import MarkdownPreview from './MarkdownPreview.svelte';

  let { activeFilePath = $bindable(null), fileContent = $bindable(""), onAddToChat } = $props<{
    activeFilePath?: string | null;
    fileContent?: string;
    onAddToChat?: (path: string, name: string) => void;
  }>();

  let activeSidebar = $state<"explorer" | "scm">("explorer");
  let showTerminal = $state(false);
  let terminalHeight = $state(250);
  let projectSelected = $state(false);
  let showSidebar = $state(false);
  let isDraggingTerminal = $state(false);

  type FileEntry = { name: string; path: string; is_dir: boolean; modified: string | null; };
  let recentProjects = $state<FileEntry[]>([]);
  let selectedProjectPath = $state<string>("data");
  let showNewProjectInput = $state(false);
  let newProjectName = $state("");
  let showFolderPicker = $state(false);

  let activeFileName = $state<string>("");
  let isSaving = $state(false);
  let savedContent = $state<string>(""); // tracks last persisted content

  let isDirty = $derived(activeFilePath !== null && fileContent !== savedContent);
  let isMarkdown = $derived(activeFileName.toLowerCase().endsWith('.md'));

  // md view mode: 'edit' | 'preview' | 'split' — resets when switching away from .md
  let mdViewMode = $state<'edit' | 'preview' | 'split'>('edit');
  $effect(() => { if (!isMarkdown) mdViewMode = 'edit'; });

  // Auto-restore project and sidebars if a file is opened programmatically
  $effect(() => {
    if (activeFilePath && !projectSelected) {
      projectSelected = true;
      showSidebar = true;
      // Auto scope to project folder (e.g., "data/my-app/...")
      const parts = activeFilePath.split(/[\/\\]/);
      if (parts[0] === 'data' && parts.length >= 2) {
        selectedProjectPath = parts.slice(0, 2).join('/');
      } else {
        selectedProjectPath = "data";
      }
    }
  });

  // Derive language from extension
  let activeLanguage = $derived(() => {
    if (!activeFileName) return "plaintext";
    const ext = activeFileName.split('.').pop()?.toLowerCase();
    switch (ext) {
      case 'ts': case 'tsx': return 'typescript';
      case 'js': case 'jsx': return 'javascript';
      case 'json': return 'json';
      case 'rs': return 'rust';
      case 'css': return 'css';
      case 'html': return 'html';
      case 'md': return 'markdown';
      case 'svelte': return 'html';
      case 'py': return 'python';
      case 'toml': return 'ini';
      case 'yaml': case 'yml': return 'yaml';
      case 'sh': case 'bash': return 'shell';
      default: return 'plaintext';
    }
  });

  async function loadRecentProjects() {
    try {
      const items: FileEntry[] = await invoke("list_directory", { path: "data" });
      recentProjects = items.filter(i => i.is_dir).sort((a,b) => {
          return (b.modified || "").localeCompare(a.modified || "");
      });
    } catch (e) {
      console.error("Failed to load projects", e);
    }
  }

  onMount(() => {
    loadRecentProjects();
  });

  async function openProject(path: string) {
    selectedProjectPath = path;
    projectSelected = true;
    showSidebar = true;
    activeSidebar = 'explorer';
  }

  async function confirmNewProject() {
     if (!newProjectName || !newProjectName.trim()) {
       showNewProjectInput = false;
       return;
     }
     const path = `data/${newProjectName.trim()}`;
     try {
       await invoke("create_folder", { path });
       await loadRecentProjects();
       await openProject(path);
       showNewProjectInput = false;
       newProjectName = "";
     } catch (e) {
       alert("Create failed: " + e);
     }
  }

  async function openFile(path: string, name: string) {
    // If current file is dirty, ask before switching
    if (isDirty) {
      const choice = confirm(`Save changes to "${activeFileName}" before opening "${name}"?`);
      if (choice) await saveFile();
    }
    try {
      const content = await invoke<string>("read_file_content", { path });
      activeFilePath = path;
      activeFileName = name;
      fileContent = content;
      savedContent = content;
    } catch (e) {
      console.error(e);
      alert("Failed to read file: " + e);
    }
  }

  async function discardChanges() {
    if (!activeFilePath) return;
    try {
      const content = await invoke<string>("read_file_content", { path: activeFilePath });
      fileContent = content;
      savedContent = content;
    } catch (e) {
      console.error(e);
    }
  }

  function closeTab() {
    if (isDirty) {
      const choice = confirm(`Save changes to "${activeFileName}" before closing?`);
      if (choice) {
        saveFile().then(() => {
          activeFilePath = null;
          activeFileName = "";
          fileContent = "";
          savedContent = "";
        });
        return;
      }
    }
    activeFilePath = null;
    activeFileName = "";
    fileContent = "";
    savedContent = "";
  }

  async function saveFile() {
    if (!activeFilePath) return;
    try {
      isSaving = true;
      await invoke("write_file_content", { path: activeFilePath, content: fileContent });
      savedContent = fileContent; // mark clean
    } catch (e) {
      console.error(e);
      alert("Failed to save file: " + e);
    } finally {
      isSaving = false;
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === 's') {
      if (activeFilePath) {
        e.preventDefault();
        saveFile();
      }
    }
  }
</script>

<svelte:window 
  onkeydown={handleKeyDown} 
  onmouseup={() => isDraggingTerminal = false} 
  onmousemove={(e) => {
    if (isDraggingTerminal) {
      const newHeight = terminalHeight - e.movementY;
      terminalHeight = Math.max(100, Math.min(newHeight, window.innerHeight * 0.8));
    }
  }} 
/>

<div class="absolute inset-0 flex h-full overflow-hidden text-left bg-black">
  <!-- Activity Bar — always visible, like VS Code -->
  <div class="w-12 bg-[#181818] border-r border-white/5 shrink-0 flex flex-col items-center py-4 gap-4 z-20">
    <!-- Explorer -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div 
      class="w-10 h-10 rounded-xl flex items-center justify-center cursor-pointer transition-all {activeSidebar === 'explorer' && showSidebar && projectSelected ? 'bg-indigo-500/20 text-indigo-400' : 'text-white/40 hover:text-white'}"
      onclick={() => {
        if (!projectSelected) return;
        activeSidebar = 'explorer';
        showSidebar = activeSidebar === 'explorer' ? !showSidebar : true;
      }}
      title={projectSelected ? "Explorer" : "Open a project first"}
    >
      <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"></path></svg>
    </div>
    <!-- Source Control -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div 
      class="w-10 h-10 rounded-xl flex items-center justify-center cursor-pointer transition-all {activeSidebar === 'scm' && showSidebar && projectSelected ? 'bg-indigo-500/20 text-indigo-400' : 'text-white/40 hover:text-white'}"
      onclick={() => {
        if (!projectSelected) return;
        activeSidebar = 'scm';
        showSidebar = activeSidebar === 'scm' ? !showSidebar : true;
      }}
      title={projectSelected ? "Source Control" : "Open a project first"}
    >
      <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7v8a2 2 0 002 2h6M8 7V5a2 2 0 012-2h4.586a1 1 0 01.707.293l4.414 4.414a1 1 0 01.293.707V15a2 2 0 01-2 2h-2M8 7H6a2 2 0 00-2 2v10a2 2 0 002 2h8a2 2 0 002-2v-2"></path></svg>
    </div>

    <div class="flex-1"></div>

    <!-- Terminal toggle — always available -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div 
      class="w-10 h-10 rounded-xl flex items-center justify-center cursor-pointer transition-all {showTerminal ? 'bg-indigo-500/20 text-indigo-400' : 'text-white/40 hover:text-white'}"
      onclick={() => showTerminal = !showTerminal}
      title="Toggle Terminal (⌘`)"
    >
      <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"></path></svg>
    </div>
    <!-- Close Project — only shown when project is open -->
    {#if projectSelected}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div 
        class="w-10 h-10 rounded-xl flex items-center justify-center cursor-pointer transition-all text-white/40 hover:text-rose-400"
        onclick={() => { projectSelected = false; showSidebar = false; activeFilePath = null; fileContent = ''; savedContent = ''; }}
        title="Close Project"
      >
        <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path></svg>
      </div>
    {/if}
  </div>

  <!-- Sidebar — only when project is open -->
  {#if projectSelected && showSidebar}
    {#if activeSidebar === 'explorer'}
      <FileExplorer
        onSelectFile={openFile}
        currentFile={activeFilePath}
        basePath={selectedProjectPath}
        {onAddToChat}
        onProjectChange={(selected) => { if (!selected) { projectSelected = false; showSidebar = false; } }}
      />
    {:else}
      <SourceControl onSelectFile={openFile} />
    {/if}
  {/if}


  <!-- Editor Area -->
  <div class="flex-1 flex flex-col min-w-0 bg-[#1e1e1e]">
    {#if activeFilePath}
      <!-- VS Code-style Tab Bar -->
      <div class="flex border-b border-white/5 bg-[#252526] shrink-0 items-stretch select-none">
        <!-- Tab -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div class="flex items-center h-9 border-r border-white/5 bg-[#1e1e1e] border-t-2 border-t-indigo-500 px-3 gap-1.5 min-w-0 max-w-[220px] group relative">
          <!-- File icon -->
          <svg class="w-3.5 h-3.5 shrink-0 text-indigo-400" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"></path></svg>
          <!-- Filename -->
          <span class="text-indigo-200 text-xs font-mono truncate flex-1 mr-1">{activeFileName}</span>
          <!-- Unsaved dot OR close button -->
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="w-4 h-4 flex items-center justify-center shrink-0 relative">
            {#if isDirty}
              <!-- Dot (unsaved): hover reveals × -->
              <span
                class="w-2 h-2 rounded-full bg-white/70 group-hover:opacity-0 transition-opacity absolute"
                title="Unsaved changes"
              ></span>
              <span
                onclick={closeTab}
                class="text-white/50 hover:text-white text-[11px] leading-none opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer absolute select-none"
                title="Close (unsaved)"
              >✕</span>
            {:else}
              <!-- Clean: hover reveals × -->
              <span
                onclick={closeTab}
                class="text-white/30 hover:text-white text-[11px] leading-none opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer select-none"
                title="Close"
              >✕</span>
            {/if}
          </div>
        </div>

        <!-- Markdown view-mode toggle (only for .md files) -->
        {#if isMarkdown}
          <div class="flex items-center gap-0.5 ml-auto mr-2">
            <button
              onclick={() => mdViewMode = 'edit'}
              title="Edit"
              class="flex items-center gap-1 px-2 py-1 rounded text-[10px] transition-colors {mdViewMode === 'edit' ? 'bg-white/10 text-white' : 'text-white/40 hover:text-white hover:bg-white/5'}"
            >
              <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"></path></svg>
              Edit
            </button>
            <button
              onclick={() => mdViewMode = 'split'}
              title="Split"
              class="flex items-center gap-1 px-2 py-1 rounded text-[10px] transition-colors {mdViewMode === 'split' ? 'bg-white/10 text-white' : 'text-white/40 hover:text-white hover:bg-white/5'}"
            >
              <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 17V7m0 10a2 2 0 01-2 2H5a2 2 0 01-2-2V7a2 2 0 012-2h2a2 2 0 012 2m0 10a2 2 0 002 2h2a2 2 0 002-2M9 7a2 2 0 012-2h2a2 2 0 012 2m0 0v10"></path></svg>
              Split
            </button>
            <button
              onclick={() => mdViewMode = 'preview'}
              title="Preview"
              class="flex items-center gap-1 px-2 py-1 rounded text-[10px] transition-colors {mdViewMode === 'preview' ? 'bg-white/10 text-white' : 'text-white/40 hover:text-white hover:bg-white/5'}"
            >
              <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"></path></svg>
              Preview
            </button>
          </div>
        {/if}
      </div>

      <!-- Editor / Preview area: relative anchor, children are absolute -->
      <div class="flex-1 relative min-h-0 overflow-hidden">
        <!-- Edit pane -->
        {#if mdViewMode === 'edit'}
          <div class="absolute inset-0">
            <CodeEditor
              content={fileContent}
              language={activeLanguage()}
              onChange={(val) => fileContent = val}
            />
          </div>
        {:else if mdViewMode === 'split'}
          <!-- Left: editor -->
          <div class="absolute top-0 bottom-0 left-0 w-1/2 border-r border-white/10">
            <CodeEditor
              content={fileContent}
              language={activeLanguage()}
              onChange={(val) => fileContent = val}
            />
          </div>
          <!-- Right: preview -->
          <div class="absolute top-0 bottom-0 right-0 w-1/2">
            <MarkdownPreview content={fileContent} />
          </div>
        {:else if mdViewMode === 'preview'}
          <div class="absolute inset-0">
            <MarkdownPreview content={fileContent} />
          </div>
        {/if}
      </div>
    {:else}
      <div class="flex-1 flex flex-col items-center justify-center text-white/30 gap-6">
        <div class="w-24 h-24 rounded-3xl bg-white/5 flex items-center justify-center border border-white/10 shadow-2xl">
          <svg class="w-12 h-12 text-white/20" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"></path>
          </svg>
        </div>
        <div class="text-3xl text-white/50 font-light tracking-wide">NDE IDE</div>
        <div class="flex flex-col items-center gap-3 mt-4 text-sm text-white/40">
          {#if projectSelected}
            <p>Select a file from the explorer to start editing</p>
            <div class="flex gap-4 mt-4">
              <button onclick={() => { activeSidebar = 'explorer'; showSidebar = true; }} class="px-4 py-2 bg-indigo-500/10 hover:bg-indigo-500/20 border border-indigo-500/20 text-indigo-300 rounded-lg transition-colors flex items-center gap-2 shadow-lg">
                <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"></path></svg>
                Show Explorer
              </button>
              <button onclick={() => { activeSidebar = 'scm'; showSidebar = true; }} class="px-4 py-2 bg-white/5 hover:bg-white/10 border border-white/10 text-white/60 hover:text-white/80 rounded-lg transition-colors flex items-center gap-2 shadow-lg">
                <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7v8a2 2 0 002 2h6M8 7V5a2 2 0 012-2h4.586a1 1 0 01.707.293l4.414 4.414a1 1 0 01.293.707V15a2 2 0 01-2 2h-2M8 7H6a2 2 0 00-2 2v10a2 2 0 002 2h8a2 2 0 002-2v-2"></path></svg>
                Open Git
              </button>
            </div>
          {:else}
            <div class="flex flex-col items-center w-full max-w-md">
              <h3 class="text-white/60 mb-3 text-lg font-medium">Recent Projects</h3>
              <div class="flex flex-col gap-2 w-full max-h-48 overflow-y-auto mb-6 px-2 scrollbar-thin scrollbar-thumb-white/10 scrollbar-track-transparent">
                {#each recentProjects as proj}
                  <button onclick={() => openProject(proj.path)} class="text-left px-4 py-3 bg-white/5 hover:bg-white/10 border border-white/10 text-white/60 hover:text-white rounded-lg transition-all flex items-center justify-between shadow-sm group">
                    <div class="flex items-center gap-3">
                      <svg class="w-4 h-4 text-indigo-400 group-hover:text-indigo-300 transition-colors" fill="currentColor" viewBox="0 0 20 20"><path d="M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z"></path></svg>
                      <span class="truncate font-medium">{proj.name}</span>
                    </div>
                    <span class="text-xs text-white/30 truncate max-w-[120px]">{proj.modified ? proj.modified.split(" ")[0] : ""}</span>
                  </button>
                {/each}
                {#if recentProjects.length === 0}
                   <div class="text-white/30 italic text-[11px] text-center py-4 bg-black/20 rounded-lg border border-white/5">No projects found in workspace</div>
                {/if}
              </div>
              
              <div class="flex gap-4 w-full justify-center">
                {#if showNewProjectInput}
                  <div class="flex w-full box-border">
                    <!-- svelte-ignore a11y_autofocus -->
                    <input bind:value={newProjectName} placeholder="Project Name... (Enter to confirm)" class="flex-1 bg-black/60 border border-white/10 rounded-l-lg py-2.5 px-3 text-white placeholder-white/30 focus:outline-none focus:ring-1 focus:ring-indigo-500/50 text-sm" 
                      onkeydown={(e) => {
                        if (e.key === 'Enter') confirmNewProject();
                        if (e.key === 'Escape') { showNewProjectInput = false; newProjectName = ''; }
                      }} 
                      autofocus
                    />
                    <button onclick={confirmNewProject} class="px-4 bg-indigo-500 hover:bg-indigo-600 text-white transition-colors font-medium text-sm">Create</button>
                    <button onclick={() => { showNewProjectInput = false; newProjectName = ''; }} class="px-4 bg-white/5 hover:bg-white/10 text-white/60 rounded-r-lg border-y border-r border-white/10 transition-colors text-sm">Cancel</button>
                  </div>
                {:else}
                  <button onclick={() => { showNewProjectInput = true; newProjectName = ''; }} class="px-5 py-2.5 bg-indigo-500/20 hover:bg-indigo-500/30 border border-indigo-500/30 text-indigo-300 rounded-lg transition-all flex items-center gap-2.5 shadow-lg font-medium flex-1 justify-center">
                    <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path></svg>
                    New Project
                  </button>
                  <button onclick={() => showFolderPicker = true} class="px-5 py-2.5 bg-white/5 hover:bg-white/10 border border-white/10 text-white/60 hover:text-white/80 rounded-lg transition-all flex items-center gap-2.5 shadow-lg font-medium flex-1 justify-center">
                    <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"></path></svg>
                    Browse Sandbox
                  </button>
                {/if}
              </div>
            </div>
            <div class="mt-8 text-xs text-white/20 space-y-2">
              <div class="flex items-center gap-3">
                <kbd class="px-2 py-1 bg-white/5 rounded border border-white/10 font-mono font-bold text-white/40">⌘S</kbd>
                <span>Save file</span>
              </div>
              <div class="flex items-center gap-3">
                <kbd class="px-2 py-1 bg-white/5 rounded border border-white/10 font-mono font-bold text-white/40">⌘`</kbd>
                <span>Toggle terminal</span>
              </div>
            </div>
          {/if}
        </div>
      </div>
    {/if}

    {#if showTerminal}
      <!-- Resizer handle -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div 
        class="h-1 bg-black cursor-row-resize hover:bg-indigo-500/50 transition-colors z-50 group"
        onmousedown={() => isDraggingTerminal = true}
      >
        <div class="h-0.5 max-w-16 mx-auto bg-white/10 group-hover:bg-indigo-400 mt-px rounded-full"></div>
      </div>
      <!-- Terminal Panel -->
      <div 
        class="bg-[#1e1e1e] flex flex-col shrink-0 overflow-hidden" 
        style="height: {terminalHeight}px;"
      >
        <div class="flex items-center justify-between px-4 py-1.5 border-b border-black/50 text-[11px] uppercase tracking-wider text-white/50 bg-[#252526]">
          <span>Terminal</span>
          <button 
            onclick={() => showTerminal = false}
            class="hover:text-white hover:bg-white/10 rounded p-0.5 transition-colors"
          >
            ✕
          </button>
        </div>
        <div class="flex-1 relative overflow-hidden">
          <TerminalPanel projectPath={selectedProjectPath} />
        </div>
      </div>
    {/if}
  </div>
</div>

<FolderPicker bind:isOpen={showFolderPicker} onSelect={(path) => openProject(path)} />

