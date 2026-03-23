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
  ProviderStatus,
  ProviderConfig,
  PluginStatus,
  ChannelStatus,
  McpTool,
  McpServerInfo,
  SkillInfo,
  KnowledgeEntry,
  MemoryEntry,
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
    // Models / LLM
    list_models:     { method: "GET",    url: `/api/models` },
    active_model:    { method: "GET",    url: `/api/models/active` },
    switch_model:    { method: "POST",   url: `/api/models/switch`, body: { name: args?.name } },
    add_provider:    { method: "POST",   url: `/api/models/providers`, body: args?.config },
    remove_provider: { method: "DELETE", url: `/api/models/providers/${args?.name}` },
    // Plugins
    list_plugins:    { method: "GET",    url: `/api/plugins` },
    get_plugin:      { method: "GET",    url: `/api/plugins/${args?.pluginId}` },
    discover_plugins:{ method: "POST",   url: `/api/plugins/discover` },
    install_plugin:  { method: "POST",   url: `/api/plugins/${args?.pluginId}/install` },
    start_plugin:    { method: "POST",   url: `/api/plugins/${args?.pluginId}/start` },
    stop_plugin:     { method: "POST",   url: `/api/plugins/${args?.pluginId}/stop` },
    // Channels
    list_channels:   { method: "GET",    url: `/api/channels` },
    // MCP
    list_mcp_tools:  { method: "GET",    url: `/api/mcp/tools` },
    list_mcp_servers:{ method: "GET",    url: `/api/mcp/servers` },
    // Skills
    list_skills:     { method: "GET",    url: `/api/skills` },
    // Knowledge
    list_knowledge:  { method: "GET",    url: `/api/knowledge` },
    search_knowledge:{ method: "GET",    url: `/api/knowledge/search?q=${encodeURIComponent(String(args?.query || ""))}` },
    // Memory
    list_memory:     { method: "GET",    url: `/api/memory` },
    get_memory:      { method: "GET",    url: `/api/memory/${args?.key}` },
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

// ── Models / LLM ──

export async function listModels(): Promise<ProviderStatus[]> {
  return smartInvoke<ProviderStatus[]>("list_models");
}

export async function activeModel(): Promise<string> {
  return smartInvoke<string>("active_model");
}

export async function switchModel(name: string): Promise<string> {
  return smartInvoke<string>("switch_model", { name });
}

export async function addProvider(config: ProviderConfig): Promise<string> {
  return smartInvoke<string>("add_provider", { config });
}

export async function removeProvider(name: string): Promise<string> {
  return smartInvoke<string>("remove_provider", { name });
}

// ── Plugins ──

export async function listPlugins(): Promise<PluginStatus[]> {
  return smartInvoke<PluginStatus[]>("list_plugins");
}

export async function getPlugin(pluginId: string): Promise<PluginStatus | null> {
  return smartInvoke<PluginStatus | null>("get_plugin", { pluginId });
}

export async function discoverPlugins(): Promise<string> {
  return smartInvoke<string>("discover_plugins");
}

export async function installPlugin(pluginId: string): Promise<string> {
  return smartInvoke<string>("install_plugin", { pluginId });
}

export async function startPlugin(pluginId: string): Promise<string> {
  return smartInvoke<string>("start_plugin", { pluginId });
}

export async function stopPlugin(pluginId: string): Promise<string> {
  return smartInvoke<string>("stop_plugin", { pluginId });
}

// ── Channels ──

export async function listChannels(): Promise<ChannelStatus[]> {
  return smartInvoke<ChannelStatus[]>("list_channels");
}

// ── MCP ──

export async function listMcpTools(): Promise<McpTool[]> {
  return smartInvoke<McpTool[]>("list_mcp_tools");
}

export async function listMcpServers(): Promise<McpServerInfo[]> {
  return smartInvoke<McpServerInfo[]>("list_mcp_servers");
}

// ── Skills ──

export async function listSkills(): Promise<SkillInfo[]> {
  return smartInvoke<SkillInfo[]>("list_skills");
}

// ── Knowledge ──

export async function listKnowledge(): Promise<KnowledgeEntry[]> {
  return smartInvoke<KnowledgeEntry[]>("list_knowledge");
}

export async function searchKnowledge(query: string): Promise<KnowledgeEntry[]> {
  return smartInvoke<KnowledgeEntry[]>("search_knowledge", { query });
}

// ── Memory ──

export async function listMemory(): Promise<MemoryEntry[]> {
  return smartInvoke<MemoryEntry[]>("list_memory");
}

export async function getMemory(key: string): Promise<MemoryEntry | null> {
  return smartInvoke<MemoryEntry | null>("get_memory", { key });
}
