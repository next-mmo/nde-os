<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  let { onSelectFile, onRefresh } = $props<{
    onSelectFile: (path: string, statusText: string) => void;
    onRefresh?: () => void;
  }>();

  type GitFile = {
    status: string; // e.g. "M ", " M", "??", "A "
    path: string;
  };

  let files = $state<GitFile[]>([]);
  let commitMessage = $state("");
  let isCommitting = $state(false);

  export async function loadStatus() {
    try {
      const lines = await invoke<string[]>("git_status");
      files = lines.map(line => {
        const status = line.substring(0, 2);
        const path = line.substring(3).trim();
        return { status, path };
      });
    } catch (e) {
      console.error(e);
      files = []; // Not a git repo, or git failed
    }
  }

  async function handleCommit() {
    if (!commitMessage.trim()) return;
    try {
      isCommitting = true;
      // For MVP, just stage all changes before commit
      await invoke("git_add", { path: "." });
      await invoke("git_commit", { message: commitMessage });
      commitMessage = "";
      await loadStatus();
      if (onRefresh) onRefresh();
    } catch (e) {
      console.error(e);
      alert("Failed to commit: " + e);
    } finally {
      isCommitting = false;
    }
  }

  async function discardChange(path: string) {
    if (!confirm(`Are you sure you want to discard changes in ${path}?`)) return;
    try {
      await invoke("git_discard", { path });
      await loadStatus();
      if (onRefresh) onRefresh();
    } catch (e) {
      console.error(e);
      alert("Failed to discard: " + e);
    }
  }

  onMount(() => {
    loadStatus();
  });
</script>

<div class="flex flex-col h-full bg-[#21252b] text-sm font-sans w-64 border-r border-[#181a1f] shrink-0 select-none overflow-y-auto">
  <div class="px-4 py-2 text-xs font-semibold text-white/50 tracking-wider sticky top-0 bg-[#21252b] backdrop-blur z-10 flex items-center justify-between border-b border-[#181a1f]">
    <span class="uppercase">Source Control</span>
    <button onclick={loadStatus} class="text-white/40 hover:text-white transition-colors" title="Refresh">↻</button>
  </div>
  
  <!-- Commit box -->
  <div class="p-4 border-b border-white/5 space-y-2">
    <textarea 
      bind:value={commitMessage}
      placeholder="Message (⌘Enter to commit)"
      rows="2"
      disabled={isCommitting}
      class="w-full bg-black/60 border border-white/10 rounded-md py-1.5 px-2 text-white placeholder-white/30 focus:outline-none focus:ring-1 focus:ring-indigo-500/50 resize-none text-xs"
      onkeydown={(e) => {
        if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
          e.preventDefault();
          handleCommit();
        }
      }}
    ></textarea>
    <button 
      onclick={handleCommit}
      disabled={isCommitting || !commitMessage.trim()}
      class="w-full bg-indigo-500 hover:bg-indigo-600 disabled:opacity-50 disabled:hover:bg-indigo-500 text-white rounded text-xs py-1.5 transition-colors font-medium border border-indigo-400"
    >
      {isCommitting ? "Committing..." : "Commit"}
    </button>
  </div>

  <div class="flex-1 py-1">
    <div class="px-4 py-1 text-[10px] text-white/40 font-semibold uppercase tracking-wider mb-1 mt-1">Changes</div>
    {#each files as file}
      <div 
        class="flex items-center justify-between px-4 py-1.5 hover:bg-white/5 group cursor-pointer"
        onclick={() => onSelectFile(file.path, file.status)}
      >
        <div class="flex items-center gap-2 truncate">
          <span class="text-[10px] w-4 font-mono font-bold shrink-0
            {file.status.includes('M') ? 'text-blue-400' :
             file.status.includes('A') || file.status.includes('??') ? 'text-emerald-400' :
             file.status.includes('D') ? 'text-red-400' : 'text-white/40'}
          ">
            {file.status.trim() || 'M'}
          </span>
          <span class="truncate text-white/70 group-hover:text-white text-xs">{file.path.split('/').pop()}</span>
        </div>
        <div class="flex items-center opacity-0 group-hover:opacity-100 transition-opacity">
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div title="Discard Changes" class="hover:bg-red-500/20 text-white/50 hover:text-red-400 rounded w-5 h-5 flex items-center justify-center p-0.5" onclick={(e) => { e.stopPropagation(); discardChange(file.path); }}>
            <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 10h10a8 8 0 018 8v2M3 10l6 6m-6-6l6-6"></path></svg>
          </div>
        </div>
      </div>
    {/each}
    {#if files.length === 0}
      <div class="px-4 py-4 text-white/30 italic text-xs text-center flex flex-col items-center gap-2">
        <svg class="w-8 h-8 text-white/10" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>
        No changes found
      </div>
    {/if}
  </div>
</div>
