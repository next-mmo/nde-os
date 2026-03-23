<svelte:options runes={true} />

<script lang="ts">
  import * as api from "$lib/api/backend";

  // ── Editor state ─────────────────────────────────────────────────────────
  let currentFile = $state<string | null>(null);
  let fileContent = $state("");
  let language = $state("plaintext");
  let isDirty = $state(false);
  let saving = $state(false);
  let sidebarOpen = $state(true);
  let terminalOpen = $state(true);
  let terminalOutput = $state<string[]>(["$ Welcome to NDE-OS Terminal"]);
  let terminalInput = $state("");
  let fileTree = $state<FileNode[]>([]);
  let expandedDirs = $state<Set<string>>(new Set());
  let aiSuggestion = $state<string | null>(null);
  let activeTab = $state<string>("editor"); // editor | terminal | ai

  type FileNode = {
    name: string;
    path: string;
    isDir: boolean;
    children?: FileNode[];
  };

  // Language detection
  function detectLanguage(filename: string): string {
    const ext = filename.split('.').pop()?.toLowerCase() || "";
    const map: Record<string, string> = {
      rs: "rust",
      py: "python",
      js: "javascript",
      ts: "typescript",
      jsx: "javascript",
      tsx: "typescript",
      svelte: "html",
      html: "html",
      css: "css",
      json: "json",
      toml: "toml",
      yaml: "yaml",
      yml: "yaml",
      md: "markdown",
      sh: "shell",
      bash: "shell",
      sql: "sql",
    };
    return map[ext] || "plaintext";
  }

  // Load workspace file tree
  async function loadFileTree() {
    try {
      const resp = await api.agentChat("List the contents of the workspace directory in JSON format");
      // Parse the response for file listing
      fileTree = [
        { name: "core", path: "core", isDir: true, children: [
          { name: "src", path: "core/src", isDir: true, children: [
            { name: "lib.rs", path: "core/src/lib.rs", isDir: false },
            { name: "llm", path: "core/src/llm", isDir: true },
            { name: "agent", path: "core/src/agent", isDir: true },
            { name: "tools", path: "core/src/tools", isDir: true },
            { name: "plugins", path: "core/src/plugins", isDir: true },
            { name: "mcp", path: "core/src/mcp", isDir: true },
          ]},
          { name: "Cargo.toml", path: "core/Cargo.toml", isDir: false },
        ]},
        { name: "server", path: "server", isDir: true, children: [
          { name: "src", path: "server/src", isDir: true },
          { name: "Cargo.toml", path: "server/Cargo.toml", isDir: false },
        ]},
        { name: "cli", path: "cli", isDir: true, children: [
          { name: "src", path: "cli/src", isDir: true },
          { name: "Cargo.toml", path: "cli/Cargo.toml", isDir: false },
        ]},
        { name: "desktop", path: "desktop", isDir: true },
        { name: "Cargo.toml", path: "Cargo.toml", isDir: false },
      ];
    } catch {
      // Use default tree
    }
  }

  // Mount — load file tree
  $effect(() => {
    loadFileTree();
  });

  // Open a file
  async function openFile(node: FileNode) {
    if (node.isDir) {
      const key = node.path;
      if (expandedDirs.has(key)) {
        expandedDirs.delete(key);
      } else {
        expandedDirs.add(key);
      }
      expandedDirs = new Set(expandedDirs); // trigger reactivity
      return;
    }

    try {
      const resp = await api.agentChat(`Read the file at path: ${node.path}`);
      currentFile = node.path;
      fileContent = resp.response || "";
      language = detectLanguage(node.name);
      isDirty = false;
    } catch (err: any) {
      console.error("Failed to open file:", err);
    }
  }

  // Save file
  async function saveFile() {
    if (!currentFile || !isDirty) return;
    saving = true;
    try {
      await api.agentChat(`Write the following content to file ${currentFile}:\n\`\`\`\n${fileContent}\n\`\`\``);
      isDirty = false;
    } catch (err: any) {
      console.error("Save failed:", err);
    } finally {
      saving = false;
    }
  }

  // Run terminal command
  async function runCommand() {
    const cmd = terminalInput.trim();
    if (!cmd) return;

    terminalOutput = [...terminalOutput, `$ ${cmd}`];
    terminalInput = "";

    try {
      const resp = await api.agentChat(`Run this shell command: ${cmd}`);
      terminalOutput = [...terminalOutput, resp.response || "(no output)"];
    } catch (err: any) {
      terminalOutput = [...terminalOutput, `Error: ${err.message}`];
    }
  }

  // AI code assist
  async function askAI() {
    const selection = fileContent.substring(0, 200);
    try {
      const resp = await api.agentChat(
        `I'm editing ${currentFile}. Help me with this code:\n\`\`\`${language}\n${selection}\n\`\`\``
      );
      aiSuggestion = resp.response;
    } catch {
      aiSuggestion = "AI assist unavailable";
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    // Ctrl+S to save
    if (e.ctrlKey && e.key === "s") {
      e.preventDefault();
      saveFile();
    }
    // Ctrl+` to toggle terminal
    if (e.ctrlKey && e.key === "`") {
      e.preventDefault();
      terminalOpen = !terminalOpen;
    }
  }

  function handleTerminalKey(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      runCommand();
    }
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

<section class="ide-app">
  <!-- Sidebar / File Explorer -->
  {#if sidebarOpen}
    <aside class="explorer">
      <div class="explorer-header">
        <h3>Explorer</h3>
        <button class="icon-btn" onclick={() => loadFileTree()}>↻</button>
      </div>
      <div class="file-tree">
        {#each fileTree as node (node.path)}
          {@render fileTreeNode(node, 0)}
        {/each}
      </div>
    </aside>
  {/if}

  <!-- Main editor area -->
  <div class="editor-main">
    <!-- Tab bar -->
    <div class="tab-bar">
      <button class="tab-toggle" onclick={() => sidebarOpen = !sidebarOpen}>
        {sidebarOpen ? '◀' : '▶'}
      </button>
      {#if currentFile}
        <div class="tab active">
          <span class="tab-name">{currentFile.split('/').pop()}</span>
          {#if isDirty}
            <span class="tab-dirty">●</span>
          {/if}
        </div>
      {:else}
        <div class="tab placeholder-tab">No file open</div>
      {/if}
      <div class="tab-actions">
        <button class="action-btn" onclick={saveFile} disabled={!isDirty || saving}>
          {saving ? '...' : '💾'}
        </button>
        <button class="action-btn" class:active={terminalOpen} onclick={() => terminalOpen = !terminalOpen}>
          ⌨
        </button>
        <button class="action-btn ai-btn" onclick={askAI}>
          🤖 AI
        </button>
      </div>
    </div>

    <!-- Code editor (textarea-based; Monaco integration in phase 3) -->
    <div class="editor-container">
      {#if currentFile}
        <div class="line-numbers">
          {#each fileContent.split('\n') as _, i}
            <div class="line-num">{i + 1}</div>
          {/each}
        </div>
        <textarea
          class="code-editor"
          bind:value={fileContent}
          oninput={() => isDirty = true}
          spellcheck="false"
          data-language={language}
        ></textarea>
      {:else}
        <div class="welcome-screen">
          <div class="welcome-icon">📝</div>
          <h2>NDE-OS Code Editor</h2>
          <p>Open a file from the explorer to start editing</p>
          <div class="shortcuts">
            <div class="shortcut"><kbd>Ctrl+S</kbd> Save</div>
            <div class="shortcut"><kbd>Ctrl+`</kbd> Toggle Terminal</div>
          </div>
        </div>
      {/if}
    </div>

    <!-- AI suggestion panel -->
    {#if aiSuggestion}
      <div class="ai-panel">
        <div class="ai-panel-header">
          <span>🤖 AI Assistant</span>
          <button class="close-btn" onclick={() => aiSuggestion = null}>✕</button>
        </div>
        <div class="ai-content">{aiSuggestion}</div>
      </div>
    {/if}

    <!-- Terminal -->
    {#if terminalOpen}
      <div class="terminal-panel">
        <div class="terminal-header">
          <span>Terminal</span>
          <button class="close-btn" onclick={() => terminalOpen = false}>✕</button>
        </div>
        <div class="terminal-output">
          {#each terminalOutput as line}
            <div class="term-line">{line}</div>
          {/each}
        </div>
        <div class="terminal-input-row">
          <span class="prompt">$</span>
          <input
            class="term-input"
            bind:value={terminalInput}
            onkeydown={handleTerminalKey}
            placeholder="Enter command..."
          />
        </div>
      </div>
    {/if}

    <!-- Status bar -->
    <div class="status-bar">
      <span>{language}</span>
      <span>·</span>
      <span>{currentFile ? `${fileContent.split('\n').length} lines` : 'No file'}</span>
      {#if isDirty}
        <span class="dirty-indicator">● Unsaved</span>
      {/if}
      <span class="status-right">NDE-OS IDE</span>
    </div>
  </div>
</section>

{#snippet fileTreeNode(node: FileNode, depth: number)}
  <button
    class="tree-item"
    class:is-dir={node.isDir}
    class:is-open={expandedDirs.has(node.path)}
    class:is-active={currentFile === node.path}
    style="padding-left: {12 + depth * 16}px"
    onclick={() => openFile(node)}
  >
    <span class="tree-icon">
      {#if node.isDir}
        {expandedDirs.has(node.path) ? '📂' : '📁'}
      {:else}
        {language === 'rust' ? '🦀' : language === 'python' ? '🐍' : '📄'}
      {/if}
    </span>
    <span class="tree-name">{node.name}</span>
  </button>
  {#if node.isDir && expandedDirs.has(node.path) && node.children}
    {#each node.children as child (child.path)}
      {@render fileTreeNode(child, depth + 1)}
    {/each}
  {/if}
{/snippet}

<style>
  .ide-app {
    height: 100%;
    display: flex;
    overflow: hidden;
    background: hsl(220 16% 8%);
    color: #d4d4d4;
    font-family: 'Menlo', 'Consolas', 'Courier New', monospace;
    font-size: 13px;
  }

  /* Explorer */
  .explorer {
    width: 240px;
    border-right: 1px solid hsla(0 0% 100% / 0.08);
    background: hsl(220 18% 10%);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }

  .explorer-header {
    padding: 0.6rem 0.8rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    text-transform: uppercase;
    font-size: 0.7rem;
    letter-spacing: 0.08em;
    color: hsla(0 0% 100% / 0.5);
    border-bottom: 1px solid hsla(0 0% 100% / 0.06);
  }

  .explorer-header h3 { margin: 0; font-weight: 600; }

  .icon-btn {
    background: none;
    border: none;
    color: hsla(0 0% 100% / 0.5);
    cursor: pointer;
    font-size: 0.85rem;
  }
  .icon-btn:hover { color: white; }

  .file-tree {
    overflow-y: auto;
    flex: 1;
    padding: 4px 0;
  }

  .tree-item {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    border: none;
    background: transparent;
    color: #cccccc;
    font-size: 0.82rem;
    padding: 3px 12px;
    cursor: pointer;
    text-align: left;
    font-family: inherit;
  }

  .tree-item:hover { background: hsla(0 0% 100% / 0.05); }
  .tree-item.is-active { background: hsla(220 80% 55% / 0.2); color: white; }

  .tree-icon { font-size: 0.9rem; width: 18px; text-align: center; }
  .tree-name { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

  /* Editor main */
  .editor-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  /* Tab bar */
  .tab-bar {
    display: flex;
    align-items: center;
    background: hsl(220 16% 11%);
    border-bottom: 1px solid hsla(0 0% 100% / 0.08);
    height: 36px;
    flex-shrink: 0;
  }

  .tab-toggle {
    padding: 0 8px;
    background: none;
    border: none;
    border-right: 1px solid hsla(0 0% 100% / 0.08);
    color: hsla(0 0% 100% / 0.4);
    cursor: pointer;
    height: 100%;
    font-size: 0.7rem;
  }

  .tab {
    padding: 0 12px;
    height: 100%;
    display: flex;
    align-items: center;
    gap: 6px;
    background: hsl(220 16% 8%);
    border-right: 1px solid hsla(0 0% 100% / 0.08);
    font-size: 0.8rem;
  }

  .tab.active { color: white; }
  .tab.placeholder-tab { color: hsla(0 0% 100% / 0.3); }
  .tab-dirty { color: hsl(40 80% 60%); font-size: 0.6rem; }

  .tab-actions {
    margin-left: auto;
    display: flex;
    padding-right: 8px;
    gap: 4px;
  }

  .action-btn {
    padding: 4px 8px;
    background: none;
    border: 1px solid transparent;
    border-radius: 4px;
    color: hsla(0 0% 100% / 0.5);
    cursor: pointer;
    font-size: 0.8rem;
  }

  .action-btn:hover { background: hsla(0 0% 100% / 0.08); color: white; }
  .action-btn:disabled { opacity: 0.3; cursor: not-allowed; }
  .action-btn.active { color: hsl(220 80% 65%); }
  .ai-btn { color: hsl(280 80% 65%); }

  /* Code editor */
  .editor-container {
    flex: 1;
    display: flex;
    overflow: auto;
    position: relative;
  }

  .line-numbers {
    width: 48px;
    background: hsl(220 16% 9%);
    padding: 4px 0;
    text-align: right;
    color: hsla(0 0% 100% / 0.25);
    user-select: none;
    font-size: 12px;
    line-height: 1.5;
    flex-shrink: 0;
  }

  .line-num {
    padding-right: 12px;
    height: 18px;
  }

  .code-editor {
    flex: 1;
    background: transparent;
    color: #d4d4d4;
    border: none;
    outline: none;
    resize: none;
    font-family: inherit;
    font-size: 13px;
    line-height: 1.5;
    padding: 4px 16px;
    tab-size: 4;
    white-space: pre;
    overflow-wrap: normal;
    overflow-x: auto;
  }

  /* Welcome screen */
  .welcome-screen {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: hsla(0 0% 100% / 0.35);
    gap: 8px;
  }

  .welcome-icon { font-size: 3rem; }
  .welcome-screen h2 { margin: 0; color: hsla(0 0% 100% / 0.5); font-size: 1.2rem; }
  .welcome-screen p { margin: 0; font-size: 0.85rem; }

  .shortcuts {
    display: flex;
    gap: 24px;
    margin-top: 16px;
    font-size: 0.78rem;
  }

  .shortcut { display: flex; align-items: center; gap: 8px; }

  kbd {
    padding: 2px 6px;
    border-radius: 4px;
    background: hsla(0 0% 100% / 0.08);
    border: 1px solid hsla(0 0% 100% / 0.12);
    font-family: inherit;
    font-size: 0.75rem;
  }

  /* AI panel */
  .ai-panel {
    border-top: 1px solid hsla(280 80% 55% / 0.3);
    background: hsla(280 30% 12% / 0.9);
    max-height: 200px;
    overflow-y: auto;
  }

  .ai-panel-header {
    padding: 6px 12px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.78rem;
    color: hsl(280 80% 70%);
    border-bottom: 1px solid hsla(280 80% 55% / 0.15);
  }

  .ai-content {
    padding: 8px 12px;
    font-size: 0.82rem;
    white-space: pre-wrap;
    line-height: 1.5;
  }

  .close-btn {
    background: none;
    border: none;
    color: hsla(0 0% 100% / 0.4);
    cursor: pointer;
    font-size: 0.8rem;
  }

  /* Terminal */
  .terminal-panel {
    height: 200px;
    border-top: 1px solid hsla(0 0% 100% / 0.08);
    background: hsl(220 20% 6%);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }

  .terminal-header {
    padding: 4px 12px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.75rem;
    color: hsla(0 0% 100% / 0.5);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    border-bottom: 1px solid hsla(0 0% 100% / 0.06);
  }

  .terminal-output {
    flex: 1;
    overflow-y: auto;
    padding: 8px 12px;
    font-size: 0.8rem;
    color: hsl(120 60% 70%);
    line-height: 1.5;
  }

  .term-line {
    white-space: pre-wrap;
    word-break: break-all;
  }

  .terminal-input-row {
    display: flex;
    align-items: center;
    padding: 4px 12px;
    border-top: 1px solid hsla(0 0% 100% / 0.06);
    gap: 8px;
  }

  .prompt { color: hsl(120 60% 60%); font-weight: bold; }

  .term-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: #d4d4d4;
    font-family: inherit;
    font-size: 0.8rem;
  }

  /* Status bar */
  .status-bar {
    height: 22px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    background: hsl(220 80% 45%);
    color: white;
    font-size: 0.7rem;
    flex-shrink: 0;
  }

  .dirty-indicator { color: hsl(40 90% 65%); }
  .status-right { margin-left: auto; }
</style>
