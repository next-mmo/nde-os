import type {
  AppManifest,
  InstalledApp,
  SystemInfo,
  SandboxVerifyResult,
  DiskUsage,
  LaunchResult,
} from "./types";

const API_BASE = "http://localhost:8080";

/**
 * Smart invoke: uses Tauri IPC when running inside the Tauri webview,
 * falls back to the real HTTP API server otherwise.
 */
async function smartInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  // Use Tauri IPC when available
  if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(command, args);
  }

  // Fallback to real HTTP API server
  return httpFallback<T>(command, args);
}

/**
 * Map Tauri command names to HTTP API calls.
 */
async function httpFallback<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const appId = args?.appId as string | undefined;

  const routeMap: Record<string, { method: string; url: string; body?: unknown }> = {
    health_check:    { method: "GET",    url: "/api/health" },
    get_system_info: { method: "GET",    url: "/api/system" },
    get_catalog:     { method: "GET",    url: "/api/catalog" },
    list_apps:       { method: "GET",    url: "/api/apps" },
    get_app:         { method: "GET",    url: `/api/apps/${appId}` },
    install_app:     { method: "POST",   url: "/api/apps", body: { manifest: args?.manifest } },
    uninstall_app:   { method: "DELETE", url: `/api/apps/${appId}` },
    launch_app:      { method: "POST",   url: `/api/apps/${appId}/launch` },
    stop_app:        { method: "POST",   url: `/api/apps/${appId}/stop` },
    verify_sandbox:  { method: "GET",    url: `/api/sandbox/${appId}/verify` },
    get_disk_usage:  { method: "GET",    url: `/api/sandbox/${appId}/disk` },
  };

  const route = routeMap[command];
  if (!route) throw new Error(`Unknown command: ${command}`);

  const fetchOpts: RequestInit = {
    method: route.method,
    headers: { "Content-Type": "application/json" },
  };

  if (route.body) {
    fetchOpts.body = JSON.stringify(route.body);
  }

  const res = await fetch(`${API_BASE}${route.url}`, fetchOpts);
  const json = await res.json();

  if (!json.success) {
    throw new Error(json.message || "API error");
  }

  return json.data as T;
}

// ── System ──

export async function healthCheck(): Promise<string> {
  return smartInvoke<string>("health_check");
}

export async function getSystemInfo(): Promise<SystemInfo> {
  return smartInvoke<SystemInfo>("get_system_info");
}

// ── Catalog ──

export async function getCatalog(): Promise<AppManifest[]> {
  return smartInvoke<AppManifest[]>("get_catalog");
}

// ── Apps ──

export async function listApps(): Promise<InstalledApp[]> {
  return smartInvoke<InstalledApp[]>("list_apps");
}

export async function getApp(appId: string): Promise<InstalledApp | null> {
  return smartInvoke<InstalledApp | null>("get_app", { appId });
}

export async function installApp(manifest: AppManifest): Promise<InstalledApp> {
  return smartInvoke<InstalledApp>("install_app", { manifest });
}

export async function uninstallApp(appId: string): Promise<string> {
  return smartInvoke<string>("uninstall_app", { appId });
}

// ── Lifecycle ──

export async function launchApp(appId: string): Promise<LaunchResult> {
  return smartInvoke<LaunchResult>("launch_app", { appId });
}

export async function stopApp(appId: string): Promise<string> {
  return smartInvoke<string>("stop_app", { appId });
}


// ── Sandbox ──

export async function verifySandbox(appId: string): Promise<SandboxVerifyResult> {
  return smartInvoke<SandboxVerifyResult>("verify_sandbox", { appId });
}

export async function getDiskUsage(appId: string): Promise<DiskUsage> {
  return smartInvoke<DiskUsage>("get_disk_usage", { appId });
}
