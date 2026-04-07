import type { LogEntry } from "$lib/api/types";
import { fetchGatewayLogs } from "$lib/api/backend";
import { writable } from "svelte/store";

export interface ExtendedLogEntry extends LogEntry {
  source?: string; // "frontend" | "telegram" | "gateway" | "system" | ...
}

let logId = 0;

function createLogStore() {
  const { subscribe, update } = writable<ExtendedLogEntry[]>([]);

  return {
    subscribe,
    add(level: LogEntry["level"], message: string, app_id?: string, source?: string) {
      const entry: ExtendedLogEntry = {
        id: ++logId,
        timestamp: new Date().toISOString(),
        level,
        message,
        app_id,
        source: source ?? "frontend",
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

// ── Backend log polling ─────────────────────────────────────────────────────

let lastBackendId = 0;
let pollingInterval: ReturnType<typeof setInterval> | null = null;

async function pollBackendLogs() {
  const entries = await fetchGatewayLogs(lastBackendId);
  if (entries.length === 0) return;

  // Track highest ID
  for (const entry of entries) {
    if (entry.id > lastBackendId) lastBackendId = entry.id;
  }

  // Merge into logStore (newest first to match frontend order)
  for (const entry of entries) {
    logStore.add(
      entry.level,
      entry.message,
      entry.app_id,
      entry.source,
    );
  }
}

export function startLogPolling(intervalMs = 3000) {
  if (pollingInterval) return;
  pollBackendLogs(); // initial fetch
  pollingInterval = setInterval(pollBackendLogs, intervalMs);
}

export function stopLogPolling() {
  if (pollingInterval) {
    clearInterval(pollingInterval);
    pollingInterval = null;
  }
}
