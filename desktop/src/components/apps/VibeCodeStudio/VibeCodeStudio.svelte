<script lang="ts">
  import type { FDocument, FNode, FFrameNode } from "$lib/figma-json/types";
  import CanvasEditor from "./canvas/CanvasEditor.svelte";
  import PropertiesPanel from "./panels/PropertiesPanel.svelte";
  import LayerTree from "./panels/LayerTree.svelte";
  import AgentChat from "./panels/AgentChat.svelte";
  import V0Runner from "./canvas/V0Runner.svelte";
  import KanbanBoard from "./tabs/KanbanBoard.svelte";
  import IDE from "./ide/IDE.svelte";
  import ResizeHandle from "./panels/ResizeHandle.svelte";
  import { closeWindow } from "🍎/state/desktop.svelte";
  import { CatalogRenderer, registry } from "$lib/json-render";

  // shadcn-svelte components
  import { Button } from "$lib/components/ui/button";
  import { Badge } from "$lib/components/ui/badge";
  import { Separator } from "$lib/components/ui/separator";
  import * as Tabs from "$lib/components/ui/tabs";
  import * as Tooltip from "$lib/components/ui/tooltip";

  // Lucide icons
  import {
    Eye,
    Code2,
    Columns3,
    Terminal,
    X,
    Minus,
    Plus,
    Frame,
    RectangleHorizontal,
    Type,
    Sparkles,
    Bot,
    PanelRightClose,
    PanelRight,
    Pencil,
    Globe,
    SplitSquareHorizontal,
    Braces,
    RotateCcw,
    Copy,
    Check,
  } from "@lucide/svelte";

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

  let activeTab = $state<"preview" | "json" | "kanban" | "ide">("preview");
  let chatMode = $state<"scrum" | "dev">("scrum");
  let showChatPanel = $state(true);
  let previewMode = $state<"editor" | "preview" | "split">("editor");

  // React to window.data.tab deep-link (e.g. from NDE Chat "Open Kanban" button)
  $effect(() => {
    const tab = window?.data?.tab;
    if (tab && ["preview", "json", "kanban", "ide"].includes(tab)) {
      activeTab = tab;
      if (tab === "kanban") chatMode = "scrum";
      if (tab === "ide") chatMode = "dev";
    }
  });

  let activeFilePath = $state<string | null>(null);
  let fileContent = $state<string>("");
  let generatedCode = $state<string>("");

  // ── JSON Spec Editor State ──────────────────────────────────────
  // specEditorText is the raw JSON string the user can edit manually.
  // It syncs FROM generatedCode when the agent produces output,
  // and syncs TO liveSpec when the user edits.
  let specEditorText = $state<string>("");
  let specParseError = $state<string | null>(null);
  let copiedSpec = $state(false);

  // When agent produces new code, push it into the editor
  $effect(() => {
    if (generatedCode) {
      const trimmed = generatedCode.trim();
      if (trimmed.startsWith("[") || trimmed.startsWith("{")) {
        try {
          // Pretty-print the spec for the editor
          specEditorText = JSON.stringify(JSON.parse(trimmed), null, 2);
          specParseError = null;
        } catch {
          // Not valid JSON — keep raw HTML/code
          specEditorText = "";
        }
      }
    }
  });

  // Live-parse the editor text into a renderable spec
  let liveSpec = $derived.by(() => {
    if (!specEditorText) {
      // Fallback: try parsing generatedCode directly
      if (!generatedCode) return null;
      const trimmed = generatedCode.trim();
      if (!trimmed.startsWith("[") && !trimmed.startsWith("{")) return null;
      try { return JSON.parse(trimmed); } catch { return null; }
    }
    try {
      return JSON.parse(specEditorText);
    } catch {
      return null;
    }
  });

  // Whether we are in "spec mode" (JSON-render) vs "code mode" (HTML/V0Runner)
  let isSpecMode = $derived(!!liveSpec || (specEditorText.trim().startsWith("[") || specEditorText.trim().startsWith("{")));

  function handleSpecEdit(text: string) {
    specEditorText = text;
    try {
      JSON.parse(text);
      specParseError = null;
    } catch (e: any) {
      specParseError = e.message ?? "Invalid JSON";
    }
  }

  function resetSpec() {
    specEditorText = "";
    specParseError = null;
    generatedCode = "";
  }

  async function copySpec() {
    try {
      await navigator.clipboard.writeText(specEditorText || generatedCode);
      copiedSpec = true;
      setTimeout(() => copiedSpec = false, 1500);
    } catch {}
  }

  // Shared context files: populated by FileExplorer drag/right-click → AgentChat
  interface ContextFile { path: string; name: string; content?: string; }
  let contextFiles = $state<ContextFile[]>([]);

  async function addContextFileFromExplorer(path: string, name: string) {
    if (contextFiles.some(f => f.path === path)) return;
    let content = "";
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      content = await invoke<string>("read_file_content", { path });
    } catch {}
    contextFiles = [...contextFiles, { path, name, content }];
  }

  // Resizable panel widths
  let layerTreeWidth = $state(256);
  let propertiesWidth = $state(288);
  let chatWidth = $state(320);
  let rootEl: HTMLDivElement | undefined = $state();

  const MIN_PANEL = 150;
  const MAX_PANEL_RATIO = 0.5;

  function clampPanel(value: number): number {
    const max = rootEl ? rootEl.clientWidth * MAX_PANEL_RATIO : 600;
    return Math.max(MIN_PANEL, Math.min(value, max));
  }

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
    if (patch.code) {
      generatedCode = patch.code;
      if (!activeFilePath || activeFilePath.endsWith('.json') || activeFilePath.endsWith('.fj')) {
        activeFilePath = 'C:\\Users\\dila\\Downloads\\ai-launcher-v0.2\\ai-launcher\\desktop\\ui.html';
      }
      fileContent = patch.code;
      // Auto-switch to split when code is generated so user sees editor + preview
      if (activeTab === "preview" && previewMode === "editor") {
        previewMode = "split";
      }
      return;
    }

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
    } else if (activeFilePath && (activeFilePath.endsWith('.html') || activeFilePath.endsWith('.svelte') || activeFilePath.endsWith('.tsx'))) {
      if (fileContent !== lastStr) {
        generatedCode = fileContent;
        lastStr = fileContent;
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

<div bind:this={rootEl} class="flex h-full w-full bg-background text-foreground overflow-hidden font-sans">

  <!-- ═══ Left Side: Main Area ═══ -->
  <div class="flex-1 flex flex-col min-w-0">

    <!-- ─── Header / Tab Bar ─── -->
    <header class="h-11 border-b border-border flex items-center justify-between px-3 shrink-0 bg-muted/30 backdrop-blur-sm" data-tauri-drag-region>
      <div class="flex items-center gap-3">
        <!-- Branding -->
        <div class="flex items-center gap-2 pointer-events-none select-none">
          <Sparkles class="size-3.5 text-chart-2" />
          <span class="text-sm font-semibold tracking-tight">Vibe Studio</span>
        </div>

        <Separator orientation="vertical" class="h-5!" />

        <!-- Tab Switcher -->
        <Tabs.Root bind:value={activeTab} onValueChange={(v) => {
          if (v === "kanban") chatMode = "scrum";
          if (v === "ide") chatMode = "dev";
        }}>
          <Tabs.List class="h-7">
            <Tabs.Trigger value="preview" class="gap-1 text-xs px-2.5">
              <Eye class="size-3" />
              Preview
            </Tabs.Trigger>
            <Tabs.Trigger value="json" class="gap-1 text-xs px-2.5">
              <Code2 class="size-3" />
              JSON
            </Tabs.Trigger>
            <Tabs.Trigger value="kanban" class="gap-1 text-xs px-2.5">
              <Columns3 class="size-3" />
              Kanban
            </Tabs.Trigger>
            <Tabs.Trigger value="ide" class="gap-1 text-xs px-2.5">
              <Terminal class="size-3" />
              IDE
            </Tabs.Trigger>
          </Tabs.List>
        </Tabs.Root>
      </div>

      <!-- Right-side controls -->
      <div class="flex items-center gap-1">
        <!-- Toggle chat panel -->
        <Tooltip.Root>
          <Tooltip.Trigger>
            <Button variant="ghost" size="icon-xs" onclick={() => showChatPanel = !showChatPanel} class="text-muted-foreground">
              {#if showChatPanel}
                <PanelRightClose class="size-3.5" />
              {:else}
                <PanelRight class="size-3.5" />
              {/if}
            </Button>
          </Tooltip.Trigger>
          <Tooltip.Content>{showChatPanel ? 'Hide chat' : 'Show chat'}</Tooltip.Content>
        </Tooltip.Root>

        {#if window}
          <Tooltip.Root>
            <Tooltip.Trigger>
              <Button variant="ghost" size="icon-xs" onclick={() => closeWindow(window!.id)} class="text-muted-foreground hover:text-destructive">
                <X class="size-3.5" />
              </Button>
            </Tooltip.Trigger>
            <Tooltip.Content>Close window</Tooltip.Content>
          </Tooltip.Root>
        {/if}
      </div>
    </header>

    <!-- ─── Main Content Area ─── -->
    <main class="flex-1 relative overflow-hidden bg-background">
      {#if activeTab === "preview"}
        <div class="absolute inset-0 flex flex-col">
          <!-- Sub-tab bar: Editor / Preview / Split -->
          <div class="h-9 border-b border-border flex items-center justify-between px-3 shrink-0 bg-muted/10">
            <div class="flex items-center gap-2">
              <Tabs.Root bind:value={previewMode}>
                <Tabs.List class="h-6">
                  <Tabs.Trigger value="editor" class="gap-1 text-[11px] px-2 h-5">
                    {#if isSpecMode}
                      <Braces class="size-3" />
                      JSON Editor
                    {:else}
                      <Pencil class="size-3" />
                      Canvas
                    {/if}
                  </Tabs.Trigger>
                  <Tabs.Trigger value="preview" class="gap-1 text-[11px] px-2 h-5">
                    <Eye class="size-3" />
                    Preview
                  </Tabs.Trigger>
                  <Tabs.Trigger value="split" class="gap-1 text-[11px] px-2 h-5">
                    <SplitSquareHorizontal class="size-3" />
                    Split
                  </Tabs.Trigger>
                </Tabs.List>
              </Tabs.Root>

              {#if isSpecMode}
                <Separator orientation="vertical" class="h-4!" />
                <Badge variant="secondary" class="h-4 px-1.5 text-[10px] gap-1">
                  <Braces class="size-2.5" />
                  Spec Mode
                </Badge>
              {/if}
            </div>

            <div class="flex items-center gap-1">
              {#if specParseError}
                <Badge variant="destructive" class="h-4 px-1.5 text-[10px]">
                  Parse Error
                </Badge>
              {:else if generatedCode || specEditorText}
                <Badge variant="secondary" class="h-4 px-1.5 text-[10px] gap-1">
                  <span class="size-1.5 rounded-full bg-green-500 animate-pulse"></span>
                  Live
                </Badge>
              {/if}

              {#if specEditorText || generatedCode}
                <Tooltip.Root>
                  <Tooltip.Trigger>
                    <Button variant="ghost" size="icon-xs" onclick={copySpec} class="text-muted-foreground">
                      {#if copiedSpec}
                        <Check class="size-3 text-green-500" />
                      {:else}
                        <Copy class="size-3" />
                      {/if}
                    </Button>
                  </Tooltip.Trigger>
                  <Tooltip.Content>Copy spec</Tooltip.Content>
                </Tooltip.Root>

                <Tooltip.Root>
                  <Tooltip.Trigger>
                    <Button variant="ghost" size="icon-xs" onclick={resetSpec} class="text-muted-foreground hover:text-destructive">
                      <RotateCcw class="size-3" />
                    </Button>
                  </Tooltip.Trigger>
                  <Tooltip.Content>Reset</Tooltip.Content>
                </Tooltip.Root>
              {/if}
            </div>
          </div>

          <!-- Sub-tab content -->
          <div class="flex-1 relative overflow-hidden">
            {#if previewMode === "editor"}
              {#if isSpecMode}
                <!-- ── JSON Spec Editor ── -->
                <div class="absolute inset-0 flex flex-col">
                  <textarea
                    class="flex-1 w-full rounded-none border-none bg-zinc-950 p-4 font-mono text-xs text-emerald-400 placeholder:text-zinc-600 focus:outline-none resize-none leading-relaxed"
                    placeholder="Paste or type your JSON spec here..."
                    value={specEditorText}
                    oninput={(e) => handleSpecEdit(e.currentTarget.value)}
                    spellcheck="false"
                  ></textarea>
                  {#if specParseError}
                    <div class="shrink-0 px-3 py-1.5 bg-destructive/10 border-t border-destructive/30 text-destructive text-[11px] font-mono truncate">
                      ⚠ {specParseError}
                    </div>
                  {/if}
                </div>
              {:else}
                <!-- ── Figma Canvas Editor (non-spec mode) ── -->
                <div class="absolute inset-0 flex">
                  <LayerTree
                    {document}
                    {selectedNodeId}
                    onSelectNode={(id) => selectedNodeId = id}
                    width={layerTreeWidth}
                  />
                  <ResizeHandle onResize={(d) => layerTreeWidth = clampPanel(layerTreeWidth + d)} />

                  <div class="flex-1 relative min-w-0">
                    <CanvasEditor
                      {document}
                      {selectedNodeId}
                      bind:zoom
                      onSelectNode={(id) => selectedNodeId = id}
                      onUpdateNodePosition={updateNodePosition}
                      onUpdateNodeSize={updateNodeSize}
                    />

                    <!-- Floating Toolbar -->
                    <div class="absolute top-3 left-1/2 -translate-x-1/2 flex items-center gap-1 rounded-lg border border-border bg-popover/90 px-2 py-1 shadow-lg backdrop-blur-md z-50">
                      <Tooltip.Root>
                        <Tooltip.Trigger>
                          <Button variant="ghost" size="icon-xs" onclick={() => createNode('FRAME')} class="text-muted-foreground">
                            <Frame class="size-3.5" />
                          </Button>
                        </Tooltip.Trigger>
                        <Tooltip.Content>Add Frame</Tooltip.Content>
                      </Tooltip.Root>
                      <Tooltip.Root>
                        <Tooltip.Trigger>
                          <Button variant="ghost" size="icon-xs" onclick={() => createNode('RECTANGLE')} class="text-muted-foreground">
                            <RectangleHorizontal class="size-3.5" />
                          </Button>
                        </Tooltip.Trigger>
                        <Tooltip.Content>Add Rectangle</Tooltip.Content>
                      </Tooltip.Root>
                      <Tooltip.Root>
                        <Tooltip.Trigger>
                          <Button variant="ghost" size="icon-xs" onclick={() => createNode('TEXT')} class="text-muted-foreground">
                            <Type class="size-3.5" />
                          </Button>
                        </Tooltip.Trigger>
                        <Tooltip.Content>Add Text</Tooltip.Content>
                      </Tooltip.Root>
                    </div>
                  </div>

                  <ResizeHandle onResize={(d) => propertiesWidth = clampPanel(propertiesWidth - d)} />
                  <PropertiesPanel {document} {selectedNodeId} width={propertiesWidth} />
                </div>
              {/if}

            {:else if previewMode === "preview"}
              <!-- ── Full-width live preview ── -->
              <div class="absolute inset-0">
                {#if liveSpec}
                  <div class="w-full h-full p-6 overflow-auto bg-background">
                    <CatalogRenderer spec={liveSpec} {registry} />
                  </div>
                {:else if generatedCode}
                  <V0Runner code={generatedCode} />
                {:else}
                  <div class="absolute inset-0 flex flex-col items-center justify-center text-muted-foreground gap-3">
                    <Eye class="size-10 text-muted-foreground/30" />
                    <p class="text-sm font-medium">No preview yet</p>
                    <p class="text-xs text-muted-foreground/60 max-w-xs text-center">Ask the AI agent to design something, or paste a JSON spec in the editor.</p>
                  </div>
                {/if}
              </div>

            {:else if previewMode === "split"}
              <!-- ── Split: Editor left, Preview right ── -->
              <div class="absolute inset-0 flex">
                <!-- Left half: JSON editor or Canvas -->
                <div class="flex-1 relative min-w-0 flex flex-col border-r border-border">
                  {#if isSpecMode}
                    <textarea
                      class="flex-1 w-full rounded-none border-none bg-zinc-950 p-4 font-mono text-xs text-emerald-400 placeholder:text-zinc-600 focus:outline-none resize-none leading-relaxed"
                      placeholder="Paste or edit your JSON spec here..."
                      value={specEditorText}
                      oninput={(e) => handleSpecEdit(e.currentTarget.value)}
                      spellcheck="false"
                    ></textarea>
                    {#if specParseError}
                      <div class="shrink-0 px-3 py-1.5 bg-destructive/10 border-t border-destructive/30 text-destructive text-[11px] font-mono truncate">
                        ⚠ {specParseError}
                      </div>
                    {/if}
                  {:else}
                    <div class="flex-1 relative min-w-0 flex">
                      <LayerTree
                        {document}
                        {selectedNodeId}
                        onSelectNode={(id) => selectedNodeId = id}
                        width={Math.min(layerTreeWidth, 180)}
                      />
                      <ResizeHandle onResize={(d) => layerTreeWidth = clampPanel(layerTreeWidth + d)} />
                      <div class="flex-1 relative min-w-0">
                        <CanvasEditor
                          {document}
                          {selectedNodeId}
                          bind:zoom
                          onSelectNode={(id) => selectedNodeId = id}
                          onUpdateNodePosition={updateNodePosition}
                          onUpdateNodeSize={updateNodeSize}
                        />
                      </div>
                    </div>
                  {/if}
                </div>

                <!-- Right half: Live preview -->
                <div class="flex-1 relative min-w-0">
                  {#if liveSpec}
                    <div class="w-full h-full p-6 overflow-auto bg-background">
                      <CatalogRenderer spec={liveSpec} {registry} />
                    </div>
                  {:else if generatedCode}
                    <V0Runner code={generatedCode} />
                  {:else}
                    <div class="absolute inset-0 flex flex-col items-center justify-center text-muted-foreground gap-3">
                      <Eye class="size-8 text-muted-foreground/30" />
                      <p class="text-xs">Preview will appear here</p>
                    </div>
                  {/if}
                </div>
              </div>
            {/if}
          </div>
        </div>
      {:else if activeTab === "json"}
        <div class="absolute inset-0 p-3">
          <textarea
            class="w-full h-full rounded-lg border border-border bg-muted/30 p-4 font-mono text-sm text-chart-1 placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring resize-none"
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
          <IDE bind:activeFilePath bind:fileContent onAddToChat={addContextFileFromExplorer} />
        </div>
      {/if}
    </main>

    <!-- ─── Status Bar ─── -->
    <footer class="h-7 border-t border-border flex items-center justify-between px-3 text-[11px] text-muted-foreground bg-muted/20 shrink-0">
      <div class="flex items-center gap-3">
        {#if activeTab === "preview" && isSpecMode}
          <Badge variant="secondary" class="h-4 px-1.5 text-[10px] gap-1">
            <Braces class="size-2.5" />
            json-render
          </Badge>
          {#if liveSpec}
            <span class="text-green-500">● Valid</span>
          {:else if specParseError}
            <span class="text-destructive">● Error</span>
          {/if}
          <Separator orientation="vertical" class="h-3!" />
        {:else if activeTab === "preview" && (previewMode === "preview" || previewMode === "split") && generatedCode}
          <Badge variant="secondary" class="h-4 px-1.5 text-[10px] gap-1">
            <span class="size-1.5 rounded-full bg-chart-1 animate-pulse"></span>
            v0 Runner
          </Badge>
          <Separator orientation="vertical" class="h-3!" />
        {/if}
        <span>Nodes: {document.children.length}</span>
        <Separator orientation="vertical" class="h-3!" />
        <span>Selected: {selectedNodeId ?? 'None'}</span>
      </div>
      <div class="flex items-center gap-1">
        <Button variant="ghost" size="icon-xs" class="size-5 text-muted-foreground" onclick={() => zoom = Math.max(0.1, zoom - 0.1)}>
          <Minus class="size-3" />
        </Button>
        <span class="w-9 text-center tabular-nums">{Math.round(zoom * 100)}%</span>
        <Button variant="ghost" size="icon-xs" class="size-5 text-muted-foreground" onclick={() => zoom = Math.min(5, zoom + 0.1)}>
          <Plus class="size-3" />
        </Button>
      </div>
    </footer>
  </div>

  <!-- ═══ Resize handle: Main Area <-> Chat Panel ═══ -->
  {#if showChatPanel}
    <div class="hidden lg:block h-full z-50 relative">
      <ResizeHandle onResize={(d) => chatWidth = clampPanel(chatWidth - d)} />
    </div>
  {/if}

  <!-- ═══ Right Side: Chat Panel ═══ -->
  {#if showChatPanel}
    <div class="border-l border-border bg-muted/10 shrink-0 flex-none hidden lg:flex flex-col" style="width: {chatWidth}px">
      <!-- Chat header -->
      <div class="h-11 border-b border-border flex items-center justify-between px-3 shrink-0 bg-muted/20">
        <div class="flex items-center gap-2">
          <Bot class="size-3.5 text-chart-2" />
          <span class="text-xs font-semibold">AI Agent</span>
        </div>
        <Tabs.Root bind:value={chatMode}>
          <Tabs.List class="h-6">
            <Tabs.Trigger value="scrum" class="text-[10px] px-2 h-5">Scrum</Tabs.Trigger>
            <Tabs.Trigger value="dev" class="text-[10px] px-2 h-5">Dev</Tabs.Trigger>
          </Tabs.List>
        </Tabs.Root>
      </div>

      <AgentChat {document} {chatMode} {activeFilePath} {fileContent} bind:contextFiles onApplyPatch={applyChatPatch} />
    </div>
  {/if}
</div>
