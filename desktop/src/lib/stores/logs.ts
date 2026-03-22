import type { LogEntry } from "$lib/api/types";
import { writable } from "svelte/store";

let logId = 0;

function createLogStore() {
  const { subscribe, update } = writable<LogEntry[]>([]);

  return {
    subscribe,
    add(level: LogEntry["level"], message: string, app_id?: string) {
      const entry: LogEntry = {
        id: ++logId,
        timestamp: new Date().toISOString(),
        level,
        message,
        app_id,
      };
      update((logs) => [entry, ...logs].slice(0, 500));
    },
    info: (msg: string, app?: string) => logStore.add("info", msg, app),
    success: (msg: string, app?: string) => logStore.add("success", msg, app),
    warn: (msg: string, app?: string) => logStore.add("warning", msg, app),
    error: (msg: string, app?: string) => logStore.add("error", msg, app),
    clear() {
      update(() => []);
    },
  };
}

export const logStore = createLogStore();
