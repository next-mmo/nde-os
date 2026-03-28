<script lang="ts">
  import { createKanbanState, type KanbanTask } from "./kanban.svelte";

  const kanbanState = createKanbanState();
  const columns = ["Plan", "Waiting Approval", "YOLO mode", "Done by AI", "Verified by Human", "Re-open"];

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
      <div class="px-4 py-3 border-b-2 {columnAccentClass(column)} bg-black/30 flex items-center justify-between shrink-0 rounded-t-xl">
        <h3 class="font-semibold tracking-wide text-white/90 text-xs uppercase">{column}</h3>
        <span class="text-[10px] bg-white/10 px-2 py-0.5 rounded-full text-white/50 font-medium font-mono">
          {colTasks.length}
        </span>
      </div>

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
          <div
            role="listitem"
            data-card-id={task.filename}
            draggable={!task.locked}
            ondragstart={(e) => onDragStart(e, task)}
            ondragend={onDragEnd}
            class="group p-3 rounded-lg border transition-all duration-100 relative
                   {task.locked
                     ? 'opacity-60 cursor-not-allowed border-amber-500/40 bg-amber-500/5'
                     : isGhost
                       ? 'opacity-25 scale-[0.97] border-white/5 bg-white/3'
                       : 'border-white/10 bg-white/5 hover:bg-white/10 hover:border-white/25 cursor-grab active:cursor-grabbing'}"
          >
            {#if task.locked}
              <div class="absolute top-2.5 right-2.5 text-amber-400/80" title="Locked — AI in YOLO mode">
                <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                  <path stroke-linecap="round" stroke-linejoin="round"
                    d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"/>
                </svg>
              </div>
            {:else}
              <div class="absolute top-2.5 right-2.5 text-white/20 group-hover:text-white/50 transition-colors">
                <svg class="w-3.5 h-3.5" fill="currentColor" viewBox="0 0 24 24">
                  <circle cx="9"  cy="7"  r="1.5"/>
                  <circle cx="15" cy="7"  r="1.5"/>
                  <circle cx="9"  cy="12" r="1.5"/>
                  <circle cx="15" cy="12" r="1.5"/>
                  <circle cx="9"  cy="17" r="1.5"/>
                  <circle cx="15" cy="17" r="1.5"/>
                </svg>
              </div>
            {/if}

            <h4 class="font-medium text-white/90 pr-5 break-words text-[13px] leading-snug mb-2">
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
