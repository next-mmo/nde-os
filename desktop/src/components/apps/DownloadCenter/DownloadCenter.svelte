<svelte:options runes={true} />

<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { openStaticApp } from "🍎/state/desktop.svelte";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { ScrollArea } from "$lib/components/ui/scroll-area";
  import { Progress } from "$lib/components/ui/progress";
  import { Badge } from "$lib/components/ui/badge";
  import {
    Loader2, Play, Pause, X, Trash2, Download, Globe,
    FolderOpen, Clock, CheckCircle, XCircle, Film,
  } from "@lucide/svelte";

  // ── State ──────────────────────────────────────────────────────────────
  let resolveUrl = $state("");
  let isResolving = $state(false);
  let resolvedPlaylist = $state<any>(null);
  let selectedItems = $state<Set<string>>(new Set());
  let errorMsg = $state("");

  let providers = $state<any[]>([]);
  let jobs = $state<any[]>([]);
  let pollInterval: ReturnType<typeof setInterval>;

  /** "downloads" | "gallery" */
  let activeTab = $state<"downloads" | "gallery">("downloads");

  // ── Derived: split jobs into active vs history ─────────────────────────
  const activeJobs = $derived(
    jobs.filter((j: any) => ["queued", "running", "paused"].includes(j.status))
  );
  const historyJobs = $derived(
    jobs
      .filter((j: any) => ["completed", "failed", "cancelled"].includes(j.status))
      .sort((a: any, b: any) => b.updated_at - a.updated_at)
  );

  // ── API helpers ────────────────────────────────────────────────────────
  async function resolveUrlHandle() {
    if (!resolveUrl.trim()) return;
    isResolving = true;
    try {
      const res = await fetch(`http://localhost:8080/api/downloads/resolve`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ url: resolveUrl }),
      });
      const data = await res.json();
      if (data.success) {
        resolvedPlaylist = data.data;
        selectedItems = new Set(data.data.items.map((i: any) => i.id));
        errorMsg = "";
      } else {
        errorMsg = data.message || "Unknown error";
      }
    } catch (e: any) {
      errorMsg = "Error: " + e.message;
    } finally {
      isResolving = false;
    }
  }

  function toggleSelection(id: string) {
    const next = new Set(selectedItems);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    selectedItems = next;
  }

  function selectAll() {
    if (!resolvedPlaylist) return;
    if (selectedItems.size === resolvedPlaylist.items.length) {
      selectedItems = new Set();
    } else {
      selectedItems = new Set(resolvedPlaylist.items.map((i: any) => i.id));
    }
  }

  async function startDownload() {
    if (!resolvedPlaylist || selectedItems.size === 0) return;
    try {
      const res = await fetch(`http://localhost:8080/api/downloads`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          url: resolveUrl,
          item_ids: Array.from(selectedItems)
        }),
      });
      const data = await res.json();
      if (data.success) {
        resolvedPlaylist = null;
        resolveUrl = "";
        activeTab = "downloads";
        fetchJobs();
      } else {
        alert(data.message);
      }
    } catch (e: any) {
      alert("Error: " + e.message);
    }
  }

  async function fetchJobs() {
    try {
      const res = await fetch("http://localhost:8080/api/downloads");
      const data = await res.json();
      if (data.success) {
        jobs = data.data;
      }
    } catch (e) {
      console.error("Failed to fetch jobs", e);
    }
  }

  async function actionJob(id: string, action: string) {
    try {
      const isDelete = action === "delete";
      const url = isDelete
        ? `http://localhost:8080/api/downloads/${id}`
        : `http://localhost:8080/api/downloads/${id}/${action}`;
      await fetch(url, { method: isDelete ? "DELETE" : "POST" });
      fetchJobs();
    } catch (e) {
      console.error(`Failed to ${action} job`, e);
    }
  }

  function openFolder(dirPath: string) {
    openStaticApp("file-explorer", { path: dirPath });
  }

  async function fetchProviders() {
    try {
      const res = await fetch("http://localhost:8080/api/downloads/providers");
      const data = await res.json();
      if (data.success) providers = data.data;
    } catch (e) {
      console.error("Failed to fetch providers", e);
    }
  }

  onMount(() => {
    fetchProviders();
    fetchJobs();
    pollInterval = setInterval(fetchJobs, 1000);
  });

  onDestroy(() => {
    if (pollInterval) clearInterval(pollInterval);
  });

  function getJobProgress(job: any) {
    if (!job.items || job.items.length === 0) return 0;
    const completed = job.items.filter((i: any) => i.status === "completed" || i.status === "skipped").length;
    return (completed / job.items.length) * 100;
  }

  function formatDate(epoch: number): string {
    const d = new Date(epoch * 1000);
    const now = new Date();
    const diff = now.getTime() - d.getTime();
    const mins = Math.floor(diff / 60000);
    if (mins < 1) return "Just now";
    if (mins < 60) return `${mins}m ago`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    if (days < 7) return `${days}d ago`;
    return d.toLocaleDateString();
  }

  function getTotalSize(job: any): string {
    const total = job.items.reduce((sum: number, it: any) => sum + (it.bytes_downloaded || 0), 0);
    if (total === 0) return "";
    if (total < 1024 * 1024) return `${(total / 1024).toFixed(0)} KB`;
    if (total < 1024 * 1024 * 1024) return `${(total / (1024 * 1024)).toFixed(1)} MB`;
    return `${(total / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }
</script>

<div class="h-full w-full flex flex-col bg-background text-foreground overflow-hidden">
  <!-- Header: URL input + providers -->
  <div class="p-4 border-b border-border bg-card flex flex-col gap-3">
    <div class="flex items-center gap-2">
      <Input
        placeholder="Paste a drama/playlist URL to start downloading..."
        bind:value={resolveUrl}
        onkeydown={(e: KeyboardEvent) => e.key === "Enter" && resolveUrlHandle()}
        class="flex-1"
      />
      <Button onclick={resolveUrlHandle} disabled={isResolving}>
        {#if isResolving}
          <Loader2 class="w-4 h-4 mr-2 animate-spin" />
        {/if}
        Resolve
      </Button>
    </div>

    {#if errorMsg}
      <div class="text-sm text-destructive bg-destructive/10 border border-destructive/20 rounded-lg px-3 py-2">
        {errorMsg}
      </div>
    {/if}

    {#if providers.length > 0}
      <div class="flex items-center gap-2 text-xs text-muted-foreground">
        <Globe class="w-3.5 h-3.5" />
        <span>Supported:</span>
        {#each providers as p}
          <Badge variant="outline" class="text-xs">{p.name}</Badge>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Tabs: Downloads / Gallery -->
  {#if !resolvedPlaylist}
    <div class="flex items-center gap-1 px-4 pt-3 pb-0">
      <button
        class="px-4 py-1.5 text-sm font-medium rounded-md transition-colors
               {activeTab === 'downloads'
                 ? 'bg-primary text-primary-foreground shadow-sm'
                 : 'text-muted-foreground hover:text-foreground hover:bg-muted'}"
        onclick={() => activeTab = "downloads"}
      >
        <span class="flex items-center gap-1.5">
          <Download class="w-3.5 h-3.5" />
          Downloads
          {#if activeJobs.length > 0}
            <Badge variant="secondary" class="text-[10px] h-4 px-1.5 ml-0.5">{activeJobs.length}</Badge>
          {/if}
        </span>
      </button>
      <button
        class="px-4 py-1.5 text-sm font-medium rounded-md transition-colors
               {activeTab === 'gallery'
                 ? 'bg-primary text-primary-foreground shadow-sm'
                 : 'text-muted-foreground hover:text-foreground hover:bg-muted'}"
        onclick={() => activeTab = "gallery"}
      >
        <span class="flex items-center gap-1.5">
          <Clock class="w-3.5 h-3.5" />
          History
          {#if historyJobs.length > 0}
            <Badge variant="outline" class="text-[10px] h-4 px-1.5 ml-0.5">{historyJobs.length}</Badge>
          {/if}
        </span>
      </button>
    </div>
  {/if}

  <!-- Content -->
  <ScrollArea class="flex-1 p-4">
    {#if resolvedPlaylist}
      <!-- Resolved playlist preview -->
      <div class="flex flex-col gap-4 max-w-4xl mx-auto">
        <div class="flex gap-4 items-start bg-card border border-border p-4 rounded-xl">
          {#if resolvedPlaylist.cover}
            <img src={resolvedPlaylist.cover} alt="Cover" class="w-32 rounded object-cover shadow" />
          {/if}
          <div class="flex-1">
            <h2 class="text-2xl font-bold">{resolvedPlaylist.title}</h2>
            <div class="flex items-center gap-2 mt-2">
              <Badge variant="secondary">{resolvedPlaylist.provider}</Badge>
              <span class="text-sm text-muted-foreground">{resolvedPlaylist.items.length} Episodes</span>
            </div>
            {#if resolvedPlaylist.synopsis}
              <p class="text-sm mt-3 text-muted-foreground line-clamp-3">{resolvedPlaylist.synopsis}</p>
            {/if}
          </div>
          <div class="flex flex-col gap-2">
            <Button onclick={startDownload} disabled={selectedItems.size === 0}>
              <Download class="w-4 h-4 mr-2" />
              Download {selectedItems.size} Items
            </Button>
            <Button variant="outline" onclick={() => resolvedPlaylist = null}>Cancel</Button>
          </div>
        </div>

        <div class="bg-card border border-border rounded-xl p-4">
          <div class="flex items-center justify-between mb-4">
            <h3 class="font-semibold text-lg">Select Episodes</h3>
            <Button variant="ghost" size="sm" onclick={selectAll}>
              {selectedItems.size === resolvedPlaylist.items.length ? "Deselect All" : "Select All"}
            </Button>
          </div>
          
          <div class="grid grid-cols-4 sm:grid-cols-6 md:grid-cols-8 lg:grid-cols-10 gap-2">
            {#each resolvedPlaylist.items as item}
              <button
                class="relative h-12 flex flex-col items-center justify-center rounded border text-sm transition-colors cursor-pointer
                       {selectedItems.has(item.id) ? 'bg-primary/10 border-primary text-primary' : 'bg-muted border-border hover:bg-accent'}"
                onclick={() => toggleSelection(item.id)}
              >
                <span class="font-medium text-lg leading-none">{item.index}</span>
              </button>
            {/each}
          </div>
        </div>
      </div>

    {:else if activeTab === "downloads"}
      <!-- Active downloads list -->
      <div class="flex flex-col gap-4 max-w-5xl mx-auto">
        {#if activeJobs.length === 0}
          <div class="flex flex-col items-center justify-center py-20 text-muted-foreground">
            <Download class="w-12 h-12 mb-4 opacity-50" />
            <p>No active downloads.</p>
            {#if historyJobs.length > 0}
              <button
                class="mt-2 text-sm text-primary hover:underline"
                onclick={() => activeTab = "gallery"}
              >
                View download history →
              </button>
            {/if}
          </div>
        {/if}

        {#each activeJobs as job}
          <div class="bg-card border border-border rounded-xl p-4 flex flex-col gap-3">
            <div class="flex items-start justify-between">
              <div class="flex items-center gap-3">
                {#if job.cover}
                  <img src={job.cover} alt="" class="w-14 h-14 rounded-lg object-cover shadow-sm flex-shrink-0" />
                {/if}
                <div class="flex flex-col">
                  <h3 class="font-bold text-lg leading-tight">{job.title}</h3>
                  <div class="flex items-center gap-2 mt-1">
                    <Badge variant={
                      job.status === 'running' ? 'secondary' : 'outline'
                    }>
                      {job.status}
                    </Badge>
                    <span class="text-xs text-muted-foreground">
                      {job.items.filter((i: any) => i.status === 'completed').length} / {job.items.length} items
                    </span>
                  </div>
                </div>
              </div>
              <div class="flex items-center gap-2">
                {#if job.status === "running"}
                  <Button variant="outline" size="icon" onclick={() => actionJob(job.id, "pause")} title="Pause">
                    <Pause class="w-4 h-4" />
                  </Button>
                  <Button variant="destructive" size="icon" onclick={() => actionJob(job.id, "cancel")} title="Cancel">
                    <X class="w-4 h-4" />
                  </Button>
                {:else if job.status === "paused" || job.status === "failed"}
                  <Button variant="outline" size="icon" onclick={() => actionJob(job.id, "resume")} title="Resume">
                    <Play class="w-4 h-4" />
                  </Button>
                  <Button variant="destructive" size="icon" onclick={() => actionJob(job.id, "cancel")} title="Cancel">
                    <X class="w-4 h-4" />
                  </Button>
                {:else}
                  <Button variant="ghost" size="icon" onclick={() => actionJob(job.id, "delete")} title="Remove Job">
                    <Trash2 class="w-4 h-4 text-destructive" />
                  </Button>
                {/if}
              </div>
            </div>

            <div class="flex items-center gap-3">
              <Progress value={getJobProgress(job)} class="h-2 flex-1" />
              <span class="text-xs w-12 text-right">{Math.round(getJobProgress(job))}%</span>
            </div>

            <!-- Active items -->
            {#if job.items.some((i: any) => i.status === 'downloading')}
              <div class="flex flex-col gap-1 mt-2">
                {#each job.items.filter((i: any) => i.status === 'downloading') as item}
                  <div class="flex items-center gap-2 text-xs text-muted-foreground">
                    <span class="truncate w-32">Ep {item.index}</span>
                    <Progress value={Math.round(item.progress)} class="h-1 flex-1" />
                    <span class="w-10 text-right">{Math.round(item.progress)}%</span>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {/each}
      </div>

    {:else}
      <!-- Gallery / History view -->
      <div class="flex flex-col gap-4 max-w-5xl mx-auto">
        {#if historyJobs.length === 0}
          <div class="flex flex-col items-center justify-center py-20 text-muted-foreground">
            <Clock class="w-12 h-12 mb-4 opacity-50" />
            <p>No download history yet.</p>
            <p class="text-xs mt-1">Completed downloads will appear here.</p>
          </div>
        {:else}
          <!-- Gallery grid -->
          <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {#each historyJobs as job}
              <div class="group relative bg-card border border-border rounded-xl overflow-hidden hover:border-primary/40 transition-all hover:shadow-lg hover:shadow-primary/5">
                <!-- Cover image / gradient header -->
                <div class="relative h-36 bg-gradient-to-br from-primary/20 via-primary/5 to-background overflow-hidden">
                  {#if job.cover}
                    <img
                      src={job.cover}
                      alt=""
                      class="w-full h-full object-cover opacity-80 group-hover:opacity-100 group-hover:scale-105 transition-all duration-300"
                    />
                    <div class="absolute inset-0 bg-gradient-to-t from-card via-card/40 to-transparent"></div>
                  {:else}
                    <div class="absolute inset-0 flex items-center justify-center">
                      <Download class="w-10 h-10 text-muted-foreground/30" />
                    </div>
                  {/if}

                  <!-- Status pill top-right -->
                  <div class="absolute top-2 right-2">
                    {#if job.status === "completed"}
                      <div class="flex items-center gap-1 bg-emerald-500/90 text-white text-[10px] font-semibold px-2 py-0.5 rounded-full backdrop-blur-sm">
                        <CheckCircle class="w-3 h-3" />
                        Done
                      </div>
                    {:else if job.status === "failed"}
                      <div class="flex items-center gap-1 bg-red-500/90 text-white text-[10px] font-semibold px-2 py-0.5 rounded-full backdrop-blur-sm">
                        <XCircle class="w-3 h-3" />
                        Failed
                      </div>
                    {:else}
                      <div class="flex items-center gap-1 bg-yellow-500/90 text-white text-[10px] font-semibold px-2 py-0.5 rounded-full backdrop-blur-sm">
                        <X class="w-3 h-3" />
                        Cancelled
                      </div>
                    {/if}
                  </div>
                </div>

                <!-- Info section -->
                <div class="p-3 flex flex-col gap-2">
                  <h4 class="font-semibold text-sm leading-tight truncate" title={job.title}>
                    {job.title}
                  </h4>

                  <div class="flex items-center gap-2 text-xs text-muted-foreground">
                    <Badge variant="outline" class="text-[10px] h-4">{job.provider}</Badge>
                    <span>{job.items.filter((i: any) => i.status === 'completed').length}/{job.items.length} eps</span>
                    {#if getTotalSize(job)}
                      <span>·</span>
                      <span>{getTotalSize(job)}</span>
                    {/if}
                  </div>

                  <div class="flex items-center justify-between mt-1">
                    <span class="text-[10px] text-muted-foreground/70">
                      {formatDate(job.updated_at)}
                    </span>
                    <div class="flex items-center gap-1">
                      {#if job.status === "completed" && job.items.some((i: any) => i.status === 'completed')}
                        <Button
                          variant="ghost"
                          size="sm"
                          class="h-7 text-xs px-2"
                          onclick={() => openStaticApp("video-player", {
                            playlist: {
                              title: job.title,
                              items: job.items
                                .filter((i: any) => i.status === 'completed')
                                .sort((a: any, b: any) => a.index - b.index)
                                .map((i: any) => ({ path: i.output_path, title: i.title, index: i.index })),
                            },
                            startIndex: 0,
                          })}
                          title="Watch in Video Player"
                        >
                          <Film class="w-3 h-3 mr-1" />
                          Watch
                        </Button>
                      {/if}
                      {#if job.status === "failed"}
                        <Button variant="ghost" size="sm" class="h-7 text-xs px-2" onclick={() => actionJob(job.id, "resume")} title="Retry">
                          <Play class="w-3 h-3 mr-1" />
                          Retry
                        </Button>
                      {/if}
                      <Button
                        variant="ghost"
                        size="sm"
                        class="h-7 text-xs px-2"
                        onclick={() => openFolder(job.output_dir)}
                        title="Open folder"
                      >
                        <FolderOpen class="w-3 h-3 mr-1" />
                        Open
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        class="h-7 w-7"
                        onclick={() => actionJob(job.id, "delete")}
                        title="Remove from history"
                      >
                        <Trash2 class="w-3 h-3 text-destructive/70" />
                      </Button>
                    </div>
                  </div>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </ScrollArea>
</div>
