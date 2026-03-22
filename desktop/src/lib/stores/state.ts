import { writable, derived } from "svelte/store";
import * as api from "$lib/api/backend";
import type { AppManifest, InstalledApp, ResourceUsage, SystemInfo } from "$lib/api/types";
import { logStore } from "./logs";

export type HealthStatus = "unknown" | "online" | "offline";

export const catalog = writable<AppManifest[]>([]);
export const installed = writable<InstalledApp[]>([]);
export const systemInfo = writable<SystemInfo | null>(null);
export const resourceUsage = writable<ResourceUsage | null>(null);
export const healthStatus = writable<HealthStatus>("unknown");
export const lastRefreshAt = writable<string | null>(null);

export const installedMap = derived(installed, ($installed) => {
  const map: Record<string, InstalledApp> = {};
  for (const app of $installed) {
    map[app.manifest.id] = app;
  }
  return map;
});

export const runningCount = derived(installed, ($installed) =>
  $installed.filter((app) => app.status.state === "Running").length,
);

export const catalogCount = derived(catalog, ($catalog) => $catalog.length);

export async function refreshCatalog() {
  try {
    catalog.set(await api.getCatalog());
  } catch (error) {
    logStore.error(`Failed to load catalog: ${error}`);
  }
}

export async function refreshInstalled() {
  try {
    installed.set(await api.listApps());
  } catch (error) {
    logStore.error(`Failed to load installed apps: ${error}`);
  }
}

export async function refreshSystemInfo() {
  try {
    systemInfo.set(await api.getSystemInfo());
  } catch (error) {
    logStore.error(`Failed to load system info: ${error}`);
  }
}

export async function refreshResourceUsage() {
  try {
    resourceUsage.set(await api.getResourceUsage());
  } catch (error) {
    logStore.error(`Failed to load resource usage: ${error}`);
  }
}

export async function refreshHealth() {
  try {
    await api.healthCheck();
    healthStatus.set("online");
  } catch (error) {
    healthStatus.set("offline");
    logStore.warn(`Server health check failed: ${error}`);
  }
}

export async function refreshAll() {
  await Promise.all([
    refreshCatalog(),
    refreshInstalled(),
    refreshSystemInfo(),
    refreshResourceUsage(),
    refreshHealth(),
  ]);
  lastRefreshAt.set(new Date().toISOString());
}

export async function installApp(manifest: AppManifest) {
  logStore.info(`Installing ${manifest.name}...`, manifest.id);
  try {
    await api.installApp(manifest);
    logStore.success(`${manifest.name} installed`, manifest.id);
    await refreshInstalled();
  } catch (error) {
    logStore.error(`Install failed: ${error}`, manifest.id);
    throw error;
  }
}

export async function launchApp(appId: string) {
  logStore.info(`Launching ${appId}...`, appId);
  try {
    const result = await api.launchApp(appId);
    logStore.success(`Launched PID:${result.pid} port:${result.port}`, appId);
    await refreshInstalled();
    return result;
  } catch (error) {
    logStore.error(`Launch failed: ${error}`, appId);
    throw error;
  }
}

export async function stopApp(appId: string) {
  logStore.info(`Stopping ${appId}...`, appId);
  try {
    await api.stopApp(appId);
    logStore.success(`${appId} stopped`, appId);
    await refreshInstalled();
  } catch (error) {
    logStore.error(`Stop failed: ${error}`, appId);
    throw error;
  }
}

export async function uninstallApp(appId: string) {
  logStore.info(`Uninstalling ${appId}...`, appId);
  try {
    await api.uninstallApp(appId);
    logStore.success(`${appId} uninstalled`, appId);
    await refreshInstalled();
  } catch (error) {
    logStore.error(`Uninstall failed: ${error}`, appId);
    throw error;
  }
}
