import { writable, derived } from "svelte/store";
import * as api from "$lib/api/backend";
import type { AppManifest, InstalledApp, SystemInfo } from "$lib/api/types";
import { logStore } from "./logs";

// ── Stores ──

export const catalog = writable<AppManifest[]>([]);
export const installed = writable<InstalledApp[]>([]);
export const systemInfo = writable<SystemInfo | null>(null);
export const searchFilter = writable("");

// Derived: map installed apps by ID for quick lookup
export const installedMap = derived(installed, ($installed) => {
  const map: Record<string, InstalledApp> = {};
  for (const app of $installed) {
    map[app.manifest.id] = app;
  }
  return map;
});

// Derived: count of running apps
export const runningCount = derived(installed, ($installed) =>
  $installed.filter((a) => a.status.state === "Running").length
);

// ── Actions ──

export async function refreshCatalog() {
  try {
    const data = await api.getCatalog();
    catalog.set(data);
  } catch (e) {
    logStore.error(`Failed to load catalog: ${e}`);
  }
}

export async function refreshInstalled() {
  try {
    const data = await api.listApps();
    installed.set(data);
  } catch (e) {
    logStore.error(`Failed to load installed apps: ${e}`);
  }
}

export async function refreshSystemInfo() {
  try {
    const data = await api.getSystemInfo();
    systemInfo.set(data);
  } catch (e) {
    logStore.error(`Failed to load system info: ${e}`);
  }
}

export async function refreshAll() {
  await Promise.all([refreshCatalog(), refreshInstalled(), refreshSystemInfo()]);
}

export async function installApp(manifest: AppManifest) {
  logStore.info(`Installing ${manifest.name}...`, manifest.id);
  try {
    await api.installApp(manifest);
    logStore.success(`${manifest.name} installed`, manifest.id);
    await refreshInstalled();
  } catch (e) {
    logStore.error(`Install failed: ${e}`, manifest.id);
    throw e;
  }
}

export async function launchApp(appId: string) {
  logStore.info(`Launching ${appId}...`, appId);
  try {
    const result = await api.launchApp(appId);
    logStore.success(`Launched PID:${result.pid} port:${result.port}`, appId);
    await refreshInstalled();
    return result;
  } catch (e) {
    logStore.error(`Launch failed: ${e}`, appId);
    throw e;
  }
}

export async function stopApp(appId: string) {
  logStore.info(`Stopping ${appId}...`, appId);
  try {
    await api.stopApp(appId);
    logStore.success(`${appId} stopped`, appId);
    await refreshInstalled();
  } catch (e) {
    logStore.error(`Stop failed: ${e}`, appId);
    throw e;
  }
}

export async function uninstallApp(appId: string) {
  logStore.info(`Uninstalling ${appId}...`, appId);
  try {
    await api.uninstallApp(appId);
    logStore.success(`${appId} uninstalled`, appId);
    await refreshInstalled();
  } catch (e) {
    logStore.error(`Uninstall failed: ${e}`, appId);
    throw e;
  }
}
