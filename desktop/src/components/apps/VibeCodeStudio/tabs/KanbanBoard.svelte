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
  let manuallyUnlocked = $state<Set<string>>(new Set());

  // ── Actions ───────────────────────────────────────────────────
  async function submitCreateTask(col: string) {
    if (!createTitle.trim()) { showCreateCol = null; return; }
    try {
      await invoke("create_agent_task", { title: createTitle, description: "", checklist: [], content: null });
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

  // ── Pointer-based drag state (replaces HTML5 DragEvent for WebView2) ──
  let draggedFilename = $state<string | null>(null);
  let dragOverColumn  = $state<string | null>(null);
  let insertBefore    = $state<string | null>(null);
  let ghostX = $state(0);
  let ghostY = $state(0);
  let dragStarted = false;
  let pointerDownPos: { x: number; y: number } | null = null;
  let pendingDragTask: KanbanTask | null = null;
  let boardEl: HTMLDivElement;

  const DRAG_THRESHOLD = 5; // px before drag activates

  function tasksForColumn(col: string) {
    return kanbanState.tasks.filter(t => t.status === col);
  }

  function columnAccentClass(col: string) {
    if (col === "YOLO mode") return "border-b-amber-500/60";
    if (col === "Done by AI" || col === "Verified by Human") return "border-b-emerald-500/60";
    if (col === "Re-open") return "border-b-rose-500/60";
    return "border-b-sky-500/60";
  }

  // ── Pointer drag handlers ─────────────────────────────────────
  function onCardPointerDown(e: PointerEvent, task: KanbanTask) {
    // Ignore if locked, or if clicking on a button (delete, options, unlock)
    const effectiveLocked = task.locked && !manuallyUnlocked.has(task.filename);
    if (effectiveLocked) return;
    if ((e.target as HTMLElement).closest("button")) return;

    pointerDownPos = { x: e.clientX, y: e.clientY };
    pendingDragTask = task;
    dragStarted = false;
  }

  function onBoardPointerMove(e: PointerEvent) {
    if (!pointerDownPos || !pendingDragTask) return;

    const dx = e.clientX - pointerDownPos.x;
    const dy = e.clientY - pointerDownPos.y;

    // Activate drag only after threshold
    if (!dragStarted) {
      if (Math.abs(dx) + Math.abs(dy) < DRAG_THRESHOLD) return;
      dragStarted = true;
      draggedFilename = pendingDragTask.filename;
    }

    ghostX = e.clientX;
    ghostY = e.clientY;

    // Hit-test: which column is the pointer over?
    // Temporarily hide ghost to get element below
    const ghostEl = document.getElementById("drag-ghost");
    if (ghostEl) ghostEl.style.pointerEvents = "none";

    const elBelow = document.elementFromPoint(e.clientX, e.clientY) as HTMLElement | null;

    if (ghostEl) ghostEl.style.pointerEvents = "";

    if (elBelow) {
      const colEl = elBelow.closest<HTMLElement>("[data-col-id]");
      if (colEl) {
        dragOverColumn = colEl.dataset.colId!;

        // Find insertion position
        const cardEl = elBelow.closest<HTMLElement>("[data-card-id]");
        if (cardEl && draggedFilename) {
          const cardId = cardEl.dataset.cardId!;
          if (cardId !== draggedFilename) {
            const rect = cardEl.getBoundingClientRect();
            if (e.clientY < rect.top + rect.height / 2) {
              insertBefore = cardId;
            } else {
              const colTasks = tasksForColumn(dragOverColumn);
              const idx = colTasks.findIndex(t => t.filename === cardId);
              insertBefore = idx + 1 < colTasks.length ? colTasks[idx + 1].filename : null;
            }
          }
        } else {
          insertBefore = null;
        }
      } else {
        dragOverColumn = null;
        insertBefore = null;
      }
    }
  }

  function onBoardPointerUp(_e: PointerEvent) {
    if (dragStarted && draggedFilename && dragOverColumn) {
      kanbanState.updateStatus(draggedFilename, dragOverColumn, insertBefore);
    }

    // If we never exceeded the threshold, it's a click → open detail
    if (!dragStarted && pendingDragTask) {
      openDetail(pendingDragTask);
    }

    // Reset all drag state
    draggedFilename = null;
    dragOverColumn  = null;
    insertBefore    = null;
    pointerDownPos  = null;
    pendingDragTask = null;
    dragStarted     = false;
  }

  function getDraggedTask(): KanbanTask | undefined {
    if (!draggedFilename) return undefined;
    return kanbanState.tasks.find(t => t.filename === draggedFilename);
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  bind:this={boardEl}
  class="h-full w-full p-4 flex gap-4 overflow-x-auto overflow-y-hidden text-sm bg-black/40 select-none"
  onpointermove={onBoardPointerMove}
  onpointerup={onBoardPointerUp}
  onpointerleave={onBoardPointerUp}
>
  {#each columns as column}
    {@const colTasks   = tasksForColumn(column)}
    {@const isOver     = dragOverColumn === column}
    {@const isDragging = draggedFilename !== null}

    <div
      role="list"
      aria-label={column}
      data-col-id={column}
      class="flex flex-col w-72 shrink-0 rounded-xl border transition-all duration-150
             {isOver
               ? 'bg-white/10 border-white/25 shadow-lg shadow-white/5'
               : 'bg-black/40 border-white/10'}"
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
          {@const isUnlocked = manuallyUnlocked.has(task.filename)}
          {@const effectiveLocked = task.locked && !isUnlocked}

          <!-- Insertion line ABOVE card -->
          {#if showLineAbove}
            <div class="h-[3px] rounded-full bg-sky-400 mx-1 shadow-sm shadow-sky-400/60 my-0.5"></div>
          {/if}

          <!-- Card -->
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <div
            role="listitem"
            data-card-id={task.filename}
            onpointerdown={(e) => onCardPointerDown(e, task)}
            onclick={(e) => {
              // Only open detail on programmatic clicks (e.g. .click()) — real user clicks
              // are handled by the pointer up handler to distinguish click vs drag.
              // isTrusted=false means programmatic, or if no pointer was tracked it's a keyboard/click event.
              if (!e.isTrusted || (!pointerDownPos && !dragStarted)) {
                if (!(e.target as HTMLElement).closest("button")) openDetail(task);
              }
            }}
            class="group p-3 rounded-lg border transition-all duration-100 relative
                   {effectiveLocked
                     ? 'opacity-60 cursor-not-allowed border-amber-500/40 bg-amber-500/5'
                     : isGhost
                       ? 'opacity-25 scale-[0.97] border-white/5 bg-white/3'
                       : 'border-white/10 bg-white/5 hover:bg-white/10 hover:border-white/25 cursor-grab active:cursor-grabbing'}
                   {activeContextMenu === task.filename ? 'z-50! ring-1 ring-white/10 shadow-xl' : 'z-0!'}"
          >
            {#if effectiveLocked}
              <!-- svelte-ignore a11y_consider_explicit_label -->
              <button class="absolute top-2.5 right-2.5 text-amber-400/80 hover:text-amber-300 transition-colors" title="Click to unlock" onclick={(e) => { e.stopPropagation(); manuallyUnlocked = new Set([...manuallyUnlocked, task.filename]); }}>
                <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                  <path stroke-linecap="round" stroke-linejoin="round"
                    d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"/>
                </svg>
              </button>
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

<!-- Floating drag ghost -->
{#if draggedFilename}
  {@const dragTask = getDraggedTask()}
  {#if dragTask}
    <div
      id="drag-ghost"
      class="fixed pointer-events-none z-999 w-64 p-3 rounded-lg border border-indigo-500/60 bg-neutral-900/95 shadow-2xl shadow-indigo-500/20 backdrop-blur-sm"
      style="left: {ghostX + 12}px; top: {ghostY - 16}px;"
    >
      <h4 class="font-medium text-white/90 text-[13px] leading-snug mb-1 truncate">{dragTask.title}</h4>
      <span class="text-[10px] text-indigo-400/80 font-mono">{dragTask.filename}</span>
    </div>
  {/if}
{/if}

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
