import type {
  AppManifest,
  InstalledApp,
  ResourceUsage,
  SystemInfo,
  SandboxVerifyResult,
  DiskUsage,
  LaunchResult,
  ChatResponse,
  ConversationSummary,
  StoredMessage,
  AgentConfigInfo,
} from "./types";
import type { StoreUploadRequest, StoreUploadResult } from "./types";

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
    get_resource_usage: { method: "GET", url: "/api/system/resources" },
    get_catalog:     { method: "GET",    url: "/api/catalog" },
    list_apps:       { method: "GET",    url: "/api/apps" },
    get_app:         { method: "GET",    url: `/api/apps/${appId}` },
    install_app:     { method: "POST",   url: "/api/apps", body: { manifest: args?.manifest } },
    uninstall_app:   { method: "DELETE", url: `/api/apps/${appId}` },
    launch_app:      { method: "POST",   url: `/api/apps/${appId}/launch` },
    stop_app:        { method: "POST",   url: `/api/apps/${appId}/stop` },
    verify_sandbox:  { method: "GET",    url: `/api/sandbox/${appId}/verify` },
    get_disk_usage:  { method: "GET",    url: `/api/sandbox/${appId}/disk` },
    upload_app:      { method: "POST",   url: `/api/store/upload`, body: args?.req },
    // Agent chat
    agent_chat:      { method: "POST",   url: `/api/agent/chat`, body: { message: args?.message, conversation_id: args?.conversationId } },
    agent_conversations: { method: "GET", url: `/api/agent/conversations` },
    agent_messages:  { method: "GET",    url: `/api/agent/conversations/${args?.conversationId}/messages` },
    agent_config:    { method: "GET",    url: `/api/agent/config` },
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
    if (command === "upload_app" && json.data) {
      // Return the StoreUploadResult even if backend returns 400 for validation details.
      return json.data as T;
    }
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

export async function getResourceUsage(): Promise<ResourceUsage> {
  return smartInvoke<ResourceUsage>("get_resource_usage");
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

export async function uploadApp(req: StoreUploadRequest): Promise<StoreUploadResult> {
  return smartInvoke<StoreUploadResult>("upload_app", { req });
}

// ── Agent Chat ──

export async function agentChat(message: string, conversationId?: string): Promise<ChatResponse> {
  return smartInvoke<ChatResponse>("agent_chat", { message, conversationId });
}

export async function agentConversations(): Promise<ConversationSummary[]> {
  return smartInvoke<ConversationSummary[]>("agent_conversations");
}

export async function agentMessages(conversationId: string): Promise<StoredMessage[]> {
  return smartInvoke<StoredMessage[]>("agent_messages", { conversationId });
}

export async function agentConfig(): Promise<AgentConfigInfo> {
  return smartInvoke<AgentConfigInfo>("agent_config");
}
