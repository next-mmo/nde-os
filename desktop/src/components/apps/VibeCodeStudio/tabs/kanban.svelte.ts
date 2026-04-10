import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { onMount, onDestroy } from "svelte";

export interface KanbanTask {
  id: number;
  filename: string;
  title: string;
  status: string;
  locked: boolean;
  order?: number;
}

export function createKanbanState() {
  let tasks = $state<KanbanTask[]>([]);
  let unlisten: UnlistenFn | null = null;

  const refresh = async () => {
    try {
      tasks = await invoke<KanbanTask[]>("get_agent_tasks");
    } catch (e) {
      console.error(e);
    }
  };

  onMount(async () => {
    try {
      await invoke("watch_tasks_dir");
    } catch (e) {
      console.warn("Failed to start tasks watcher:", e);
    }
    refresh();
    unlisten = await listen("tasks://updated", () => {
      refresh();
    });
  });

  onDestroy(() => {
    if (unlisten) unlisten();
  });

  return {
    get tasks() { return tasks; },
    refresh,

    /**
     * Move task to a new status column and optionally reorder it.
     * @param filename  Task to move
     * @param newStatus Target column
     * @param insertBefore  Filename of the card to insert before (null = end of column)
     */
    async updateStatus(filename: string, newStatus: string, insertBefore: string | null = null) {
      // ── Optimistic update ──────────────────────────────────────────
      const idx = tasks.findIndex(t => t.filename === filename);
      if (idx === -1) return;

      // Remove from current position
      const [moved] = tasks.splice(idx, 1);
      moved.status = newStatus;

      if (insertBefore === null) {
        // Append to end
        tasks.push(moved);
      } else {
        const targetIdx = tasks.findIndex(t => t.filename === insertBefore);
        if (targetIdx === -1) {
          tasks.push(moved);
        } else {
          tasks.splice(targetIdx, 0, moved);
        }
      }

      // ── Persist ────────────────────────────────────────────────────
      try {
        await invoke("update_agent_task_status", { filename, newStatus });
        await refresh();
      } catch (e) {
        console.error(e);
        await refresh();
      }
    }
  };
}
