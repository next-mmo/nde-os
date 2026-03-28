<script lang="ts">
  import type { FDocument, FNode } from "$lib/figma-json/types";

  interface Props {
    document: FDocument;
    selectedNodeId: string | null;
    onSelectNode?: (id: string | null) => void;
    width?: number;
  }

  let { document, selectedNodeId, onSelectNode, width }: Props = $props();

  function toggleVisibility(node: FNode, e: Event) {
    e.stopPropagation();
    node.visible = node.visible === false ? true : false;
  }
</script>

{#snippet treeNode(node: FNode, depth: number)}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div 
    class="flex items-center group cursor-pointer text-xs {node.id === selectedNodeId ? 'bg-indigo-500/30 text-white' : 'text-white/70 hover:bg-white/5'}"
    style="padding-left: {depth * 12 + 8}px; padding-right: 8px; height: 28px;"
    onclick={() => onSelectNode && onSelectNode(node.id)}
  >
    <div class="flex-1 flex items-center gap-2 overflow-hidden">
      <!-- Icon based on type -->
      <span class="text-white/40 shrink-0 select-none text-[10px] w-3 text-center block">
        {#if node.type === 'FRAME'}◰
        {:else if node.type === 'TEXT'}T
        {:else if node.type === 'IMAGE'}🖼
        {:else if node.type === 'GROUP'}◱
        {:else if node.type === 'VECTOR' || node.type === 'LINE'}↗
        {:else}▨{/if}
      </span>
      <span class="truncate {node.visible === false ? 'opacity-40' : ''}">{node.name || node.type}</span>
    </div>
    
    <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
      <!-- Visibility toggle -->
      <button aria-label="Toggle Visibility" class="p-1 hover:text-white transition-colors" onclick={(e) => toggleVisibility(node, e)} title="Toggle visibility">
        {#if node.visible === false}
          <svg class="w-3 h-3 text-white/40" aria-hidden="true" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21"></path></svg>
        {:else}
          <svg class="w-3 h-3" aria-hidden="true" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.543 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.543-7z"></path></svg>
        {/if}
      </button>
    </div>
  </div>
  
  {#if "children" in node && node.children}
    {#each node.children as child (child.id)}
      {@render treeNode(child, depth + 1)}
    {/each}
  {/if}
{/snippet}

<div class="border-r border-white/10 bg-black/40 shrink-0 hidden md:flex flex-col h-full overflow-hidden select-none" style="width: {width ?? 256}px">
  <div class="h-10 border-b border-white/10 flex items-center px-4 shrink-0 font-medium text-white/80 text-xs text-center justify-between">
    <span>Layers</span>
  </div>
  <div class="flex-1 overflow-y-auto py-2">
    {#each document.children as node (node.id)}
      {@render treeNode(node, 0)}
    {/each}
  </div>
</div>
