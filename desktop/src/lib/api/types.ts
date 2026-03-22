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
