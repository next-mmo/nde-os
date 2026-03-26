<script lang="ts">
  import type { FDocument, FNode, FFrameNode } from "$lib/figma-json/types";
  import CanvasEditor from "./canvas/CanvasEditor.svelte";
  import PropertiesPanel from "./panels/PropertiesPanel.svelte";
  import LayerTree from "./panels/LayerTree.svelte";
  import AgentChat from "./panels/AgentChat.svelte";
  import KanbanBoard from "./tabs/KanbanBoard.svelte";
  import IDE from "./ide/IDE.svelte";
  import { closeWindow } from "🍎/state/desktop.svelte";

  interface Props {
    window?: import("🍎/state/desktop.svelte").DesktopWindow;
  }

  let { window }: Props = $props();

  // Create initial document state
  let document = $state<FDocument>(JSON.parse(JSON.stringify({
    version: "1.0",
    id: "0:0",
    type: "DOCUMENT",
    name: "Document",
    children: [
      {
        id: "1:2",
        type: "FRAME",
        name: "Frame 1",
        x: 100,
        y: 100,
        width: 800,
        height: 600,
        fills: [{ type: "SOLID", color: { r: 1, g: 1, b: 1, a: 1 } }],
        children: []
      }
    ]
  })));
  
  let selectedNodeId = $state<string | null>(null);
  let zoom = $state(1);

  let activeTab = $state<"preview" | "json" | "figma" | "kanban" | "ide">("preview");
  let chatMode = $state<"figma" | "scrum" | "dev">("figma");

  let activeFilePath = $state<string | null>(null);
  let fileContent = $state<string>("");

  function updateNodePosition(id: string, x: number, y: number) {
    // Recursive search and update within the document tree
    function walk(nodes: FNode[]): boolean {
      for (const node of nodes) {
        if (node.id === id) {
          node.x = Math.round(x);
          node.y = Math.round(y);
          return true;
        }
        if ("children" in node && node.children) {
          if (walk(node.children as FNode[])) return true;
        }
      }
      return false;
    }
    walk(document.children);
  }

  function updateNodeSize(id: string, width: number, height: number) {
    function walk(nodes: FNode[]): boolean {
      for (const node of nodes) {
        if (node.id === id) {
          if ("width" in node) node.width = Math.round(width);
          if ("height" in node) node.height = Math.round(height);
          return true;
        }
        if ("children" in node && node.children) {
          if (walk(node.children as FNode[])) return true;
        }
      }
      return false;
    }
    walk(document.children);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if ((e.key === "Delete" || e.key === "Backspace") && selectedNodeId) {
      // Ignore if user is currently typing in an input or textarea
      const target = e.target as HTMLElement;
      if (target.tagName === "INPUT" || target.tagName === "TEXTAREA") return;
      deleteNode(selectedNodeId);
      selectedNodeId = null;
    }
  }

  function deleteNode(id: string) {
    function walk(nodes: FNode[]): boolean {
      for (let i = 0; i < nodes.length; i++) {
        if (nodes[i].id === id) {
          nodes.splice(i, 1);
          return true;
        }
        const node = nodes[i];
        if ("children" in node && node.children) {
          if (walk(node.children)) return true;
        }
      }
      return false;
    }
    walk(document.children);
  }

  function applyChatPatch(patch: any) {
    // Better handling of chat patches directly replacing nodes or appending them
    if (patch.op === "append" && Array.isArray(patch.nodes)) {
      if (selectedNodeId) {
        const parent = findNode(document.children, selectedNodeId);
        if (parent && "children" in parent && Array.isArray(parent.children)) {
           parent.children = [...parent.children, ...patch.nodes];
           return;
        }
      }
      document.children = [...document.children, ...patch.nodes];
    } else if (patch.id) {
       // Check if replacing existing
       const existing = findNode(document.children, patch.id);
       if (existing) {
         Object.assign(existing, patch);
       } else {
         if (selectedNodeId) {
            const parent = findNode(document.children, selectedNodeId);
            if (parent && "children" in parent && Array.isArray(parent.children)) {
               parent.children = [...parent.children, patch];
               return;
            }
         }
         document.children = [...document.children, patch];
       }
    }
  }

  function findNode(nodes: FNode[], id: string): FNode | null {
    for (const node of nodes) {
      if (node.id === id) return node;
      if ("children" in node && node.children) {
        const found = findNode(node.children as FNode[], id);
        if (found) return found;
      }
    }
    return null;
  }

  function createNode(type: string) {
    const id = crypto.randomUUID().slice(0, 8);
    let node: FNode;
    const base = { id, x: 100, y: 100 };
    
    if (type === 'FRAME') {
      node = { ...base, type: "FRAME", name: "Frame", width: 200, height: 200, fills: [{ type: "SOLID", color: { r: 0.2, g: 0.2, b: 0.2, a: 1 } }], children: [] };
    } else if (type === 'TEXT') {
      node = { ...base, type: "TEXT", name: "Text", characters: "Type something...", fontSize: 16, fills: [{ type: "SOLID", color: { r: 1, g: 1, b: 1, a: 1 } }] } as FNode;
    } else {
      node = { ...base, type: "RECTANGLE", name: "Rectangle", width: 100, height: 100, fills: [{ type: "SOLID", color: { r: 0.5, g: 0.5, b: 0.5, a: 1 } }] } as FNode;
    }
    
    if (selectedNodeId) {
      const parent = findNode(document.children, selectedNodeId);
      if (parent && "children" in parent && Array.isArray(parent.children)) {
        parent.children.push(node);
        selectedNodeId = id;
        return;
      }
    }
    
    document.children.push(node);
    selectedNodeId = id;
  }

  // Synchronization with IDE files
  let lastStr = "";
  $effect(() => {
    if (activeFilePath && (activeFilePath.endsWith('.json') || activeFilePath.endsWith('.fj'))) {
      if (fileContent && fileContent !== lastStr) {
        try {
          const parsed = JSON.parse(fileContent);
          if (parsed.type === 'DOCUMENT') {
            document = parsed;
            lastStr = fileContent;
          }
        } catch (e) {}
      }
    }
  });

  $effect(() => {
    if (activeFilePath && (activeFilePath.endsWith('.json') || activeFilePath.endsWith('.fj'))) {
      const str = JSON.stringify(document, null, 2);
      if (str !== lastStr) {
        lastStr = str;
        fileContent = str;
      }
    }
  });
</script>

<svelte:window onkeydown={handleKeyDown} />

<div class="flex h-full w-full bg-background/90 backdrop-blur text-foreground overflow-hidden font-sans">
  
  <!-- Left Side: Main Area (80%) -->
  <div class="flex-1 flex flex-col min-w-0 border-r border-white/10">
    <!-- Header / Tab Bar -->
    <header class="h-12 border-b border-white/10 flex items-center justify-between px-4 shrink-0 bg-black/20" data-tauri-drag-region>
      <div class="flex items-center gap-4">
        <h1 class="text-sm font-semibold tracking-wide flex items-center gap-2 pointer-events-none text-white/90">
          <div class="w-2 h-2 rounded-full bg-indigo-500 shadow-[0_0_8px_rgba(99,102,241,0.5)]"></div>
          Vibe Studio
        </h1>
        
        <div class="flex bg-black/40 p-1 rounded-md ml-4">
          <button 
            class="px-3 py-1 text-xs font-medium rounded transition-colors {activeTab === 'preview' ? 'bg-white/10 text-white shadow-sm' : 'text-white/50 hover:text-white/80'}"
            onclick={() => activeTab = "preview"}
          >
            Preview
          </button>
          <button 
            class="px-3 py-1 text-xs font-medium rounded transition-colors {activeTab === 'json' ? 'bg-white/10 text-white shadow-sm' : 'text-white/50 hover:text-white/80'}"
            onclick={() => activeTab = "json"}
          >
            JSON
          </button>
          <button 
            class="px-3 py-1 text-xs font-medium rounded transition-colors {activeTab === 'figma' ? 'bg-white/10 text-white shadow-sm' : 'text-white/50 hover:text-white/80'}"
            onclick={() => activeTab = "figma"}
          >
            Figma
          </button>
          <button 
            class="px-3 py-1 text-xs font-medium rounded transition-colors {activeTab === 'kanban' ? 'bg-white/10 text-white shadow-sm' : 'text-white/50 hover:text-white/80'}"
            onclick={() => {
              activeTab = "kanban";
              chatMode = "scrum";
            }}
          >
            Kanban 📋
          </button>
          <button 
            class="px-3 py-1 text-xs font-medium rounded transition-colors {activeTab === 'ide' ? 'bg-white/10 text-white shadow-sm' : 'text-white/50 hover:text-white/80'}"
            onclick={() => {
              activeTab = "ide";
              chatMode = "dev";
            }}
          >
            IDE 💻
          </button>
        </div>
      </div>
      
      <!-- Window Controls -->
      <div class="flex gap-2 relative z-50">
        {#if window}
          <button aria-label="Close Desktop Window" class="w-8 h-8 rounded-full hover:bg-white/10 flex items-center justify-center transition-colors" onclick={() => closeWindow(window!.id)}>
            <svg class="w-4 h-4 text-white/70" aria-hidden="true" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path></svg>
          </button>
        {/if}
      </div>
    </header>

    <!-- Main Content Area -->
    <main class="flex-1 relative overflow-hidden bg-black/40">
      {#if activeTab === "preview"}
        <div class="absolute inset-0 flex">
          <LayerTree 
            {document} 
            {selectedNodeId} 
            onSelectNode={(id) => selectedNodeId = id} 
          />
          
          <!-- Interactive Canvas -->
          <div class="flex-1 relative">
            <CanvasEditor 
              {document} 
              {selectedNodeId}
              bind:zoom
              onSelectNode={(id) => selectedNodeId = id}
              onUpdateNodePosition={updateNodePosition}
              onUpdateNodeSize={updateNodeSize}
            />
            
            <!-- Toolbar -->
            <div class="absolute top-4 left-1/2 -translate-x-1/2 flex items-center gap-1.5 px-3 py-1.5 bg-black/60 border border-white/10 backdrop-blur-md rounded-full text-white/50 shadow-2xl z-50">
              <button class="w-8 h-8 rounded-full hover:bg-white/10 hover:text-white flex items-center justify-center transition-colors" onclick={() => createNode('FRAME')} title="Add Frame">◰</button>
              <button class="w-8 h-8 rounded-full hover:bg-white/10 hover:text-white flex items-center justify-center transition-colors" onclick={() => createNode('RECTANGLE')} title="Add Rectangle">▨</button>
              <button class="w-8 h-8 rounded-full hover:bg-white/10 hover:text-white flex items-center justify-center transition-colors font-serif font-bold" onclick={() => createNode('TEXT')} title="Add Text">T</button>
            </div>
          </div>

          <PropertiesPanel {document} {selectedNodeId} />
        </div>
      {:else if activeTab === "json"}
        <div class="absolute inset-0 p-4">
          <textarea 
            class="w-full h-full bg-black/50 text-emerald-400 font-mono text-sm p-4 rounded-lg focus:outline-none resize-none border border-white/10"
            value={JSON.stringify(document, null, 2)}
            oninput={(e) => {
              try {
                document = JSON.parse(e.currentTarget.value);
              } catch {}
            }}
          ></textarea>
        </div>
      {:else if activeTab === "kanban"}
        <div class="absolute inset-0">
          <KanbanBoard />
        </div>
      {:else if activeTab === "ide"}
        <div class="absolute inset-0">
          <IDE bind:activeFilePath bind:fileContent />
        </div>
      {:else}
        <div class="absolute inset-0 flex items-center justify-center">
          <p class="text-white/50">Figma import coming soon...</p>
        </div>
      {/if}
    </main>

    <!-- Status Bar -->
    <footer class="h-8 border-t border-white/10 flex items-center justify-between px-4 text-xs text-white/40 bg-black/60 shrink-0">
      <div class="flex items-center gap-4">
        <span>Nodes: {document.children.length}</span>
        <span>Selected: {selectedNodeId ?? 'None'}</span>
      </div>
      <div class="flex items-center gap-3">
        <button class="hover:text-white transition-colors" onclick={() => zoom = Math.max(0.1, zoom - 0.1)}>-</button>
        <span class="w-10 text-center">{Math.round(zoom * 100)}%</span>
        <button class="hover:text-white transition-colors" onclick={() => zoom = Math.min(5, zoom + 0.1)}>+</button>
      </div>
    </footer>
  </div>

  <!-- Right Side: Chat Panel (20%) -->
  <div class="w-80 border-l border-white/10 bg-black/20 shrink-0 flex-none hidden lg:flex flex-col">
    <div class="h-12 border-b border-white/10 flex items-center justify-between px-4 shrink-0 bg-black/40">
      <h2 class="text-sm font-medium text-white/80">AI Agent Workspace</h2>
      <div class="flex bg-black/40 p-1 rounded-md">
        <button class="px-2 py-0.5 text-[10px] font-medium rounded {chatMode === 'figma' ? 'bg-white/10 text-white' : 'text-white/50 hover:text-white/80'}" onclick={() => chatMode = 'figma'}>Figma Agent</button>
        <button class="px-2 py-0.5 text-[10px] font-medium rounded {chatMode === 'scrum' ? 'bg-white/10 text-white' : 'text-white/50 hover:text-white/80'}" onclick={() => chatMode = 'scrum'}>Scrum Master</button>
        <button class="px-2 py-0.5 text-[10px] font-medium rounded {chatMode === 'dev' ? 'bg-white/10 text-white' : 'text-white/50 hover:text-white/80'}" onclick={() => chatMode = 'dev'}>Agent IDE</button>
      </div>
    </div>
    
    <AgentChat {document} {chatMode} {activeFilePath} {fileContent} onApplyPatch={applyChatPatch} />
  </div>
</div>
