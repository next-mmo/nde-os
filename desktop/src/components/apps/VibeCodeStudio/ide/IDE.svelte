<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import FileExplorer from './FileExplorer.svelte';
  import SourceControl from './SourceControl.svelte';
  import CodeEditor from './CodeEditor.svelte';

  let { activeFilePath = $bindable(null), fileContent = $bindable("") } = $props<{
    activeFilePath?: string | null;
    fileContent?: string;
  }>();

  let activeSidebar = $state<"explorer" | "scm">("explorer");

  let activeFileName = $state<string>("");
  let isSaving = $state(false);

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
      case 'svelte': return 'html'; // Monaco doesn't have svelte natively without extension, fallback to html or use specific
      default: return 'plaintext';
    }
  });

  async function openFile(path: string, name: string) {
    try {
      const content = await invoke<string>("read_file_content", { path });
      activeFilePath = path;
      activeFileName = name;
      fileContent = content;
    } catch (e) {
      console.error(e);
      alert("Failed to read file: " + e);
    }
  }

  async function saveFile() {
    if (!activeFilePath) return;
    try {
      isSaving = true;
      await invoke("write_file_content", { path: activeFilePath, content: fileContent });
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

<svelte:window onkeydown={handleKeyDown} />

<div class="absolute inset-0 flex h-full overflow-hidden text-left bg-black">
  <!-- Activity Bar -->
  <div class="w-12 bg-[#181818] border-r border-white/5 shrink-0 flex flex-col items-center py-4 gap-4 z-20">
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div 
      class="w-10 h-10 rounded-xl flex items-center justify-center cursor-pointer transition-all {activeSidebar === 'explorer' ? 'bg-indigo-500/20 text-indigo-400' : 'text-white/40 hover:text-white'}"
      onclick={() => activeSidebar = 'explorer'}
      title="Explorer"
    >
      <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"></path></svg>
    </div>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div 
      class="w-10 h-10 rounded-xl flex items-center justify-center cursor-pointer transition-all {activeSidebar === 'scm' ? 'bg-indigo-500/20 text-indigo-400' : 'text-white/40 hover:text-white'}"
      onclick={() => activeSidebar = 'scm'}
      title="Source Control"
    >
      <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7v8a2 2 0 002 2h6M8 7V5a2 2 0 012-2h4.586a1 1 0 01.707.293l4.414 4.414a1 1 0 01.293.707V15a2 2 0 01-2 2h-2M8 7H6a2 2 0 00-2 2v10a2 2 0 002 2h8a2 2 0 002-2v-2"></path></svg>
    </div>
  </div>

  <!-- Sidebar -->
  {#if activeSidebar === 'explorer'}
    <FileExplorer onSelectFile={openFile} currentFile={activeFilePath} />
  {:else}
    <SourceControl onSelectFile={openFile} />
  {/if}

  <!-- Editor Area -->
  <div class="flex-1 flex flex-col min-w-0 bg-[#1e1e1e]">
    {#if activeFilePath}
      <!-- Editor Header -->
      <div class="h-10 flex border-b border-white/5 bg-[#252526] shrink-0 items-center justify-between pr-4 select-none">
        <div class="flex h-full">
          <div class="px-4 flex items-center gap-2 border-x border-white/5 bg-[#1e1e1e] text-indigo-300 text-sm font-mono min-w-32 border-t-2 border-t-indigo-500 shadow-[0_-1px_0_0_rgba(255,255,255,0.05)_inset]">
            <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"></path></svg>
            {activeFileName}
          </div>
        </div>
        <button 
          class="text-xs px-3 py-1.5 rounded bg-indigo-500/20 text-indigo-300 hover:bg-indigo-500/30 hover:text-indigo-200 transition-colors flex items-center gap-1.5 font-medium border border-indigo-500/30"
          onclick={saveFile}
          disabled={isSaving}
        >
          {#if isSaving}
            ⏳ Saving...
          {:else}
            <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4"></path></svg>
            Save
          {/if}
        </button>
      </div>

      <!-- Monaco Editor -->
      <div class="flex-1 relative">
        <CodeEditor 
          content={fileContent} 
          language={activeLanguage()} 
          onChange={(val) => fileContent = val} 
        />
      </div>
    {:else}
      <div class="flex-1 flex flex-col items-center justify-center text-white/30 gap-6">
        <div class="w-24 h-24 rounded-3xl bg-white/5 flex items-center justify-center border border-white/10 shadow-2xl">
          <svg class="w-12 h-12 text-white/20" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"></path>
          </svg>
        </div>
        <div class="text-3xl text-white/50 font-light tracking-wide">OpenCode IDE</div>
        <div class="flex flex-col items-center gap-3 mt-4 text-sm text-white/40">
          <p>Select a file from the explorer to start editing</p>
          <div class="flex gap-4 mt-4">
             <button onclick={() => activeSidebar = 'explorer'} class="px-4 py-2 bg-indigo-500/10 hover:bg-indigo-500/20 border border-indigo-500/20 text-indigo-300 rounded transition-colors flex items-center gap-2 shadow-lg">
                 <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"></path></svg>
                 Choose Project
             </button>
             <button onclick={() => activeSidebar = 'scm'} class="px-4 py-2 bg-white/5 hover:bg-white/10 border border-white/10 text-white/60 hover:text-white/80 rounded transition-colors flex items-center gap-2 shadow-lg">
                 <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7v8a2 2 0 002 2h6M8 7V5a2 2 0 012-2h4.586a1 1 0 01.707.293l4.414 4.414a1 1 0 01.293.707V15a2 2 0 01-2 2h-2M8 7H6a2 2 0 00-2 2v10a2 2 0 002 2h8a2 2 0 002-2v-2"></path></svg>
                 Open Git
             </button>
          </div>
          <div class="flex gap-4 mt-6 opacity-30">
            <div class="flex items-center gap-2">
              <kbd class="px-2 py-1 bg-white/5 rounded border border-white/10 text-xs font-mono font-bold text-white/50">⌘S</kbd>
              <span>to save</span>
            </div>
          </div>
        </div>
      </div>
    {/if}
  </div>
</div>
