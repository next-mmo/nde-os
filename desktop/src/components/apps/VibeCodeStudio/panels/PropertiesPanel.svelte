<script lang="ts">
  import type { FDocument, FNode } from "$lib/figma-json/types";

  interface Props {
    document: FDocument;
    selectedNodeId: string | null;
    width?: number;
  }

  let { document, selectedNodeId, width }: Props = $props();

  // Find node by ID
  const selectedNode = $derived.by(() => {
    if (!selectedNodeId) return null;
    let found = null;
    function walk(nodes: any[]): FNode | null {
      for (const node of nodes) {
        if (node.id === selectedNodeId) return node;
        if (node.children) {
          const res = walk(node.children);
          if (res) return res;
        }
      }
      return null;
    }
    return walk(document.children) as FNode | null;
  });
  
</script>

<div class="border-l border-white/10 bg-black/40 shrink-0 flex flex-col h-full overflow-y-auto text-xs" style="width: {width ?? 288}px">
  <div class="h-10 border-b border-white/10 flex items-center px-4 shrink-0 font-medium text-white/80">
    Design
  </div>
  
  {#if selectedNode}
    <!-- Basic Info -->
    <div class="p-4 border-b border-white/10 space-y-3">
      <div class="flex items-center justify-between">
        <span class="text-white/50">{selectedNode.type}</span>
        <span class="text-white/30 font-mono text-[10px] truncate max-w-[100px]">{selectedNode.id}</span>
      </div>
      <div>
        <input type="text" bind:value={selectedNode.name} class="w-full bg-black/50 border border-white/10 rounded px-2 py-1 text-white focus:outline-none focus:border-indigo-500" />
      </div>
    </div>
    
    <!-- Layout / Position -->
    <div class="p-4 border-b border-white/10 space-y-3">
      <div class="font-medium text-white/70">Layout</div>
      <div class="grid grid-cols-2 gap-2">
        <div class="flex items-center bg-black/50 border border-white/10 rounded px-2 py-1">
          <span class="text-white/30 w-4">X</span>
          <input type="number" bind:value={(selectedNode as any).x} class="w-full bg-transparent text-white focus:outline-none" />
        </div>
        <div class="flex items-center bg-black/50 border border-white/10 rounded px-2 py-1">
          <span class="text-white/30 w-4">Y</span>
          <input type="number" bind:value={(selectedNode as any).y} class="w-full bg-transparent text-white focus:outline-none" />
        </div>
        <div class="flex items-center bg-black/50 border border-white/10 rounded px-2 py-1">
          <span class="text-white/30 w-4">W</span>
          <input type="number" bind:value={(selectedNode as any).width} class="w-full bg-transparent text-white focus:outline-none" />
        </div>
        <div class="flex items-center bg-black/50 border border-white/10 rounded px-2 py-1">
          <span class="text-white/30 w-4">H</span>
          <input type="number" bind:value={(selectedNode as any).height} class="w-full bg-transparent text-white focus:outline-none" />
        </div>
      </div>
    </div>
    
    <!-- Fills -->
    {#if selectedNode.fills && selectedNode.fills.length > 0}
      <div class="p-4 border-b border-white/10 space-y-3">
        <div class="font-medium text-white/70">Fill</div>
        {#each selectedNode.fills as fill}
          {#if fill.type === "SOLID"}
            <div class="flex items-center gap-2">
              <div class="w-6 h-6 rounded border border-white/20" style="background: rgba({fill.color.r*255},{fill.color.g*255},{fill.color.b*255},{fill.color.a})"></div>
              <div class="flex-1 bg-black/50 border border-white/10 rounded px-2 py-1 text-white/70">
                Solid Color
              </div>
            </div>
          {/if}
        {/each}
      </div>
    {/if}

    <!-- Metadata (Readonly JSON) -->
    <div class="p-4 space-y-3 flex-1">
      <div class="font-medium text-white/70">Metadata</div>
      <div class="bg-black/50 rounded border border-white/10 p-2 overflow-x-auto text-[10px] text-emerald-400 font-mono h-48">
        <pre>{JSON.stringify(selectedNode, null, 2)}</pre>
      </div>
    </div>
    
  {:else}
    <div class="flex-1 flex flex-col items-center justify-center text-white/40 text-center p-6 gap-2">
      <svg class="w-8 h-8 text-white/20" aria-hidden="true" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 15l-2 5L9 9l11 4-5 2zm0 0l5 5M7.188 2.239l.777 2.897M5.136 7.965l-2.898-.777M13.95 4.05l-2.122 2.122m-5.657 5.656l-2.12 2.122"></path></svg>
      <div>Select a node on the canvas</div>
    </div>
  {/if}
</div>
