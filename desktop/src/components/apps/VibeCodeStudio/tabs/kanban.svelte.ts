import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { onMount, onDestroy } from "svelte";

export interface KanbanTask {
  filename: string;
  title: string;
  status: string;
  locked: boolean;
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
    async updateStatus(filename: string, newStatus: string) {
      const idx = tasks.findIndex(t => t.filename === filename);
      if (idx !== -1) tasks[idx].status = newStatus;

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
