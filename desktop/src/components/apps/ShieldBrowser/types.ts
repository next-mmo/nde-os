/** Shield Browser — shared type definitions. */

export interface ShieldProfile {
  id: string;
  name: string;
  engine: string;
  engine_version: string;
  is_running: boolean;
  last_launch: number | null;
  created_at: number;
  tags: string[];
  note: string | null;
  has_proxy: boolean;
  fingerprint_os: string | null;
}

export interface ShieldStatus {
  total_profiles: number;
  running_profiles: number;
  installed_engines: { engine: string; version: string }[];
}

export interface AvailableEngine {
  engine: string;
  name: string;
  description: string;
  available: boolean;
  icon: string;
}

export interface AdbDevice {
  serial: string;
  status: string;
  avd_name: string | null;
  is_emulator: boolean;
  display_name: string;
  device_type: "avd" | "ldplayer" | "nox" | "tcp" | "usb";
}

export interface AdbStatus {
  adb_available: boolean;
  emulator_available: boolean;
  adb_path: string | null;
  emulator_path: string | null;
}

export interface AvdInfo {
  name: string;
}

export type ShieldView = "setup" | "profiles" | "create" | "settings" | "devices" | "emulators";

// ─── LDPlayer Types ────────────────────────────────────────────────

export interface LdPlayerDetection {
  available: boolean;
  ldconsole_path: string | null;
  version_dir: string | null;
}

export interface LdPlayerInstance {
  index: number;
  name: string;
  is_running: boolean;
  pid: number;
  // DB metadata
  notes: string | null;
  tags: string[];
  linked_shield_profile_id: string | null;
  proxy_host: string | null;
  proxy_port: number | null;
  cpu: number | null;
  memory: number | null;
  resolution: string | null;
  created_at: number | null;
  updated_at: number | null;
}
