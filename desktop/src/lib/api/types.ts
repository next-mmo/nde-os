// TypeScript interfaces matching Rust structs from ai-launcher-core

export interface AppManifest {
  id: string;
  name: string;
  description: string;
  author: string;
  repo?: string;
  python_version: string;
  needs_gpu: boolean;
  pip_deps: string[];
  launch_cmd: string;
  port: number;
  env: [string, string][];
  disk_size: string;
  tags: string[];
  icon?: string;
}

export type AppStatusState = "NotInstalled" | "Installing" | "Installed" | "Running" | "Error";

export interface AppStatus {
  state: AppStatusState;
  pid?: number;
  port?: number;
  message?: string;
}

export interface InstalledApp {
  manifest: AppManifest;
  status: AppStatus;
  workspace: string;
  installed_at?: string;
  last_run?: string;
}

export interface SandboxVerifyResult {
  path_traversal_blocked: boolean;
  absolute_escape_blocked: boolean;
  symlink_escape_blocked: boolean;
  valid_path_works: boolean;
  sandbox_root: string;
  platform: string;
}

export interface DiskUsage {
  app_id: string;
  bytes: number;
  human_readable: string;
}

export type SourceType = 'folder' | 'zip' | 'git_url';

export interface StoreUploadRequest {
  source_type: SourceType;
  source_path?: string;
  git_url?: string;
}

export interface ValidationError {
  field: string;
  message: string;
}

export interface StoreUploadResult {
  accepted: boolean;
  app_id?: string;
  app_name?: string;
  validation_errors: ValidationError[];
  install_log: string[];
}

export interface LaunchResult {
  pid: number;
  port: number;
}

export interface SystemInfo {
  os: string;
  arch: string;
  python_version: string | null;
  gpu_detected: boolean;
  uv: { uv_path: string; uv_version: string };
  base_dir: string;
  total_apps: number;
  running_apps: number;
}

export interface ResourceUsage {
  memory_used_bytes: number;
  memory_total_bytes: number;
  memory_percent: number;
  disk_used_bytes: number;
  disk_total_bytes: number;
  disk_percent: number;
  disk_mount_point: string;
}

export interface LogEntry {
  id: number;
  timestamp: string;
  level: "info" | "success" | "warning" | "error";
  message: string;
  app_id?: string;
}

// ── Agent Chat API types ─────────────────────────────────────────────────────

export interface ChatRequest {
  message: string;
  conversation_id?: string;
}

export interface ChatResponse {
  response: string;
  conversation_id: string;
}

export interface ConversationSummary {
  id: string;
  title: string;
  channel: string;
  created_at: string;
  updated_at: string;
}

export interface StoredMessage {
  id: number;
  role: string;
  content: string | null;
  tool_calls: string | null;
  tool_call_id: string | null;
  created_at: string;
}

export interface AgentConfigInfo {
  name: string;
  provider: string;
  model: string;
  max_iterations: number;
  tools: string[];
  workspace: string;
}

// ── LLM / Model Management types ─────────────────────────────────────────────

export interface ProviderStatus {
  name: string;
  provider_type: string;
  is_active: boolean;
}

export interface ProviderConfig {
  name: string;
  provider_type: string;
  model: string;
  base_url?: string;
  api_key?: string;
  api_key_env?: string;
  max_tokens: number;
}

// ── Plugin types ─────────────────────────────────────────────────────────────

export type PluginState = "discovered" | "installed" | "running" | "stopped" | "error";
export type PluginType = "monitor" | "hook" | "provider" | "tool" | "ui_panel" | "daemon";
export type HookType = "on_message" | "on_response" | "on_tool_call" | "on_tool_result"
  | "on_error" | "on_start" | "on_stop" | "on_install" | "on_config_change";

export interface PluginStatus {
  id: string;
  name: string;
  version: string;
  plugin_type: PluginType;
  state: PluginState;
  pid?: number;
  port?: number;
  hooks: HookType[];
}

// ── Channel / Gateway types ──────────────────────────────────────────────────

export type ChannelType = "rest" | "telegram" | "discord" | "slack" | "web_chat" | "cli";

export interface ChannelStatus {
  name: string;
  channel_type: ChannelType;
  is_running: boolean;
  messages_received: number;
  messages_sent: number;
}

// ── MCP types ────────────────────────────────────────────────────────────────

export interface McpTool {
  name: string;
  description: string;
  parameters: Record<string, unknown>;
}

export interface McpServerInfo {
  name: string;
  transport: string;
  tools_count: number;
  is_connected: boolean;
}

// ── Skills types ─────────────────────────────────────────────────────────────

export interface SkillInfo {
  name: string;
  description: string;
  path: string;
  triggers: string[];
}

// ── Knowledge types ──────────────────────────────────────────────────────────

export interface KnowledgeEntry {
  id: string;
  key: string;
  value: string;
  category: string;
  created_at: string;
  updated_at: string;
}

// ── Memory types ─────────────────────────────────────────────────────────────

export interface MemoryEntry {
  key: string;
  value: string;
  created_at: string;
}
