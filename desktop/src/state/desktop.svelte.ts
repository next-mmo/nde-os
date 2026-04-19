import { apps_config, type StaticAppID } from "🍎/configs/apps/apps-config";
import type { InstalledApp } from "$lib/api/types";

export type WindowAppID = Exclude<StaticAppID, "launchpad">;
export type ThemeScheme = "light" | "dark";
export type LauncherSection = "overview" | "catalog" | "installed" | "running" | "server" | "command-center" | "chat" | "vibe-studio" | "model-settings" | "plugins" | "channels" | "mcp-tools" | "skills" | "knowledge" | "code-editor" | "architecture";
export type SessionMode = "embedded" | "windowed" | "drawer-left" | "drawer-right" | "fullscreen";
export type DesktopIconPosition = { x: number; y: number };

export type RunningSession = {
  id: string;
  app_id: string;
  title: string;
  url: string;
  port: number;
  mode: SessionMode;
  window_id: string | null;
  load_state: "idle" | "loading" | "ready" | "fallback";
  last_focused_at: string;
  pinned: boolean;
};

export type BrowserState = {
  history: string[];
  index: number;
  reload_key: number;
};

export interface DesktopWindow {
  id: string;
  app_id: WindowAppID | "browser" | "chat";
  title: string;
  width: number;
  height: number;
  z_index: number;
  minimized: boolean;
  fullscreen: boolean;
  resizable: boolean;
  expandable: boolean;
  closable: boolean;
  session_id: string | null;
  browser: BrowserState | null;
  data?: any;
}

export interface DesktopNotification {
  id: string;
  app: string;
  title: string;
  message: string;
  time: string;
  icon: string;
  action?: string;
}

type WorkspaceView =
  | { kind: "dashboard" }
  | { kind: "session"; session_id: string };

const makeWindowId = (prefix: string) =>
  `${prefix}-${Math.random().toString(36).slice(2, 9)}-${Date.now().toString(36)}`;

type SavedWindowGeometry = { x: number; y: number; width: number; height: number; fullscreen: boolean };

const GEOMETRY_STORAGE_KEY = "ai-launcher:window-geometry";
const OPEN_WINDOWS_STORAGE_KEY = "ai-launcher:open-windows";
const ICON_POSITIONS_STORAGE_KEY = "ai-launcher:icon-positions";
const HIDDEN_ICONS_STORAGE_KEY = "ai-launcher:hidden-icons";

function loadIconPositions(): Record<string, DesktopIconPosition> {
  try {
    const raw = localStorage.getItem(ICON_POSITIONS_STORAGE_KEY);
    if (raw) return JSON.parse(raw);
  } catch {}
  return {};
}

function loadHiddenIcons(): string[] {
  try {
    const raw = localStorage.getItem(HIDDEN_ICONS_STORAGE_KEY);
    if (raw) return JSON.parse(raw);
  } catch {}
  return [];
}

function loadAllGeometry(): Record<string, SavedWindowGeometry> {
  try {
    const raw = localStorage.getItem(GEOMETRY_STORAGE_KEY);
    if (raw) return JSON.parse(raw);
  } catch {}
  return {};
}

export function loadWindowGeometry(app_id: string): SavedWindowGeometry | null {
  const all = loadAllGeometry();
  return all[app_id] ?? null;
}

export function saveWindowGeometry(app_id: string, geo: SavedWindowGeometry) {
  try {
    const all = loadAllGeometry();
    all[app_id] = geo;
    localStorage.setItem(GEOMETRY_STORAGE_KEY, JSON.stringify(all));
  } catch {}
}

export function saveWindowFullscreen(app_id: string, fullscreen: boolean) {
  try {
    const all = loadAllGeometry();
    const existing = all[app_id];
    if (existing) {
      existing.fullscreen = fullscreen;
    } else {
      all[app_id] = { x: 0, y: 0, width: 0, height: 0, fullscreen };
    }
    localStorage.setItem(GEOMETRY_STORAGE_KEY, JSON.stringify(all));
  } catch {}
}

function saveOpenWindows() {
  try {
    const appIds = desktop.windows.map((w) => w.app_id);
    localStorage.setItem(OPEN_WINDOWS_STORAGE_KEY, JSON.stringify(appIds));
  } catch {}
}

function loadOpenWindows(): string[] {
  try {
    const raw = localStorage.getItem(OPEN_WINDOWS_STORAGE_KEY);
    if (raw) return JSON.parse(raw);
  } catch {}
  return [];
}

export type DefaultSessionMode = "embedded" | "windowed";

function getSavedSessionMode(): DefaultSessionMode {
  try {
    const saved = localStorage.getItem("ai-launcher:default-mode");
    if (saved === "embedded" || saved === "windowed") return saved as DefaultSessionMode;
  } catch {}
  return "windowed";
}

function getSavedTheme(): ThemeScheme {
  try {
    const saved = localStorage.getItem("ai-launcher:theme");
    if (saved === "light" || saved === "dark") return saved as ThemeScheme;
    
    // Fallback to system preference if no saved preference
    if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
      return "dark";
    }
  } catch {}
  return "light";
}

function getSavedLauncherSection(): LauncherSection {
  try {
    const saved = localStorage.getItem("ai-launcher:launcher-section");
    const validSections: LauncherSection[] = ["overview", "catalog", "installed", "running", "server", "command-center", "chat", "vibe-studio", "model-settings", "plugins", "channels", "mcp-tools", "skills", "knowledge", "code-editor", "architecture"];
    if (validSections.includes(saved as LauncherSection)) {
      return saved as LauncherSection;
    }
  } catch {}
  return "overview";
}

function getSavedDockAutoHide(): boolean {
  try {
    const saved = localStorage.getItem("ai-launcher:dock-auto-hide");
    if (saved === "true" || saved === "false") return saved === "true";
  } catch {}
  return false;
}

function getSavedStartExpanded(): boolean {
  try {
    const saved = localStorage.getItem("ai-launcher:start-expanded");
    if (saved === "true" || saved === "false") return saved === "true";
  } catch {}
  return true; // default: start expanded
}

function getSavedCollapsed(): boolean {
  // In Tauri dev mode, always start expanded — FAB toggle is not useful during development
  // and causes E2E test failures since tests expect the full desktop UI.
  if (import.meta.env.DEV) return false;
  if (getSavedStartExpanded()) return false; // always expanded on boot
  // Otherwise, restore last session state
  try {
    const saved = localStorage.getItem("ai-launcher:collapsed");
    if (saved === "true" || saved === "false") return saved === "true";
  } catch {}
  return false;
}

function getSavedIsLocked(): boolean {
  try {
    const saved = localStorage.getItem("ai-launcher:is-locked");
    if (saved === "true" || saved === "false") return saved === "true";
  } catch {}
  return false;
}

function getSavedWallpaper(): string {
  try {
    const saved = localStorage.getItem("ai-launcher:wallpaper");
    // Blob URLs are session-scoped — they die on page unload, never restore them
    if (saved && !saved.includes("blob:")) return saved;
  } catch {}
  return 'url("/wallpapers/ventura-1.webp")';
}

function getSavedVikingInstalled(): boolean {
  try {
    return localStorage.getItem("ai-launcher:viking-installed") === "true";
  } catch {}
  return false;
}

const createWindow = (
  app_id: WindowAppID | "browser" | "chat",
  title: string,
  width: number,
  height: number,
): DesktopWindow => ({
  id: makeWindowId(app_id),
  app_id,
  title,
  width,
  height,
  z_index: 0,
  minimized: false,
  fullscreen: true,
  resizable: true,
  expandable: true,
  closable: true,
  session_id: null,
  browser: null,
});

export type DrawerState = {
  session_id: string;
  side: "left" | "right";
};

export const desktop = $state({
  theme: getSavedTheme(),
  collapsed: getSavedCollapsed(),
  start_expanded: getSavedStartExpanded(),
  launchpad_open: false,
  launcher_section: getSavedLauncherSection(),
  launcher_query: "",
  selected_app_id: null as string | null,
  selected_session_id: null as string | null,
  workspace_view: { kind: "dashboard" } as WorkspaceView,
  windows: [] as DesktopWindow[],
  sessions: [] as RunningSession[],
  next_z_index: 10,
  default_session_mode: getSavedSessionMode(),
  dock_auto_hide: getSavedDockAutoHide(),
  drawer: null as DrawerState | null,
  fullscreen_session_id: null as string | null,
  icon_positions: loadIconPositions() as Record<string, DesktopIconPosition>,
  icon_selection: new Set<string>(),
  hidden_icons: new Set<string>(loadHiddenIcons()),
  is_locked: getSavedIsLocked(),
  spotlight_open: false,
  notification_center_open: false,
  log_drawer_open: false,
  wallpaper: getSavedWallpaper(),
  notifications: [] as DesktopNotification[],
  viking_onboard_state: null as { stage: "installing" | "starting" | "ready" | "error"; message: string } | null,
  viking_is_installed: getSavedVikingInstalled(),
});

export function addNotification(notif: Omit<DesktopNotification, "id" | "time">) {
  const newNotif = {
    ...notif,
    id: Math.random().toString(36).substring(2, 9),
    time: new Intl.DateTimeFormat('en-US', { hour: 'numeric', minute: 'numeric' }).format(new Date()),
  };
  desktop.notifications = [newNotif, ...desktop.notifications];
}

export function setVikingInstalled() {
  desktop.viking_is_installed = true;
  try { localStorage.setItem("ai-launcher:viking-installed", "true"); } catch {}
}

export function toggleStartExpanded() {
  desktop.start_expanded = !desktop.start_expanded;
  try {
    localStorage.setItem("ai-launcher:start-expanded", String(desktop.start_expanded));
  } catch {}
}

export function setWallpaper(bg: string) {
  desktop.wallpaper = bg;
  // Blob URLs are session-scoped — never persist them to storage or they will
  // produce broken wallpapers on the next boot session.
  if (!bg.includes("blob:")) {
    try {
      localStorage.setItem("ai-launcher:wallpaper", bg);
    } catch {}
  }
}

export function lockScreen() {
  desktop.is_locked = true;
  desktop.launchpad_open = false;
  try {
    localStorage.setItem("ai-launcher:is-locked", "true");
  } catch {}
}

export function unlockScreen() {
  desktop.is_locked = false;
  try {
    localStorage.setItem("ai-launcher:is-locked", "false");
  } catch {}
}

export function toggleDockAutoHide() {
  desktop.dock_auto_hide = !desktop.dock_auto_hide;
  try {
    localStorage.setItem("ai-launcher:dock-auto-hide", String(desktop.dock_auto_hide));
  } catch {}
}

export async function expandDesktop() {
  desktop.collapsed = false;
  try { localStorage.setItem("ai-launcher:collapsed", "false"); } catch {}

  // Save current FAB position before expanding
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow();
    const pos = await win.outerPosition();
    const scale = (await win.scaleFactor()) ?? 1;
    localStorage.setItem("ai-launcher:fab-position", JSON.stringify({
      x: pos.x / scale,
      y: pos.y / scale,
    }));
  } catch (_) {}

  // Restore saved window geometry or default to maximized
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const { LogicalSize, LogicalPosition } = await import("@tauri-apps/api/dpi");
    const win = getCurrentWindow();

    await win.setAlwaysOnTop(false);
    await win.setSkipTaskbar(false);
    await win.setMinSize(new LogicalSize(900, 600));

    const saved = loadWindowGeometry("__tauri-main__");
    if (saved && saved.width >= 900 && saved.height >= 600) {
      await win.setSize(new LogicalSize(saved.width, saved.height));
      await win.setPosition(new LogicalPosition(saved.x, saved.y));
      if (saved.fullscreen) {
        await win.setFullscreen(true);
      }
    } else {
      // First run: maximize
      await win.setSize(new LogicalSize(1100, 750));
      await win.center();
      await win.setFullscreen(true);
    }
  } catch (_) {
    // Not in Tauri
  }
}

/** Save the current Tauri window geometry so it can be restored after collapse */
async function saveMainWindowGeometry() {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow();
    const size = await win.innerSize();
    const pos = await win.outerPosition();
    const scale = (await win.scaleFactor()) ?? 1;
    const fs = await win.isFullscreen();
    saveWindowGeometry("__tauri-main__", {
      x: pos.x / scale,
      y: pos.y / scale,
      width: size.width / scale,
      height: size.height / scale,
      fullscreen: fs,
    });
  } catch (_) {}
}

const TAB_WIDTH = 24;
const TAB_HEIGHT = 48;

function loadFabY(): number | null {
  try {
    const raw = localStorage.getItem("ai-launcher:fab-y");
    if (raw) {
      const y = JSON.parse(raw);
      if (typeof y === "number") return y;
    }
  } catch (_) {}
  return null;
}

export async function saveFabPosition() {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow();
    const pos = await win.outerPosition();
    const scale = (await win.scaleFactor()) ?? 1;
    localStorage.setItem("ai-launcher:fab-y", JSON.stringify(pos.y / scale));
  } catch (_) {}
}

export async function collapseDesktop() {
  // In dev mode, collapsing is disabled — the desktop stays expanded for development and testing
  if (import.meta.env.DEV) return;

  // Save current window geometry before shrinking
  await saveMainWindowGeometry();

  desktop.collapsed = true;
  try { localStorage.setItem("ai-launcher:collapsed", "true"); } catch {}

  // Shrink to right-edge tab
  try {
    const { getCurrentWindow, primaryMonitor } = await import("@tauri-apps/api/window");
    const { LogicalSize, LogicalPosition } = await import("@tauri-apps/api/dpi");
    const win = getCurrentWindow();

    const monitor = await primaryMonitor();
    if (!monitor) return;

    const scale = monitor.scaleFactor;
    const screenW = monitor.size.width / scale;
    const screenH = monitor.size.height / scale;

    await win.setMinSize(new LogicalSize(TAB_WIDTH, TAB_HEIGHT));
    await win.setSize(new LogicalSize(TAB_WIDTH, TAB_HEIGHT));

    // Restore saved Y or default to center-right; validate within screen bounds
    const centerY = screenH / 2 - TAB_HEIGHT / 2;
    const savedY = loadFabY();
    const y = (savedY !== null && savedY >= 40 && savedY <= screenH - TAB_HEIGHT - 40)
      ? savedY
      : centerY;
    await win.setPosition(new LogicalPosition(screenW - TAB_WIDTH, y));

    await win.setAlwaysOnTop(true);
    await win.setSkipTaskbar(true);
  } catch (_) {
    // Not in Tauri
  }
}

export function toggleDefaultSessionMode() {
  desktop.default_session_mode = desktop.default_session_mode === "windowed" ? "embedded" : "windowed";
  try {
    localStorage.setItem("ai-launcher:default-mode", desktop.default_session_mode);
  } catch {}
}

function nextZIndex() {
  desktop.next_z_index += 2;
  return desktop.next_z_index;
}

function assignWindowFocus(window: DesktopWindow) {
  window.z_index = nextZIndex();
  window.minimized = false;
}

function topVisibleWindow(excludeWindowId?: string) {
  return [...desktop.windows]
    .filter((item) => !item.minimized && item.id !== excludeWindowId)
    .sort((left, right) => right.z_index - left.z_index)[0] ?? null;
}

export function bootDesktop() {
  if (desktop.windows.length > 0) return;

  // Restore previously open windows from localStorage (deduplicated)
  const savedAppIds = loadOpenWindows();
  const hasSavedState = savedAppIds.length > 0;
  const validStaticIds = Object.keys(apps_config) as StaticAppID[];
  const appsToRestore = [...new Set(savedAppIds.filter(
    (id) => validStaticIds.includes(id as StaticAppID) && id !== "launchpad",
  ))];

  // Do not auto-open the launcher on first startup — let the user open it manually

  for (const appId of appsToRestore) {
    const config = apps_config[appId as Exclude<StaticAppID, "launchpad">];
    if (!config) continue;
    const savedGeo = loadWindowGeometry(appId);
    const win = createWindow(
      appId as WindowAppID,
      config.title,
      savedGeo && savedGeo.width >= 200 ? savedGeo.width : config.width!,
      savedGeo && savedGeo.height >= 200 ? savedGeo.height : config.height!,
    );
    win.resizable = config.resizable!;
    win.expandable = config.expandable!;
    win.fullscreen = true;
    assignWindowFocus(win);
    desktop.windows.push(win);
  }
}

export function toggleTheme() {
  desktop.theme = desktop.theme === "light" ? "dark" : "light";
  try {
    localStorage.setItem("ai-launcher:theme", desktop.theme);
  } catch {}
}

export function toggleLaunchpad(force?: boolean) {
  desktop.launchpad_open = force ?? !desktop.launchpad_open;
}

export function toggleSpotlight(force?: boolean) {
  desktop.spotlight_open = force ?? !desktop.spotlight_open;
}

export function toggleNotificationCenter(force?: boolean) {
  desktop.notification_center_open = force ?? !desktop.notification_center_open;
}

export function toggleLogDrawer(force?: boolean) {
  desktop.log_drawer_open = force ?? !desktop.log_drawer_open;
}

export function focusWindow(window_id: string) {
  const window = desktop.windows.find((item) => item.id === window_id);
  if (!window) return;
  assignWindowFocus(window);
  if (window.session_id) {
    desktop.selected_session_id = window.session_id;
  }
}

export function closeWindow(window_id: string) {
  const index = desktop.windows.findIndex((item) => item.id === window_id);
  if (index < 0) return;
  const [window] = desktop.windows.splice(index, 1);
  if (window.session_id) {
    const session = desktop.sessions.find((item) => item.id === window.session_id);
    if (session) {
      session.window_id = null;
      session.mode = "embedded";
      desktop.workspace_view = { kind: "session", session_id: session.id };
    }
  }
  saveOpenWindows();
}

export function minimizeWindow(window_id: string) {
  const window = desktop.windows.find((item) => item.id === window_id);
  if (!window) return;
  const wasActive = activeWindow()?.id === window_id;
  window.minimized = true;
  if (wasActive) {
    const nextWindow = topVisibleWindow(window_id);
    if (nextWindow) {
      assignWindowFocus(nextWindow);
      if (nextWindow.session_id) {
        desktop.selected_session_id = nextWindow.session_id;
      }
    }
  }
}

export function toggleFullscreen(window_id: string) {
  const window = desktop.windows.find((item) => item.id === window_id);
  if (!window) return;
  window.fullscreen = !window.fullscreen;
  assignWindowFocus(window);
  saveWindowFullscreen(window.app_id, window.fullscreen);
}

export function openStaticApp(app_id: StaticAppID, data?: any) {
  if (app_id === "launchpad") {
    toggleLaunchpad();
    return;
  }
  if (app_id === "browser") {
    const existing = [...desktop.windows]
      .sort((a, b) => b.z_index - a.z_index)
      .find((item) => item.app_id === "browser");
    if (existing) {
      if (data) existing.data = data;
      focusWindow(existing.id);
      return existing;
    }
    return openGenericBrowserWindow();
  }

  const existing = desktop.windows.find((item) => item.app_id === app_id);
  if (existing) {
    if (data) existing.data = data;
    focusWindow(existing.id);
    return existing;
  }

  const config = apps_config[app_id];
  const savedGeo = loadWindowGeometry(app_id);
  const window = createWindow(app_id, config.title, savedGeo?.width && savedGeo.width >= 200 ? savedGeo.width : config.width!, savedGeo?.height && savedGeo.height >= 200 ? savedGeo.height : config.height!);
  window.resizable = config.resizable!;
  window.expandable = config.expandable!;
  if (data) window.data = data;
  assignWindowFocus(window);
  desktop.windows.push(window);
  saveOpenWindows();
  return window;
}

export function selectLauncherSection(section: LauncherSection) {
  desktop.launcher_section = section;
  try {
    localStorage.setItem("ai-launcher:launcher-section", section);
  } catch {}
  desktop.workspace_view = { kind: "dashboard" };
  desktop.selected_session_id = null;
  openStaticApp("ai-launcher");
}

export function selectManifest(app_id: string | null) {
  desktop.selected_app_id = app_id;
}

export function upsertSession({
  app_id,
  title,
  port,
  mode,
}: {
  app_id: string;
  title: string;
  port: number;
  mode: SessionMode;
}) {
  const url = `http://localhost:${port}`;
  let session = desktop.sessions.find((item) => item.app_id === app_id);
  if (!session) {
    session = {
      id: makeWindowId(app_id),
      app_id,
      title,
      url,
      port,
      mode,
      window_id: null,
      load_state: "idle",
      last_focused_at: new Date().toISOString(),
      pinned: false,
    };
    desktop.sessions.unshift(session);
  } else {
    session.title = title;
    session.port = port;
    session.url = url;
    session.mode = mode;
    session.last_focused_at = new Date().toISOString();
  }
  desktop.selected_session_id = session.id;
  if (mode === "embedded") {
    desktop.workspace_view = { kind: "session", session_id: session.id };
    openStaticApp("ai-launcher");
  }
  return session;
}

export function openSessionInDashboard(session_id: string) {
  const session = desktop.sessions.find((item) => item.id === session_id);
  if (!session) return;
  session.mode = "embedded";
  session.window_id = null;
  desktop.selected_session_id = session_id;
  desktop.workspace_view = { kind: "session", session_id };
  openStaticApp("ai-launcher");
}

export function sendSessionToDashboard(session_id: string) {
  const session = desktop.sessions.find((item) => item.id === session_id);
  if (!session) return;
  if (session.window_id) {
    const index = desktop.windows.findIndex((item) => item.id === session.window_id);
    if (index >= 0) {
      desktop.windows.splice(index, 1);
    }
  }
  openSessionInDashboard(session_id);
}

export function openSessionInWindow(session_id: string) {
  const session = desktop.sessions.find((item) => item.id === session_id);
  if (!session) return;
  if (session.window_id) {
    focusWindow(session.window_id);
    return;
  }
  const browserConfig = apps_config.browser;
  const window = createWindow("browser", session.title, browserConfig.width!, browserConfig.height!);
  window.session_id = session.id;
  window.browser = {
    history: [session.url],
    index: 0,
    reload_key: 0,
  };
  assignWindowFocus(window);
  desktop.windows.push(window);
  session.mode = "windowed";
  session.window_id = window.id;
  desktop.selected_session_id = session.id;
}

export function openSessionInDrawer(session_id: string, side: "left" | "right") {
  const session = desktop.sessions.find((item) => item.id === session_id);
  if (!session) return;
  // Close any existing windowed view for this session
  if (session.window_id) {
    const index = desktop.windows.findIndex((item) => item.id === session.window_id);
    if (index >= 0) desktop.windows.splice(index, 1);
    session.window_id = null;
  }
  // Clear fullscreen if set
  if (desktop.fullscreen_session_id === session_id) {
    desktop.fullscreen_session_id = null;
  }
  session.mode = side === "left" ? "drawer-left" : "drawer-right";
  desktop.drawer = { session_id, side };
  desktop.selected_session_id = session_id;
}

export function closeDrawer() {
  if (desktop.drawer) {
    const session = desktop.sessions.find((item) => item.id === desktop.drawer!.session_id);
    if (session) session.mode = "embedded";
  }
  desktop.drawer = null;
}

export function openSessionFullscreen(session_id: string) {
  const session = desktop.sessions.find((item) => item.id === session_id);
  if (!session) return;
  // Close any existing windowed view for this session
  if (session.window_id) {
    const index = desktop.windows.findIndex((item) => item.id === session.window_id);
    if (index >= 0) desktop.windows.splice(index, 1);
    session.window_id = null;
  }
  // Clear drawer if set
  if (desktop.drawer?.session_id === session_id) {
    desktop.drawer = null;
  }
  session.mode = "fullscreen";
  desktop.fullscreen_session_id = session_id;
  desktop.selected_session_id = session_id;
}

export function closeFullscreenSession() {
  if (desktop.fullscreen_session_id) {
    const session = desktop.sessions.find((item) => item.id === desktop.fullscreen_session_id);
    if (session) session.mode = "embedded";
  }
  desktop.fullscreen_session_id = null;
}

export function openGenericBrowserWindow(initialUrl = "", title = apps_config.browser.title) {
  const browserConfig = apps_config.browser;
  const window = createWindow("browser", title, browserConfig.width!, browserConfig.height!);
  window.browser = {
    history: [initialUrl],
    index: 0,
    reload_key: 0,
  };
  assignWindowFocus(window);
  desktop.windows.push(window);
  return window;
}

export function updateBrowserWindowUrl(window_id: string, url: string) {
  const window = desktop.windows.find((item) => item.id === window_id);
  if (!window?.browser) return;
  window.browser.history = window.browser.history.slice(0, window.browser.index + 1);
  window.browser.history.push(url);
  window.browser.index = window.browser.history.length - 1;
  window.title = url.replace(/^https?:\/\//, "");
}

export function stepBrowserHistory(window_id: string, direction: -1 | 1) {
  const window = desktop.windows.find((item) => item.id === window_id);
  if (!window?.browser) return;
  const nextIndex = window.browser.index + direction;
  if (nextIndex < 0 || nextIndex >= window.browser.history.length) return;
  window.browser.index = nextIndex;
}

export function reloadBrowserWindow(window_id: string) {
  const window = desktop.windows.find((item) => item.id === window_id);
  if (!window?.browser) return;
  window.browser.reload_key += 1;
}

export function currentBrowserUrl(window: DesktopWindow) {
  if (!window.browser) return "";
  return window.browser.history[window.browser.index] ?? "";
}

export function getSessionById(session_id: string | null) {
  if (!session_id) return null;
  return desktop.sessions.find((item) => item.id === session_id) ?? null;
}

export function syncSessionsFromInstalled(installedApps: InstalledApp[]) {
  const runningApps = installedApps.filter(
    (item) => item.status.state === "Running" && typeof item.status.port === "number",
  );
  const runningIds = new Set(runningApps.map((item) => item.manifest.id));

  for (let index = desktop.sessions.length - 1; index >= 0; index -= 1) {
    const session = desktop.sessions[index];
    if (!runningIds.has(session.app_id)) {
      if (session.window_id) {
        const windowIndex = desktop.windows.findIndex((item) => item.id === session.window_id);
        if (windowIndex >= 0) {
          desktop.windows.splice(windowIndex, 1);
        }
      }
      desktop.sessions.splice(index, 1);
      if (desktop.workspace_view.kind === "session" && desktop.workspace_view.session_id === session.id) {
        desktop.workspace_view = { kind: "dashboard" };
      }
    }
  }

  for (const app of runningApps) {
    const session = desktop.sessions.find((item) => item.app_id === app.manifest.id);
    if (!session) {
      desktop.sessions.push({
        id: makeWindowId(app.manifest.id),
        app_id: app.manifest.id,
        title: app.manifest.name,
        url: `http://localhost:${app.status.port!}`,
        port: app.status.port!,
        mode: desktop.default_session_mode,
        window_id: null,
        load_state: "idle",
        last_focused_at: new Date().toISOString(),
        pinned: false,
      });
      continue;
    }
    session.title = app.manifest.name;
    session.port = app.status.port!;
    session.url = `http://localhost:${app.status.port!}`;
  }
}

export function markSessionLoadState(session_id: string, state: RunningSession["load_state"]) {
  const session = getSessionById(session_id);
  if (!session) return;
  session.load_state = state;
}

export function focusSessionDetails(session_id: string) {
  desktop.selected_session_id = session_id;
}

export function launcherRunningSessions() {
  return [...desktop.sessions].sort((left, right) =>
    right.last_focused_at.localeCompare(left.last_focused_at),
  );
}

export function visibleWindows() {
  return [...desktop.windows].filter((item) => !item.minimized).sort((left, right) => left.z_index - right.z_index);
}

export function activeWindow() {
  return topVisibleWindow();
}

export function windowForApp(app_id: WindowAppID | "browser" | "chat") {
  return desktop.windows.find((item) => item.app_id === app_id) ?? null;
}

export function sessionForApp(app_id: string) {
  return desktop.sessions.find((item) => item.app_id === app_id) ?? null;
}

export function isDockAppOpen(app_id: StaticAppID | string) {
  if (app_id === "launchpad") {
    return desktop.launchpad_open;
  }
  if (app_id === "browser") {
    return desktop.windows.some((item) => item.app_id === "browser");
  }
  return desktop.windows.some((item) => item.app_id === app_id) || desktop.sessions.some((item) => item.app_id === app_id);
}

export function saveIconPositions() {
  try {
    localStorage.setItem(ICON_POSITIONS_STORAGE_KEY, JSON.stringify(desktop.icon_positions));
  } catch {}
}

export function setIconPosition(id: string, x: number, y: number) {
  desktop.icon_positions[id] = { x, y };
  saveIconPositions();
}

export function resetIconPositions() {
  desktop.icon_positions = {};
  try {
    localStorage.removeItem(ICON_POSITIONS_STORAGE_KEY);
  } catch {}
}

export function selectIcon(id: string | null, additive = false) {
  if (id === null) {
    desktop.icon_selection = new Set();
    return;
  }
  if (additive) {
    const next = new Set(desktop.icon_selection);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    desktop.icon_selection = next;
  } else {
    desktop.icon_selection = new Set([id]);
  }
}

export function hideDesktopIcon(id: string) {
  const next = new Set(desktop.hidden_icons);
  next.add(id);
  desktop.hidden_icons = next;
  try {
    localStorage.setItem(HIDDEN_ICONS_STORAGE_KEY, JSON.stringify([...next]));
  } catch {}
}

export function showDesktopIcon(id: string) {
  const next = new Set(desktop.hidden_icons);
  next.delete(id);
  desktop.hidden_icons = next;
  try {
    localStorage.setItem(HIDDEN_ICONS_STORAGE_KEY, JSON.stringify([...next]));
  } catch {}
}

/** Open the Service Hub with deep-link context (required services + return-to app). */
export function openServiceHub(options: { require?: string[]; returnTo?: string; returnData?: any; autoInstall?: boolean } = {}) {
  const existing = desktop.windows.find((item) => item.app_id === "service-hub");
  if (existing) {
    // Update context on re-open
    existing.data = options;
    focusWindow(existing.id);
    return existing;
  }
  const config = apps_config["service-hub"];
  const window = createWindow("service-hub" as WindowAppID, config.title, config.width!, config.height!);
  window.resizable = config.resizable!;
  window.expandable = config.expandable!;
  window.data = options;
  assignWindowFocus(window);
  desktop.windows.push(window);
  saveOpenWindows();
  return window;
}

export type { StaticAppID };
