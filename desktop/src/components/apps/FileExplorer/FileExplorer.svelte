<svelte:options runes={true} />

<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  interface FileEntry {
    name: string;
    path: string;
    is_dir: boolean;
    size: number;
    modified: string | null;
  }

  let currentPath = $state("");
  let entries = $state<FileEntry[]>([]);
  let loading = $state(false);
  let error = $state("");
  let history = $state<string[]>([]);
  let historyIndex = $state(-1);
  let viewMode = $state<"list" | "grid">("list");
  let selectedPath = $state<string | null>(null);
  let renamingPath = $state<string | null>(null);
  let renameValue = $state("");
  let showNewFolderInput = $state(false);
  let newFolderName = $state("");

  // Sidebar quick-access folders
  const sidebarItems = [
    { label: "Home", icon: "🏠", path: "" },
    { label: "Desktop", icon: "🖥️", path: "Desktop" },
    { label: "Downloads", icon: "⬇️", path: "Downloads" },
    { label: "Documents", icon: "📄", path: "Documents" },
    { label: "Pictures", icon: "🖼️", path: "Pictures" },
    { label: "Music", icon: "🎵", path: "Music" },
    { label: "Videos", icon: "🎬", path: "Videos" },
  ];

  let homeDir = $state("");

  async function init() {
    try {
      homeDir = await invoke<string>("get_home_dir");
      await navigate(homeDir);
    } catch (e) {
      error = String(e);
    }
  }

  async function navigate(path: string) {
    loading = true;
    error = "";
    selectedPath = null;
    renamingPath = null;
    showNewFolderInput = false;
    try {
      const result = await invoke<FileEntry[]>("list_directory", { path });
      entries = result;
      currentPath = path || homeDir;

      // Update history
      if (historyIndex < history.length - 1) {
        history = history.slice(0, historyIndex + 1);
      }
      history = [...history, currentPath];
      historyIndex = history.length - 1;
    } catch (e) {
      error = String(e);
    }
    loading = false;
  }

  function goBack() {
    if (historyIndex > 0) {
      historyIndex -= 1;
      loadPath(history[historyIndex]);
    }
  }

  function goForward() {
    if (historyIndex < history.length - 1) {
      historyIndex += 1;
      loadPath(history[historyIndex]);
    }
  }

  async function loadPath(path: string) {
    loading = true;
    error = "";
    selectedPath = null;
    try {
      entries = await invoke<FileEntry[]>("list_directory", { path });
      currentPath = path;
    } catch (e) {
      error = String(e);
    }
    loading = false;
  }

  function goUp() {
    const sep = currentPath.includes("\\") ? "\\" : "/";
    const parts = currentPath.split(sep).filter(Boolean);
    if (parts.length <= 1) {
      // On Windows, going up from C:\Users → C:\
      if (currentPath.match(/^[A-Z]:\\/i)) {
        navigate(currentPath.slice(0, 3));
      }
      return;
    }
    parts.pop();
    const parent = currentPath.startsWith("/")
      ? "/" + parts.join("/")
      : parts.join(sep);
    navigate(parent);
  }

  async function handleEntryClick(entry: FileEntry) {
    selectedPath = entry.path;
  }

  async function handleEntryDblClick(entry: FileEntry) {
    if (entry.is_dir) {
      await navigate(entry.path);
    } else {
      try {
        await invoke("open_file", { path: entry.path });
      } catch (e) {
        error = String(e);
      }
    }
  }

  async function handleCreateFolder() {
    if (!newFolderName.trim()) return;
    const sep = currentPath.includes("\\") ? "\\" : "/";
    const folderPath = currentPath + sep + newFolderName.trim();
    try {
      await invoke("create_folder", { path: folderPath });
      showNewFolderInput = false;
      newFolderName = "";
      await loadPath(currentPath);
    } catch (e) {
      error = String(e);
    }
  }

  async function handleDelete() {
    if (!selectedPath) return;
    try {
      await invoke("delete_entry", { path: selectedPath });
      selectedPath = null;
      await loadPath(currentPath);
    } catch (e) {
      error = String(e);
    }
  }

  function startRename(entry: FileEntry) {
    renamingPath = entry.path;
    renameValue = entry.name;
  }

  async function handleRename(entry: FileEntry) {
    if (!renameValue.trim() || renameValue === entry.name) {
      renamingPath = null;
      return;
    }
    const sep = currentPath.includes("\\") ? "\\" : "/";
    const newPath = currentPath + sep + renameValue.trim();
    try {
      await invoke("rename_entry", { oldPath: entry.path, newPath });
      renamingPath = null;
      await loadPath(currentPath);
    } catch (e) {
      error = String(e);
    }
  }

  function formatSize(bytes: number): string {
    if (bytes === 0) return "—";
    const units = ["B", "KB", "MB", "GB", "TB"];
    let i = 0;
    let size = bytes;
    while (size >= 1024 && i < units.length - 1) {
      size /= 1024;
      i++;
    }
    return `${size.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
  }

  function fileIcon(entry: FileEntry): string {
    if (entry.is_dir) return "📁";
    const ext = entry.name.split(".").pop()?.toLowerCase() ?? "";
    const map: Record<string, string> = {
      png: "🖼️", jpg: "🖼️", jpeg: "🖼️", gif: "🖼️", webp: "🖼️", svg: "🖼️", ico: "🖼️",
      mp3: "🎵", wav: "🎵", flac: "🎵", aac: "🎵", ogg: "🎵",
      mp4: "🎬", mov: "🎬", avi: "🎬", mkv: "🎬", webm: "🎬",
      pdf: "📕", doc: "📝", docx: "📝", txt: "📝", md: "📝",
      zip: "📦", tar: "📦", gz: "📦", rar: "📦", "7z": "📦",
      rs: "🦀", py: "🐍", js: "📜", ts: "📜", json: "📋", toml: "📋", yaml: "📋", yml: "📋",
      exe: "⚙️", msi: "⚙️", sh: "⚙️", bat: "⚙️",
      html: "🌐", css: "🎨", svelte: "🔶",
    };
    return map[ext] || "📄";
  }

  // Breadcrumb segments
  const breadcrumbs = $derived.by(() => {
    if (!currentPath) return [];
    const sep = currentPath.includes("\\") ? "\\" : "/";
    const parts = currentPath.split(sep).filter(Boolean);
    const crumbs: { label: string; path: string }[] = [];
    for (let i = 0; i < parts.length; i++) {
      const path = currentPath.startsWith("/")
        ? "/" + parts.slice(0, i + 1).join("/")
        : parts.slice(0, i + 1).join(sep);
      crumbs.push({ label: parts[i], path });
    }
    return crumbs;
  });

  function handleSidebar(item: { path: string }) {
    if (item.path === "") {
      navigate(homeDir);
    } else {
      const sep = homeDir.includes("\\") ? "\\" : "/";
      navigate(homeDir + sep + item.path);
    }
  }

  // Init on mount
  init();
</script>

<div class="flex w-full h-full overflow-hidden text-sm">
  <!-- Sidebar -->
  <aside class="w-48 shrink-0 border-r border-black/8 dark:border-white/8 bg-white/30 dark:bg-gray-900/30 backdrop-blur-md flex flex-col overflow-y-auto">
    <div class="px-3 pt-3 pb-1.5">
      <span class="text-[0.65rem] uppercase tracking-widest font-semibold text-gray-400 dark:text-gray-500">Favorites</span>
    </div>
    {#each sidebarItems as item}
      <button
        class="flex items-center gap-2 px-3 py-1.5 text-left text-[0.82rem] font-medium rounded-lg mx-1.5 transition-colors duration-150 hover:bg-black/5 dark:hover:bg-white/5 {currentPath.endsWith(item.path) || (item.path === '' && currentPath === homeDir) ? 'bg-blue-500/12 text-blue-600 dark:text-blue-400' : 'text-gray-700 dark:text-gray-300'}"
        onclick={() => handleSidebar(item)}
      >
        <span class="text-base">{item.icon}</span>
        {item.label}
      </button>
    {/each}
  </aside>

  <!-- Main content -->
  <div class="flex-1 flex flex-col min-w-0 overflow-hidden">
    <!-- Toolbar -->
    <div class="flex items-center gap-1.5 px-3 py-2 border-b border-black/8 dark:border-white/8 bg-white/40 dark:bg-gray-800/40 backdrop-blur-sm">
      <!-- Nav buttons -->
      <button
        class="p-1.5 rounded-md transition-colors hover:bg-black/8 dark:hover:bg-white/8 disabled:opacity-30 disabled:cursor-default text-gray-600 dark:text-gray-400"
        disabled={historyIndex <= 0}
        onclick={goBack}
        aria-label="Go back"
      >
        <svg class="w-4 h-4" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M12.707 5.293a1 1 0 010 1.414L9.414 10l3.293 3.293a1 1 0 01-1.414 1.414l-4-4a1 1 0 010-1.414l4-4a1 1 0 011.414 0z" clip-rule="evenodd"/></svg>
      </button>
      <button
        class="p-1.5 rounded-md transition-colors hover:bg-black/8 dark:hover:bg-white/8 disabled:opacity-30 disabled:cursor-default text-gray-600 dark:text-gray-400"
        disabled={historyIndex >= history.length - 1}
        onclick={goForward}
        aria-label="Go forward"
      >
        <svg class="w-4 h-4" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M7.293 14.707a1 1 0 010-1.414L10.586 10 7.293 6.707a1 1 0 011.414-1.414l4 4a1 1 0 010 1.414l-4 4a1 1 0 01-1.414 0z" clip-rule="evenodd"/></svg>
      </button>
      <button
        class="p-1.5 rounded-md transition-colors hover:bg-black/8 dark:hover:bg-white/8 text-gray-600 dark:text-gray-400"
        onclick={goUp}
        aria-label="Go up"
      >
        <svg class="w-4 h-4" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M14.707 12.707a1 1 0 01-1.414 0L10 9.414l-3.293 3.293a1 1 0 01-1.414-1.414l4-4a1 1 0 011.414 0l4 4a1 1 0 010 1.414z" clip-rule="evenodd"/></svg>
      </button>

      <!-- Breadcrumbs -->
      <div class="flex-1 flex items-center gap-0.5 overflow-x-auto min-w-0 px-2 py-1 rounded-lg bg-black/4 dark:bg-white/4">
        {#each breadcrumbs as crumb, i}
          {#if i > 0}
            <svg class="w-3 h-3 text-gray-400 shrink-0" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M7.293 14.707a1 1 0 010-1.414L10.586 10 7.293 6.707a1 1 0 011.414-1.414l4 4a1 1 0 010 1.414l-4 4a1 1 0 01-1.414 0z" clip-rule="evenodd"/></svg>
          {/if}
          <button
            class="text-[0.78rem] px-1 py-0.5 rounded font-medium whitespace-nowrap transition-colors hover:bg-black/6 dark:hover:bg-white/6 {i === breadcrumbs.length - 1 ? 'text-gray-900 dark:text-gray-100' : 'text-gray-500 dark:text-gray-400'}"
            onclick={() => navigate(crumb.path)}
          >
            {crumb.label}
          </button>
        {/each}
      </div>

      <!-- Actions -->
      <div class="flex items-center gap-0.5">
        <button
          class="p-1.5 rounded-md transition-colors hover:bg-black/8 dark:hover:bg-white/8 text-gray-600 dark:text-gray-400"
          onclick={() => { showNewFolderInput = !showNewFolderInput; newFolderName = ""; }}
          aria-label="New folder"
          title="New Folder"
        >
          <svg class="w-4 h-4" viewBox="0 0 20 20" fill="currentColor"><path d="M4 4a2 2 0 00-2 2v8a2 2 0 002 2h12a2 2 0 002-2V8a2 2 0 00-2-2h-5L9 4H4z"/></svg>
        </button>
        <button
          class="p-1.5 rounded-md transition-colors hover:bg-black/8 dark:hover:bg-white/8 disabled:opacity-30 disabled:cursor-default text-red-500 dark:text-red-400"
          disabled={!selectedPath}
          onclick={handleDelete}
          aria-label="Delete"
          title="Delete"
        >
          <svg class="w-4 h-4" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M9 2a1 1 0 00-.894.553L7.382 4H4a1 1 0 000 2v10a2 2 0 002 2h8a2 2 0 002-2V6a1 1 0 100-2h-3.382l-.724-1.447A1 1 0 0011 2H9zM7 8a1 1 0 012 0v6a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v6a1 1 0 102 0V8a1 1 0 00-1-1z" clip-rule="evenodd"/></svg>
        </button>
        <!-- View toggle -->
        <button
          class="p-1.5 rounded-md transition-colors hover:bg-black/8 dark:hover:bg-white/8 text-gray-600 dark:text-gray-400"
          onclick={() => viewMode = viewMode === "list" ? "grid" : "list"}
          aria-label="Toggle view"
          title={viewMode === "list" ? "Grid view" : "List view"}
        >
          {#if viewMode === "list"}
            <svg class="w-4 h-4" viewBox="0 0 20 20" fill="currentColor"><path d="M5 3a2 2 0 00-2 2v2a2 2 0 002 2h2a2 2 0 002-2V5a2 2 0 00-2-2H5zM5 11a2 2 0 00-2 2v2a2 2 0 002 2h2a2 2 0 002-2v-2a2 2 0 00-2-2H5zM11 5a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V5zM11 13a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z"/></svg>
          {:else}
            <svg class="w-4 h-4" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M3 4a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zm0 4a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zm0 4a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zm0 4a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1z" clip-rule="evenodd"/></svg>
          {/if}
        </button>
      </div>
    </div>

    <!-- New folder input -->
    {#if showNewFolderInput}
      <div class="flex items-center gap-2 px-4 py-2 bg-blue-50/80 dark:bg-blue-900/20 border-b border-blue-200/40 dark:border-blue-700/30">
        <span class="text-base">📁</span>
        <input
          type="text"
          class="flex-1 text-sm px-2 py-1 rounded-md border border-black/10 dark:border-white/10 bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 outline-none focus:ring-2 focus:ring-blue-500/40"
          placeholder="New folder name…"
          bind:value={newFolderName}
          onkeydown={(e) => { if (e.key === "Enter") handleCreateFolder(); if (e.key === "Escape") { showNewFolderInput = false; } }}
        />
        <button
          class="px-3 py-1 text-xs font-medium rounded-md bg-blue-500 text-white hover:bg-blue-600 transition-colors"
          onclick={handleCreateFolder}
        >Create</button>
        <button
          class="px-3 py-1 text-xs font-medium rounded-md bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
          onclick={() => { showNewFolderInput = false; }}
        >Cancel</button>
      </div>
    {/if}

    <!-- Error -->
    {#if error}
      <div class="px-4 py-2 text-xs text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-900/20 border-b border-red-200/40 dark:border-red-700/30">
        {error}
      </div>
    {/if}

    <!-- File list -->
    <div class="flex-1 overflow-auto">
      {#if loading}
        <div class="flex items-center justify-center h-full text-gray-400 dark:text-gray-500">
          <svg class="w-5 h-5 animate-spin mr-2" viewBox="0 0 24 24" fill="none"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/></svg>
          Loading…
        </div>
      {:else if viewMode === "list"}
        <!-- List view -->
        <table class="w-full text-[0.78rem]">
          <thead>
            <tr class="border-b border-black/6 dark:border-white/6 text-left text-gray-500 dark:text-gray-400">
              <th class="px-4 py-2 font-medium">Name</th>
              <th class="px-4 py-2 font-medium w-24">Size</th>
              <th class="px-4 py-2 font-medium w-40">Modified</th>
            </tr>
          </thead>
          <tbody>
            {#each entries as entry (entry.path)}
              <tr
                class="border-b border-black/3 dark:border-white/3 cursor-default transition-colors {selectedPath === entry.path ? 'bg-blue-500/12 dark:bg-blue-500/15' : 'hover:bg-black/3 dark:hover:bg-white/3'}"
                onclick={() => handleEntryClick(entry)}
                ondblclick={() => handleEntryDblClick(entry)}
                tabindex="0"
                onkeydown={(e) => { if (e.key === "Enter") handleEntryDblClick(entry); if (e.key === "F2") startRename(entry); }}
              >
                <td class="px-4 py-1.5">
                  <div class="flex items-center gap-2">
                    <span class="text-base">{fileIcon(entry)}</span>
                    {#if renamingPath === entry.path}
                      <input
                        type="text"
                        class="flex-1 text-sm px-1.5 py-0.5 rounded border border-blue-400 bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 outline-none"
                        bind:value={renameValue}
                        onkeydown={(e) => { if (e.key === "Enter") handleRename(entry); if (e.key === "Escape") { renamingPath = null; } }}
                        onblur={() => handleRename(entry)}
                      />
                    {:else}
                      <span class="text-gray-800 dark:text-gray-200 truncate">{entry.name}</span>
                    {/if}
                  </div>
                </td>
                <td class="px-4 py-1.5 text-gray-500 dark:text-gray-400 tabular-nums">
                  {entry.is_dir ? "—" : formatSize(entry.size)}
                </td>
                <td class="px-4 py-1.5 text-gray-500 dark:text-gray-400 tabular-nums">
                  {entry.modified ?? "—"}
                </td>
              </tr>
            {/each}
            {#if entries.length === 0 && !loading}
              <tr>
                <td colspan="3" class="px-4 py-12 text-center text-gray-400 dark:text-gray-500">
                  This folder is empty
                </td>
              </tr>
            {/if}
          </tbody>
        </table>
      {:else}
        <!-- Grid view -->
        <div class="grid grid-cols-[repeat(auto-fill,minmax(96px,1fr))] gap-1 p-4">
          {#each entries as entry (entry.path)}
            <button
              class="flex flex-col items-center gap-1 p-2 rounded-xl transition-colors cursor-default {selectedPath === entry.path ? 'bg-blue-500/12 dark:bg-blue-500/15 ring-1 ring-blue-400/30' : 'hover:bg-black/4 dark:hover:bg-white/4'}"
              onclick={() => handleEntryClick(entry)}
              ondblclick={() => handleEntryDblClick(entry)}
              onkeydown={(e) => { if (e.key === "Enter") handleEntryDblClick(entry); }}
            >
              <span class="text-3xl">{fileIcon(entry)}</span>
              <span class="text-[0.7rem] text-gray-700 dark:text-gray-300 text-center leading-tight line-clamp-2 w-full break-all">{entry.name}</span>
            </button>
          {/each}
          {#if entries.length === 0 && !loading}
            <div class="col-span-full text-center py-12 text-gray-400 dark:text-gray-500">
              This folder is empty
            </div>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Status bar -->
    <div class="flex items-center justify-between px-4 py-1.5 text-[0.7rem] text-gray-500 dark:text-gray-400 border-t border-black/6 dark:border-white/6 bg-white/30 dark:bg-gray-800/30">
      <span>{entries.length} item{entries.length !== 1 ? "s" : ""}</span>
      <span class="truncate max-w-[60%] text-right">{currentPath}</span>
    </div>
  </div>
</div>
