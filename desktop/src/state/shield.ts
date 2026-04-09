import { createStore } from "zustand/vanilla";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  ShieldProfile,
  ShieldStatus,
  AvailableEngine,
  AdbDevice,
  AdbStatus,
  AvdInfo,
  ShieldView,
} from "../components/apps/ShieldBrowser/types";

export interface ShieldBrowserState {
  loading: boolean;
  view: ShieldView;
  profiles: ShieldProfile[];
  status: ShieldStatus | null;
  availableEngines: AvailableEngine[];

  // Selection & UI
  selectedProfile: ShieldProfile | null;
  drawerOpen: boolean;

  // Create Form
  newName: string;
  newEngine: string;
  newVersion: string;
  resolvingVersion: boolean;

  // Setup
  setupStep: "choose" | "installing";
  setupEngine: string;
  setupVersion: string;
  setupError: string;

  // Downloads / Background ops
  downloading: boolean;
  downloadProgress: string;
  downloadPercent: number;
  launching: boolean;
  resolvingLatest: boolean;
  settingsLatestVersions: Record<string, string>;

  // Android Devices
  adbStatus: AdbStatus | null;
  androidDevices: AdbDevice[];
  avdList: AvdInfo[];
  devicesLoading: boolean;
  devicesError: string;
  showConnectDialog: boolean;
  connectAddress: string;
  selectedDevice: AdbDevice | null;
  deviceProxyHost: string;
  deviceProxyPort: string;
  deviceUrlInput: string;
  screenshotPath: string | null;
  deviceActionBusy: boolean;
}

export interface ShieldBrowserActions {
  // Initialization & Refresh
  init: () => Promise<void>;
  refresh: () => Promise<void>;
  setView: (view: ShieldView) => void;

  // Profile Management
  setSelectedProfile: (profile: ShieldProfile | null) => void;
  setDrawerOpen: (open: boolean) => void;
  setNewName: (name: string) => void;
  setNewEngine: (engine: string) => void;
  resolveCreateVersion: () => Promise<void>;
  createProfile: () => Promise<void>;
  deleteProfile: (id: string) => Promise<void>;
  launchProfile: (id: string) => Promise<void>;
  stopProfile: (id: string) => Promise<void>;

  // Engine Setup / Settings
  setSetupEngine: (engine: string) => void;
  setSetupVersion: (version: string) => void;
  resolveSetupVersion: () => Promise<void>;
  installEngine: () => Promise<void>;
  removeEngine: (engine: string, version: string) => Promise<void>;
  reinstallEngine: (engine: string, version: string) => Promise<void>;
  checkAvailableEngines: () => Promise<void>;
  openSettings: () => Promise<void>;

  // Device Management
  loadDevices: () => Promise<void>;
  setShowConnectDialog: (show: boolean) => void;
  setConnectAddress: (addr: string) => void;
  connectTcpDevice: () => Promise<void>;
  launchAvd: (name: string) => Promise<void>;
  stopDevice: (serial: string) => Promise<void>;
  setSelectedDevice: (device: AdbDevice | null) => void;
  setDeviceProxyHost: (host: string) => void;
  setDeviceProxyPort: (port: string) => void;
  setDeviceUrlInput: (url: string) => void;
  pushProxyToDevice: (serial: string) => Promise<void>;
  clearDeviceProxy: (serial: string) => Promise<void>;
  openUrlOnDevice: (serial: string) => Promise<void>;
  takeScreenshot: (serial: string) => Promise<void>;
  stopListenerTracker: () => void;
}

export const shieldBrowserStore = createStore<ShieldBrowserState & ShieldBrowserActions>()((set, get) => {
  let stopListenerCleanup: UnlistenFn | null = null;
  let hasInitProcessListener = false;

  const initProcessListener = () => {
    if (hasInitProcessListener) return;
    hasInitProcessListener = true;
    listen<string>("shield-profile-stopped", () => {
      console.log("Browser process exited, refreshing...");
      get().refresh();
    }).then((unlisten) => {
      stopListenerCleanup = unlisten;
    });
  };

  return {
    // ─── Initial State ───
    loading: true,
    view: "profiles",
    profiles: [],
    status: null,
    availableEngines: [],
    selectedProfile: null,
    drawerOpen: false,
    newName: "",
    newEngine: "camoufox",
    newVersion: "",
    resolvingVersion: false,
    setupStep: "choose",
    setupEngine: "camoufox",
    setupVersion: "",
    setupError: "",
    downloading: false,
    downloadProgress: "",
    downloadPercent: 0,
    launching: false,
    resolvingLatest: false,
    settingsLatestVersions: {},
    adbStatus: null,
    androidDevices: [],
    avdList: [],
    devicesLoading: false,
    devicesError: "",
    showConnectDialog: false,
    connectAddress: "127.0.0.1:5555",
    selectedDevice: null,
    deviceProxyHost: "",
    deviceProxyPort: "8080",
    deviceUrlInput: "https://browserleaks.com/canvas",
    screenshotPath: null,
    deviceActionBusy: false,

    // ─── Actions ───
    setView: (view) => set({ view }),
    setSelectedProfile: (p) => set({ selectedProfile: p }),
    setDrawerOpen: (open) => set({ drawerOpen: open, ...(open ? {} : { selectedProfile: null }) }),
    setNewName: (newName) => set({ newName }),
    setNewEngine: (newEngine) => {
      set({ newEngine });
      get().resolveCreateVersion();
    },
    setSetupEngine: (setupEngine) => {
      set({ setupEngine });
      get().resolveSetupVersion();
    },
    setSetupVersion: (setupVersion) => set({ setupVersion }),
    setShowConnectDialog: (v) => set({ showConnectDialog: v }),
    setConnectAddress: (v) => set({ connectAddress: v }),
    setSelectedDevice: (v) => set({ selectedDevice: v }),
    setDeviceProxyHost: (v) => set({ deviceProxyHost: v }),
    setDeviceProxyPort: (v) => set({ deviceProxyPort: v }),
    setDeviceUrlInput: (v) => set({ deviceUrlInput: v }),

    init: async () => {
      set({ loading: true });
      initProcessListener();
      // Safety timeout: never stay loading > 8 seconds
      const safetyTimer = setTimeout(() => {
        if (get().loading) {
          console.warn("Shield init timed out, forcing to setup view");
          set({ loading: false, view: "setup" });
        }
      }, 8000);
      try {
        const availableEngines = await invoke<AvailableEngine[]>("get_available_engines");
        const profiles = await invoke<ShieldProfile[]>("list_shield_profiles");
        const status = await invoke<ShieldStatus>("get_shield_status");

        set({ availableEngines, profiles, status });

        if (status.installed_engines.length === 0) {
          set({ view: "setup" });
          await get().resolveSetupVersion();
        } else {
          set({ view: "profiles" });
        }
        await get().resolveCreateVersion();
      } catch (e) {
        console.error("Shield init failed:", e);
        // Fall through to setup so user sees something, not a blank screen
        set({ profiles: [], status: { total_profiles: 0, running_profiles: 0, installed_engines: [] }, view: "setup" });
      } finally {
        clearTimeout(safetyTimer);
        set({ loading: false });
      }
    },

    checkAvailableEngines: async () => {
      try {
        const availableEngines = await invoke<AvailableEngine[]>("get_available_engines");
        set({ availableEngines });
      } catch (e) {
        console.error("Failed to check engines:", e);
      }
    },

    refresh: async () => {
      try {
        const profiles = await invoke<ShieldProfile[]>("list_shield_profiles");
        const status = await invoke<ShieldStatus>("get_shield_status");
        set({ profiles, status });

        const selected = get().selectedProfile;
        if (selected) {
          const updated = profiles.find((p) => p.id === selected.id);
          if (updated) set({ selectedProfile: updated });
        }
      } catch (e) {
        console.error("Failed to refresh:", e);
      }
    },

    resolveSetupVersion: async () => {
      try {
        const setupVersion = await invoke<string>("resolve_engine_version", { engine: get().setupEngine });
        set({ setupVersion });
      } catch (e) {
        set({ setupVersion: "", setupError: `Failed to resolve version: ${e}` });
      }
    },

    resolveCreateVersion: async () => {
      set({ resolvingVersion: true });
      try {
        const newVersion = await invoke<string>("resolve_engine_version", { engine: get().newEngine });
        set({ newVersion });
      } catch (e) {
        set({ newVersion: "" });
      } finally {
        set({ resolvingVersion: false });
      }
    },

    installEngine: async () => {
      const state = get();
      if (!state.setupVersion) {
        set({ setupError: "No version resolved. Check your internet connection." });
        return;
      }
      set({
        setupStep: "installing",
        setupError: "",
        downloading: true,
        downloadPercent: 0,
        downloadProgress: `Downloading binary…`,
      });

      let unlisten: UnlistenFn | null = null;
      try {
        unlisten = await listen<{ downloaded: number; total: number; percent: number }>(
          "shield-download-progress",
          (event) => {
            const { downloaded, total, percent } = event.payload;
            const dlMB = (downloaded / 1048576).toFixed(1);
            const totalMB = total > 0 ? (total / 1048576).toFixed(0) : "?";
            set({
              downloadPercent: percent,
              downloadProgress: `Downloading — ${percent}%  (${dlMB} MB / ${totalMB} MB)`
            });
          }
        );

        await invoke("download_shield_engine", { engine: state.setupEngine, version: state.setupVersion });
        set({ downloadPercent: 100, downloadProgress: "Extracting & installing…" });
        await get().refresh();
        set({ downloadProgress: "Installation complete!", downloading: false, setupStep: "choose" });
        setTimeout(() => set({ view: "profiles" }), 800);
        get().resolveCreateVersion();
      } catch (e: unknown) {
        set({
          setupError: `Download failed: ${e}`,
          setupStep: "choose",
          downloading: false,
          downloadPercent: 0,
          downloadProgress: "",
        });
      } finally {
        unlisten?.();
      }
    },

    removeEngine: async (engine, version) => {
      try {
        await invoke("remove_shield_engine", { engine, version });
        await get().refresh();
      } catch (e: unknown) {
        alert(`Failed to remove engine: ${e}`);
      }
    },

    reinstallEngine: async (engine, version) => {
      await get().removeEngine(engine, version);
      const prevSetupEngine = get().setupEngine;
      const prevSetupVersion = get().setupVersion;
      
      set({ setupEngine: engine, setupVersion: version });
      await get().installEngine();
      
      set({ setupEngine: prevSetupEngine, setupVersion: prevSetupVersion });
      await get().refresh();
    },

    openSettings: async () => {
      set({ view: "settings", resolvingLatest: true, settingsLatestVersions: {} });
      const status = get().status;
      if (!status) return;

      try {
        const resolved: Record<string, string> = {};
        for (const eng of status.installed_engines) {
          try {
            const latest = await invoke<string>("resolve_engine_version", { engine: eng.engine });
            resolved[eng.engine] = latest;
          } catch { /* skip */ }
        }
        set({ settingsLatestVersions: resolved });
      } finally {
        set({ resolvingLatest: false });
      }
    },

    createProfile: async () => {
      const state = get();
      if (!state.newName.trim() || !state.newVersion) return;
      set({ resolvingVersion: true }); // hijack for loading spinner
      try {
        await invoke("create_shield_profile", {
          name: state.newName.trim(),
          engine: state.newEngine,
          engineVersion: state.newVersion,
          tags: [],
          note: null,
          fingerprintOs: null,
        });
        set({ newName: "", view: "profiles" });
        await get().refresh();
      } catch (e: unknown) {
        alert(`Failed to create profile: ${e}`);
      } finally {
        set({ resolvingVersion: false });
      }
    },

    deleteProfile: async (id) => {
      try {
        await invoke("delete_shield_profile", { id });
        set({ selectedProfile: null });
        await get().refresh();
      } catch (e: unknown) {
        alert(`Failed to delete profile: ${e}`);
      }
    },

    launchProfile: async (id) => {
      set({ launching: true });
      try {
        await invoke("launch_shield_profile", { id });
        await get().refresh();
      } catch (e: unknown) {
        alert(`Failed to launch profile: ${e}`);
      } finally {
        set({ launching: false });
      }
    },

    stopProfile: async (id) => {
      try {
        await invoke("stop_shield_profile", { id });
        await get().refresh();
      } catch (e: unknown) {
        alert(`Failed to stop profile: ${e}`);
      }
    },

    // ─── Devices ───
    loadDevices: async () => {
      set({ devicesLoading: true, devicesError: "" });
      try {
        const adbStatus = await invoke<AdbStatus>("shield_adb_status");
        if (!adbStatus.adb_available) {
          set({ devicesError: "ADB not found. Install Platform-Tools.", androidDevices: [], avdList: [], adbStatus });
          return;
        }
        const androidDevices = await invoke<AdbDevice[]>("shield_list_android_devices");
        let avdList: AvdInfo[] = [];
        if (adbStatus.emulator_available) {
          avdList = await invoke<AvdInfo[]>("shield_list_avds");
        }
        set({ adbStatus, androidDevices, avdList });
      } catch (e: unknown) {
        set({ devicesError: `${e}` });
      } finally {
        set({ devicesLoading: false });
      }
    },

    connectTcpDevice: async () => {
      const state = get();
      if (!state.connectAddress.trim()) return;
      set({ deviceActionBusy: true });
      try {
        await invoke("shield_adb_connect", { address: state.connectAddress.trim() });
        set({ showConnectDialog: false });
        await get().loadDevices();
      } catch (e: unknown) {
        alert(`Failed to connect: ${e}`);
      } finally {
        set({ deviceActionBusy: false });
      }
    },

    launchAvd: async (name) => {
      set({ deviceActionBusy: true });
      try {
        await invoke("shield_launch_avd", { avdName: name });
        setTimeout(() => get().loadDevices(), 3000);
      } catch (e: unknown) {
        alert(`Failed to launch AVD: ${e}`);
      } finally {
        set({ deviceActionBusy: false });
      }
    },

    stopDevice: async (serial) => {
      set({ deviceActionBusy: true });
      try {
        await invoke("shield_stop_device", { serial });
        await get().loadDevices();
        if (get().selectedDevice?.serial === serial) set({ selectedDevice: null });
      } catch (e: unknown) {
        alert(`Failed to stop device: ${e}`);
      } finally {
        set({ deviceActionBusy: false });
      }
    },

    pushProxyToDevice: async (serial) => {
      const state = get();
      if (!state.deviceProxyHost.trim()) return;
      set({ deviceActionBusy: true });
      try {
        await invoke("shield_configure_proxy", {
          serial,
          host: state.deviceProxyHost.trim(),
          port: parseInt(state.deviceProxyPort) || 8080,
        });
        alert(`Proxy set on ${serial}`);
      } catch (e: unknown) {
        alert(`Failed to set proxy: ${e}`);
      } finally {
        set({ deviceActionBusy: false });
      }
    },

    clearDeviceProxy: async (serial) => {
      set({ deviceActionBusy: true });
      try {
        await invoke("shield_clear_proxy", { serial });
        alert(`Proxy cleared on ${serial}`);
      } catch (e: unknown) {
        alert(`Failed to clear proxy: ${e}`);
      } finally {
        set({ deviceActionBusy: false });
      }
    },

    openUrlOnDevice: async (serial) => {
      const url = get().deviceUrlInput.trim();
      if (!url) return;
      try {
        set({ deviceActionBusy: true });
        await invoke("shield_open_url_on_device", { serial, url });
      } catch (e: unknown) {
        alert(`Failed to open URL: ${e}`);
      } finally {
        set({ deviceActionBusy: false });
      }
    },

    takeScreenshot: async (serial) => {
      const state = get();
      if (state.deviceActionBusy) return;
      set({ deviceActionBusy: true, screenshotPath: null });
      try {
        const path = await invoke<string>("shield_take_screenshot", { serial });
        set({ screenshotPath: path });
        setTimeout(() => set({ screenshotPath: null }), 5000);
      } catch (e: unknown) {
        alert(`Screenshot failed: ${e}`);
      } finally {
        set({ deviceActionBusy: false });
      }
    },

    stopListenerTracker: () => {
      stopListenerCleanup?.();
      stopListenerCleanup = null;
      hasInitProcessListener = false;
    }
  };
});
