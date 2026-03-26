<script lang="ts">
  import { createKanbanState, type KanbanTask } from "./kanban.svelte";
  
  const kanbanState = createKanbanState();
  const columns = ["Plan", "Waiting Approval", "YOLO mode", "Done by AI", "Verified by Human", "Re-open"];
  
  let draggedTask = $state<KanbanTask | null>(null);
  
  function handleDragStart(e: DragEvent, task: KanbanTask) {
    if (task.locked) {
      e.preventDefault();
      return;
    }
    draggedTask = task;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", task.filename);
    }
  }
  
  function handleDragOver(e: DragEvent) {
    e.preventDefault();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = "move";
    }
  }
  
  function handleDrop(e: DragEvent, targetStatus: string) {
    e.preventDefault();
    if (draggedTask) {
      if (draggedTask.status !== targetStatus) {
         kanbanState.updateStatus(draggedTask.filename, targetStatus);
      }
      draggedTask = null;
    }
  }
</script>

<div class="h-full w-full p-6 flex gap-6 overflow-x-auto overflow-y-hidden text-sm bg-black/40">
  {#each columns as column}
    <div 
      role="list"
      class="flex flex-col w-72 shrink-0 bg-black/40 border border-white/10 rounded-xl overflow-hidden backdrop-blur-md"
      ondragover={handleDragOver}
      ondrop={(e) => handleDrop(e, column)}
    >
      <!-- Column Header -->
      <div class="px-4 py-3 border-b-2 {column === 'YOLO mode' ? 'border-b-amber-500/50' : column === 'Done by AI' || column === 'Verified by Human' ? 'border-b-green-500/50' : 'border-b-rose-500/50'} bg-black/30 flex items-center justify-between">
        <h3 class="font-semibold tracking-wide text-white/90">{column}</h3>
        <span class="text-xs bg-white/10 px-2 py-0.5 rounded-full text-white/50 font-medium font-mono">
          {kanbanState.tasks.filter(t => t.status === column).length}
        </span>
      </div>
      
      <!-- Cards List -->
      <div class="flex-1 p-3 overflow-y-auto overflow-x-hidden flex flex-col gap-3 scrollbar-thin">
        {#each kanbanState.tasks.filter(t => t.status === column) as task}
          <div 
            role="listitem"
            draggable={!task.locked}
            ondragstart={(e) => handleDragStart(e, task)}
            class="group p-4 bg-white/5 hover:bg-white/10 border {task.locked ? 'opacity-70 cursor-not-allowed! border-amber-500/50 bg-amber-500/5' : 'border-white/10 hover:border-white/20'} rounded-lg shadow-sm cursor-grab active:cursor-grabbing transition-all relative"
          >
            {#if task.locked}
              <div class="absolute top-3 right-3 text-amber-400 opacity-90" title="Locked by AI in YOLO mode">
                 <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                   <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"></path>
                 </svg>
              </div>
            {/if}
            
            <h4 class="font-medium text-white/90 pr-6 wrap-break-word text-sm mb-2 leading-snug">{task.title}</h4>
            <div class="flex items-center gap-2">
              <span class="text-[10px] bg-black/40 text-white/40 font-mono truncate px-1.5 py-0.5 rounded border border-white/5">{task.filename}</span>
            </div>
          </div>
        {/each}
        
        <!-- Empty state placeholder -->
        {#if kanbanState.tasks.filter(t => t.status === column).length === 0}
          <div class="h-20 border-2 border-dashed border-white/5 rounded-lg flex items-center justify-center text-white/20 text-xs italic">
            Drop task here
          </div>
        {/if}
      </div>
    </div>
  {/each}
</div>
