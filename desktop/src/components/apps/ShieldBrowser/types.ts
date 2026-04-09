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

export type ShieldView = "setup" | "profiles" | "create" | "settings" | "devices";
