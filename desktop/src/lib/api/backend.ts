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
  PluginLogEntry,
  ChannelStatus,
  McpTool,
  McpServerInfo,
  SkillInfo,
  KnowledgeEntry,
  MemoryEntry,
  VikingStatus,
  ServiceConfig,
} from "./types";
import type { StoreUploadRequest, StoreUploadResult } from "./types";

const API_BASE = "http://localhost:8080";

// List of commands natively registered in the Tauri backend (`desktop/src-tauri/src/lib.rs`)
// All other commands (Phase 2: agents, models, plugins, etc) will intelligently fall back to the HTTP server!
const TAURI_COMMANDS = new Set([
  "health_check", "get_system_info", "get_resource_usage", "get_catalog",
  "list_apps", "get_app", "install_app", "uninstall_app", "upload_app",
  "launch_app", "stop_app", "open_app_browser",
  "verify_sandbox", "get_disk_usage"
]);

/**
 * Smart invoke: uses Tauri IPC when running inside the Tauri webview AND the command is registered,
 * falls back to the real HTTP API server otherwise.
 */
async function smartInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  // Use Tauri IPC when available
  if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window && TAURI_COMMANDS.has(command)) {
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
    recommend_models:{ method: "GET",    url: `/api/models/recommendations` },
    local_models:    { method: "GET",    url: `/api/models/local` },
    switch_model:    { method: "POST",   url: `/api/models/switch`, body: { name: args?.name } },
    add_provider:    { method: "POST",   url: `/api/models/providers`, body: args?.config },
    verify_provider: { method: "POST",   url: `/api/models/verify`, body: args?.config },
    remove_provider: { method: "DELETE", url: `/api/models/providers/${args?.name}` },
    codex_oauth_start: { method: "POST", url: `/api/codex/oauth/start`, body: { model: args?.model } },
    codex_oauth_status: { method: "GET", url: `/api/codex/oauth/status` },
    // Plugins
    list_plugins:    { method: "GET",    url: `/api/plugins` },
    get_plugin:      { method: "GET",    url: `/api/plugins/${args?.pluginId}` },
    discover_plugins:{ method: "POST",   url: `/api/plugins/discover` },
    install_plugin:  { method: "POST",   url: `/api/plugins/${args?.pluginId}/install` },
    start_plugin:    { method: "POST",   url: `/api/plugins/${args?.pluginId}/start` },
    stop_plugin:     { method: "POST",   url: `/api/plugins/${args?.pluginId}/stop` },
    plugin_logs:     { method: "GET",    url: `/api/plugins/${args?.pluginId}/logs` },
    clear_plugin_logs: { method: "DELETE", url: `/api/plugins/${args?.pluginId}/logs` },
    // Channels
    list_channels:   { method: "GET",    url: `/api/channels` },
    configure_channel: { method: "POST", url: `/api/channels/${args?.name}/configure`, body: { channel_type: args?.channel_type, enabled: args?.enabled, token: args?.token } },
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
    // OpenViking
    viking_status:   { method: "GET",    url: `/api/viking/status` },
    viking_install:  { method: "POST",   url: `/api/viking/install` },
    viking_start:    { method: "POST",   url: `/api/viking/start` },
    viking_stop:     { method: "POST",   url: `/api/viking/stop` },
    // Kanban
    kanban_get_tasks:   { method: "GET",    url: `/api/kanban/tasks` },
    kanban_create_task: { method: "POST",   url: `/api/kanban/tasks`, body: { title: args?.title, description: args?.description, checklist: args?.checklist } },
    kanban_update_task: { method: "PUT",    url: `/api/kanban/tasks/${args?.filename}`, body: { status: args?.status } },
    kanban_delete_task: { method: "DELETE", url: `/api/kanban/tasks/${args?.filename}` },
    kanban_get_content: { method: "GET",    url: `/api/kanban/tasks/${args?.filename}/content` },
    kanban_update_content: { method: "PUT", url: `/api/kanban/tasks/${args?.filename}/content`, body: { content: args?.content } },
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

export interface GgufModelRecommendation {
  id: string;
  name: string;
  url: string;
  description: string;
  size_gb: number;
  recommended_ram_gb: number;
}

export async function recommendModels(): Promise<GgufModelRecommendation[]> {
  return smartInvoke<GgufModelRecommendation[]>("recommend_models");
}

export interface LocalGgufModel {
  filename: string;
  path: string;
  size_bytes: number;
  size_display: string;
}

export async function localModels(): Promise<LocalGgufModel[]> {
  return smartInvoke<LocalGgufModel[]>("local_models");
}

export interface VerifyResult {
  ok: boolean;
  model_exists?: boolean;
  model_path?: string;
  server_available?: boolean;
  server_path?: string | null;
  error?: string | null;
}

export async function verifyProvider(config: ProviderConfig): Promise<VerifyResult> {
  return smartInvoke<VerifyResult>("verify_provider", { config });
}

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

// ── Codex OAuth ──

export interface CodexOAuthStartResult {
  auth_url: string;
  already_authenticated?: boolean;
  message?: string;
}

export interface CodexOAuthStatus {
  authenticated: boolean;
  email?: string;
  plan_type?: string;
}

export async function codexOAuthStart(model?: string): Promise<CodexOAuthStartResult> {
  return smartInvoke<CodexOAuthStartResult>("codex_oauth_start", { model });
}

export async function codexOAuthStatus(): Promise<CodexOAuthStatus> {
  return smartInvoke<CodexOAuthStatus>("codex_oauth_status");
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

export async function getPluginLogs(pluginId: string): Promise<PluginLogEntry[]> {
  return smartInvoke<PluginLogEntry[]>("plugin_logs", { pluginId });
}

export async function clearPluginLogs(pluginId: string): Promise<string> {
  return smartInvoke<string>("clear_plugin_logs", { pluginId });
}

// ── Channels ──

export async function listChannels(): Promise<ChannelStatus[]> {
  return smartInvoke<ChannelStatus[]>("list_channels");
}

export async function configureChannel(name: string, channelType: string, enabled: boolean, token: string, allowedUsers?: number[]): Promise<{success: boolean}> {
  return smartInvoke<{success: boolean}>("configure_channel", { name, channel_type: channelType, enabled, token, allowed_users: allowedUsers ?? [] });
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

// ── OpenViking ──

export async function vikingStatus(): Promise<VikingStatus> {
  return smartInvoke<VikingStatus>("viking_status");
}

export async function vikingInstall(): Promise<boolean> {
  return smartInvoke<boolean>("viking_install");
}

export async function vikingStart(): Promise<VikingStatus> {
  return smartInvoke<VikingStatus>("viking_start");
}

export async function vikingStop(): Promise<VikingStatus> {
  return smartInvoke<VikingStatus>("viking_stop");
}

// ── Service Config ──

export async function getServiceConfig(serviceId: string): Promise<ServiceConfig> {
  return smartInvoke<ServiceConfig>("service_hub_get_config", { serviceId });
}

export async function setServiceConfig(serviceId: string, values: Record<string, unknown>): Promise<string> {
  return smartInvoke<string>("service_hub_set_config", { serviceId, values });
}

// ── Kanban ──

export interface KanbanTask {
  filename: string;
  title: string;
  status: string;
  locked: boolean;
}

export async function kanbanGetTasks(): Promise<KanbanTask[]> {
  return smartInvoke<KanbanTask[]>("kanban_get_tasks");
}

export async function kanbanCreateTask(title: string, description?: string, checklist?: string[]): Promise<{ success: boolean; filename: string; title: string; message: string }> {
  return smartInvoke("kanban_create_task", { title, description, checklist });
}

export async function kanbanUpdateTask(filename: string, status: string): Promise<{ result: string }> {
  return smartInvoke("kanban_update_task", { filename, status });
}

export async function kanbanDeleteTask(filename: string): Promise<{ result: string }> {
  return smartInvoke("kanban_delete_task", { filename });
}

export async function kanbanGetContent(filename: string): Promise<{ content: string }> {
  return smartInvoke("kanban_get_content", { filename });
}

export async function kanbanUpdateContent(filename: string, content: string): Promise<{ result: string }> {
  return smartInvoke("kanban_update_content", { filename, content });
}
