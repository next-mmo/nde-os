<svelte:options runes={true} />

<script lang="ts">
  import { Film, Plus, Trash2 } from "@lucide/svelte";
  import type { ProjectSummary } from "../types";

  interface Props {
    projects: ProjectSummary[];
    onCreate: () => void;
    onOpen: (id: string) => void;
    onDelete: (id: string) => void;
  }

  let { projects, onCreate, onOpen, onDelete }: Props = $props();
</script>

<div class="flex flex-col h-full bg-linear-to-br from-zinc-950 via-zinc-900 to-zinc-950">
  <header class="flex items-center justify-between px-6 py-4 border-b border-white/5">
    <div class="flex items-center gap-3">
      <div class="w-9 h-9 rounded-xl bg-linear-to-br from-violet-500 to-fuchsia-600 grid place-items-center shadow-lg shadow-violet-500/20">
        <Film class="w-5 h-5 text-white" />
      </div>
      <div>
        <h1 class="text-lg font-semibold text-white tracking-tight">FreeCut</h1>
        <p class="text-xs text-white/40">Video Editor</p>
      </div>
    </div>
    <button
      class="flex items-center gap-2 px-4 py-2 rounded-lg bg-violet-600 hover:bg-violet-500 text-white text-sm font-medium transition-colors shadow-lg shadow-violet-600/20"
      onclick={onCreate}
    >
      <Plus class="w-4 h-4" />
      New Project
    </button>
  </header>
  <div class="flex-1 overflow-y-auto p-6">
    {#if projects.length === 0}
      <div class="flex flex-col items-center justify-center h-full gap-4 text-white/30">
        <Film class="w-16 h-16" />
        <p class="text-lg font-medium">No projects yet</p>
        <p class="text-sm">Create a new project to get started</p>
      </div>
    {:else}
      <div class="grid grid-cols-[repeat(auto-fill,minmax(240px,1fr))] gap-4">
        {#each projects as project (project.id)}
          <div
            role="button"
            tabindex="0"
            class="group flex flex-col rounded-xl border border-white/5 bg-white/[0.02] hover:bg-white/[0.05] hover:border-white/10 transition-all duration-200 overflow-hidden text-left cursor-pointer"
            onclick={() => onOpen(project.id)}
            onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') onOpen(project.id); }}
          >
            <div class="aspect-video bg-linear-to-br from-zinc-800 to-zinc-900 flex items-center justify-center relative">
              <Film class="w-10 h-10 text-white/10 group-hover:text-white/20 transition-colors" />
              <button
                class="absolute top-2 right-2 p-1 rounded bg-red-500/0 hover:bg-red-500/80 text-white/0 group-hover:text-white/60 hover:text-white! transition-all"
                onclick={(e) => { e.stopPropagation(); onDelete(project.id); }}
                aria-label="Delete project"
              >
                <Trash2 class="w-3.5 h-3.5" />
              </button>
            </div>
            <div class="p-3">
              <p class="text-sm font-medium text-white/80 truncate">{project.name}</p>
              <p class="text-xs text-white/30 mt-1">{project.updatedAt?.split("T")[0] ?? ""}</p>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
