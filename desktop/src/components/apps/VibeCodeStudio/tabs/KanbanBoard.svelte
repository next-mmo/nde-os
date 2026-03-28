<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { fade, fly } from "svelte/transition";
  import { createKanbanState, type KanbanTask } from "./kanban.svelte";

  const kanbanState = createKanbanState();
  const columns = ["Plan", "Waiting Approval", "YOLO mode", "Done by AI", "Verified by Human", "Re-open"];

  // ── Modals & Quick Actions state ────────────────────────────────
  let showCreateCol = $state<string | null>(null);
  let createTitle   = $state("");

  let selectedTask  = $state<KanbanTask | null>(null);
  let detailContent = $state("");
  let showDetail    = $state(false);

  let activeContextMenu = $state<string | null>(null);

  // ── Actions ───────────────────────────────────────────────────
  async function submitCreateTask(col: string) {
    if (!createTitle.trim()) { showCreateCol = null; return; }
    try {
      await invoke("create_agent_task", { title: createTitle, description: "", checklist: [] });
      // update_agent_task_status will automatically move it if it's not Plan, but create_agent_task defaults to Plan.
      // Easiest way to properly sort this if creating in another col is to move it after creation.
      // But creating directly handles plan state. Actually, just create it. We can worry about instant-move later.
      showCreateCol = null;
      createTitle = "";
    } catch (e) {
      console.error("Action error", e);
    }
  }

  async function deleteTask(filename: string) {
    try {
      await invoke("delete_agent_task", { filename });
    } catch (e) { console.error("Error", e); }
  }

  async function openDetail(task: KanbanTask) {
    selectedTask = task;
    try {
      detailContent = await invoke("get_agent_task_content", { filename: task.filename });
      showDetail = true;
    } catch (e) { console.error("Error", e); }
  }

  async function saveDetail() {
    if (!selectedTask) return;
    try {
      await invoke("update_agent_task_content", { filename: selectedTask.filename, content: detailContent });
      showDetail = false;
      selectedTask = null;
    } catch (e) { console.error("Error", e); }
  }

  // ── Drag state ────────────────────────────────────────────────
  let draggedFilename = $state<string | null>(null);
  let dragOverColumn  = $state<string | null>(null);
  let insertBefore    = $state<string | null>(null); // filename to insert before, null = end

  function tasksForColumn(col: string) {
    return kanbanState.tasks.filter(t => t.status === col);
  }

  function columnAccentClass(col: string) {
    if (col === "YOLO mode") return "border-b-amber-500/60";
    if (col === "Done by AI" || col === "Verified by Human") return "border-b-emerald-500/60";
    if (col === "Re-open") return "border-b-rose-500/60";
    return "border-b-sky-500/60";
  }

  // ── Card drag start / end ─────────────────────────────────────
  function onDragStart(e: DragEvent, task: KanbanTask) {
    if (task.locked) { e.preventDefault(); return; }
    draggedFilename = task.filename;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", task.filename);
    }
  }

  function onDragEnd(_e: DragEvent) {
    // Always reset — fires after drop or on cancel
    draggedFilename = null;
    dragOverColumn  = null;
    insertBefore    = null;
  }

  // ── Column drag-over: ONE handler, reads e.target for insert pos ──
  function onColumnDragOver(e: DragEvent, col: string) {
    e.preventDefault(); // required to allow drop — DO NOT remove
    if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
    dragOverColumn = col;

    // Walk up from e.target to find the card element
    const cardEl = (e.target as HTMLElement).closest<HTMLElement>("[data-card-id]");
    if (cardEl && draggedFilename) {
      const cardId = cardEl.dataset.cardId!;
      if (cardId !== draggedFilename) {
        const rect = cardEl.getBoundingClientRect();
        if (e.clientY < rect.top + rect.height / 2) {
          insertBefore = cardId;
        } else {
          // Insert after this card → find next sibling card in the column
          const colTasks = tasksForColumn(col);
          const idx = colTasks.findIndex(t => t.filename === cardId);
          insertBefore = idx + 1 < colTasks.length ? colTasks[idx + 1].filename : null;
        }
      }
    } else {
      // Not over a specific card → append to end
      insertBefore = null;
    }
  }

  function onColumnDragLeave(e: DragEvent) {
    // Only clear when leaving the column entirely (not entering a child)
    const related = e.relatedTarget as HTMLElement | null;
    if (!related || !(e.currentTarget as HTMLElement).contains(related)) {
      dragOverColumn = null;
      insertBefore   = null;
    }
  }

  function onColumnDrop(e: DragEvent, col: string) {
    e.preventDefault();
    if (!draggedFilename) return;
    kanbanState.updateStatus(draggedFilename, col, insertBefore);
    draggedFilename = null;
    dragOverColumn  = null;
    insertBefore    = null;
  }
</script>

<div class="h-full w-full p-4 flex gap-4 overflow-x-auto overflow-y-hidden text-sm bg-black/40 select-none">
  {#each columns as column}
    {@const colTasks   = tasksForColumn(column)}
    {@const isOver     = dragOverColumn === column}
    {@const isDragging = draggedFilename !== null}

    <!--
      NO overflow-hidden on the column — it breaks drag hit-testing.
      All handlers live here; cards do NOT call stopPropagation.
    -->
    <div
      role="list"
      aria-label={column}
      class="flex flex-col w-72 shrink-0 rounded-xl border transition-all duration-150
             {isOver
               ? 'bg-white/10 border-white/25 shadow-lg shadow-white/5'
               : 'bg-black/40 border-white/10'}"
      ondragover={(e) => onColumnDragOver(e, column)}
      ondragleave={onColumnDragLeave}
      ondrop={(e) => onColumnDrop(e, column)}
    >
      <!-- Column Header -->
      <div class="px-4 py-3 border-b-2 {columnAccentClass(column)} bg-black/30 flex items-center justify-between shrink-0 rounded-t-xl group">
        <div class="flex items-center gap-2">
          <h3 class="font-semibold tracking-wide text-white/90 text-xs uppercase">{column}</h3>
          <span class="text-[10px] bg-white/10 px-2 py-0.5 rounded-full text-white/50 font-medium font-mono">
            {colTasks.length}
          </span>
        </div>
        <button 
          class="w-5 h-5 flex items-center justify-center rounded bg-white/5 hover:bg-white/20 text-white/50 hover:text-white transition-colors opacity-0 group-hover:opacity-100" 
          title="New Task" 
          onclick={() => { showCreateCol = column; createTitle = ""; }}
        >
          <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
        </button>
      </div>

      <!-- Create input -->
      {#if showCreateCol === column}
        <div class="p-2 pb-0" transition:fly={{ y: -10, duration: 150 }}>
          <!-- svelte-ignore a11y_autofocus -->
          <input 
            type="text" 
            placeholder="Task title..." 
            bind:value={createTitle} 
            onkeydown={(e) => { 
               if (e.key === 'Enter') submitCreateTask(column); 
               if (e.key === 'Escape') showCreateCol = null; 
            }} 
            class="w-full bg-black/50 border border-indigo-500/50 rounded-lg px-3 py-2 text-xs text-white focus:outline-none" 
            autofocus 
          />
        </div>
      {/if}

      <!-- Cards List -->
      <div class="flex-1 p-2 overflow-y-auto flex flex-col gap-0.5 min-h-0 rounded-b-xl">
        {#each colTasks as task (task.filename)}
          {@const isGhost        = draggedFilename === task.filename}
          {@const showLineAbove  = insertBefore === task.filename && isOver && !isGhost}

          <!-- Insertion line ABOVE card -->
          {#if showLineAbove}
            <div class="h-[3px] rounded-full bg-sky-400 mx-1 shadow-sm shadow-sky-400/60 my-0.5"></div>
          {/if}

          <!-- Card — NO stopPropagation on any handler -->
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <div
            role="listitem"
            data-card-id={task.filename}
            draggable={!task.locked}
            ondragstart={(e) => onDragStart(e, task)}
            ondragend={onDragEnd}
            onclick={() => !isGhost && openDetail(task)}
            transition:fade={{ duration: 150 }}
            class="group p-3 rounded-lg border transition-all duration-100 relative
                   {task.locked
                     ? 'opacity-60 cursor-not-allowed border-amber-500/40 bg-amber-500/5'
                     : isGhost
                       ? 'opacity-25 scale-[0.97] border-white/5 bg-white/3'
                       : 'border-white/10 bg-white/5 hover:bg-white/10 hover:border-white/25 cursor-pointer active:cursor-grabbing'}
                   {activeContextMenu === task.filename ? 'z-50! ring-1 ring-white/10 shadow-xl' : 'z-0!'}"
          >
            {#if task.locked}
              <div class="absolute top-2.5 right-2.5 text-amber-400/80" title="Locked — AI in YOLO mode">
                <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                  <path stroke-linecap="round" stroke-linejoin="round"
                    d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"/>
                </svg>
              </div>
            {:else}
              <div class="absolute top-2.5 right-2 flex items-center gap-1 transition-opacity {activeContextMenu === task.filename ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'}">
                <!-- svelte-ignore a11y_consider_explicit_label -->
                <button title="Delete Task" class="p-1 hover:bg-white/10 rounded text-rose-400" onclick={(e) => { e.stopPropagation(); deleteTask(task.filename); }}>
                  <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
                </button>
                <div class="relative">
                  <!-- svelte-ignore a11y_consider_explicit_label -->
                  <button title="Options" class="p-1 hover:bg-white/10 rounded text-white/50" onclick={(e) => { e.stopPropagation(); activeContextMenu = activeContextMenu === task.filename ? null : task.filename; }}>
                    <svg class="w-3.5 h-3.5" fill="currentColor" viewBox="0 0 24 24">
                      <circle cx="12" cy="5" r="2"/><circle cx="12" cy="12" r="2"/><circle cx="12" cy="19" r="2"/>
                    </svg>
                  </button>
                  {#if activeContextMenu === task.filename}
                    <div class="absolute top-full right-0 mt-1 w-32 bg-neutral-800 border border-white/10 rounded shadow-xl z-50 overflow-hidden text-xs">
                       {#each columns as statusOption}
                         <button class="w-full text-left px-3 py-1.5 hover:bg-white/10 {task.status === statusOption ? 'text-indigo-400' : 'text-white/80'}" onclick={(e) => { e.stopPropagation(); kanbanState.updateStatus(task.filename, statusOption); activeContextMenu = null; }}>{statusOption}</button>
                       {/each}
                    </div>
                  {/if}
                </div>
              </div>
            {/if}
            <!-- svelte-ignore a11y_consider_explicit_label -->
            <h4 class="font-medium text-white/90 pr-12 wrap-break-word text-[13px] leading-snug mb-2">
              {task.title}
            </h4>
            <span class="text-[10px] bg-black/40 text-white/30 font-mono px-1.5 py-0.5 rounded border border-white/5 truncate block w-fit max-w-full">
              {task.filename}
            </span>
          </div>
        {/each}

        <!-- Insertion line at END of column -->
        {#if insertBefore === null && isOver && isDragging && !colTasks.find(t => t.filename === draggedFilename)}
          <div class="h-[3px] rounded-full bg-sky-400 mx-1 shadow-sm shadow-sky-400/60 my-0.5"></div>
        {/if}

        <!-- Empty column placeholder -->
        {#if colTasks.length === 0}
          <div class="flex-1 min-h-[80px] border-2 border-dashed rounded-lg flex items-center justify-center text-xs italic transition-all
                      {isOver ? 'border-sky-500/50 text-sky-400/60' : 'border-white/5 text-white/20'}">
            {isOver ? "Release to drop" : "No tasks"}
          </div>
        {/if}
      </div>
    </div>
  {/each}
</div>

<!-- Detail Panel -->
{#if showDetail && selectedTask}
<div class="absolute inset-y-0 right-0 w-[500px] bg-neutral-900 border-l border-white/10 shadow-2xl flex flex-col z-50 shadow-black/50" transition:fly={{ x: 100, duration: 200 }}>
  <div class="h-12 border-b border-white/10 px-4 flex items-center justify-between shrink-0 bg-black/20">
    <div class="font-medium text-sm text-white/90 truncate mr-4">{selectedTask.filename}</div>
    <div class="flex items-center gap-2 shrink-0">
      <button class="px-3 py-1 bg-indigo-500 hover:bg-indigo-600 text-white text-xs font-medium rounded transition-colors" onclick={saveDetail}>Save & Close</button>
      <!-- svelte-ignore a11y_consider_explicit_label -->
      <button class="w-8 h-8 rounded hover:bg-white/10 flex items-center justify-center text-white/50 transition-colors" onclick={() => { showDetail = false; selectedTask = null; }}>
        <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
      </button>
    </div>
  </div>
  <div class="flex-1 p-4 overflow-y-auto bg-black/40">
    <textarea bind:value={detailContent} class="w-full h-full bg-transparent text-[13px] leading-relaxed text-white/80 font-mono resize-none focus:outline-none scrollbar-thin"></textarea>
  </div>
</div>
{/if}

<!-- Click outside context menu -->
{#if activeContextMenu}
<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="fixed inset-0 z-40" onclick={(e) => { e.stopPropagation(); activeContextMenu = null; }}></div>
{/if}
