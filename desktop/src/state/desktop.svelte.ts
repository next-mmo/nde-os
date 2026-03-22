import { apps_config, type StaticAppID } from "🍎/configs/apps/apps-config";
import type { InstalledApp } from "$lib/api/types";

export type WindowAppID = Exclude<StaticAppID, "launchpad">;
export type ThemeScheme = "light" | "dark";
export type LauncherSection = "overview" | "catalog" | "installed" | "running" | "server";
export type SessionMode = "embedded" | "windowed";

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

export type DesktopWindow = {
  id: string;
  app_id: WindowAppID | "browser";
  title: string;
  width: number;
  height: number;
  z_index: number;
  minimized: boolean;
  fullscreen: boolean;
  resizable: boolean;
  expandable: boolean;
  session_id: string | null;
  browser: BrowserState | null;
};

type WorkspaceView =
  | { kind: "dashboard" }
  | { kind: "session"; session_id: string };

const makeWindowId = (prefix: string) =>
  `${prefix}-${Math.random().toString(36).slice(2, 9)}-${Date.now().toString(36)}`;

const createWindow = (
  app_id: WindowAppID | "browser",
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
  fullscreen: false,
  resizable: true,
  expandable: true,
  session_id: null,
  browser: null,
});

export const desktop = $state({
  theme: "light" as ThemeScheme,
  launchpad_open: false,
  launcher_section: "overview" as LauncherSection,
  launcher_query: "",
  selected_app_id: null as string | null,
  selected_session_id: null as string | null,
  workspace_view: { kind: "dashboard" } as WorkspaceView,
  windows: [] as DesktopWindow[],
  sessions: [] as RunningSession[],
  next_z_index: 10,
});

function nextZIndex() {
  desktop.next_z_index += 2;
  return desktop.next_z_index;
}

function assignWindowFocus(window: DesktopWindow) {
  window.z_index = nextZIndex();
  window.minimized = false;
}

export function bootDesktop() {
  if (desktop.windows.length > 0) return;
  const launcherConfig = apps_config["ai-launcher"];
  const launcher = createWindow("ai-launcher", launcherConfig.title, launcherConfig.width!, launcherConfig.height!);
  launcher.resizable = launcherConfig.resizable!;
  launcher.expandable = launcherConfig.expandable!;
  assignWindowFocus(launcher);
  desktop.windows.push(launcher);
}

export function toggleTheme() {
  desktop.theme = desktop.theme === "light" ? "dark" : "light";
}

export function toggleLaunchpad(force?: boolean) {
  desktop.launchpad_open = force ?? !desktop.launchpad_open;
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
  if (!desktop.windows.some((item) => item.app_id === "ai-launcher")) {
    bootDesktop();
  }
}

export function minimizeWindow(window_id: string) {
  const window = desktop.windows.find((item) => item.id === window_id);
  if (!window) return;
  window.minimized = true;
}

export function toggleFullscreen(window_id: string) {
  const window = desktop.windows.find((item) => item.id === window_id);
  if (!window) return;
  window.fullscreen = !window.fullscreen;
  assignWindowFocus(window);
}

export function openStaticApp(app_id: StaticAppID) {
  if (app_id === "launchpad") {
    toggleLaunchpad();
    return;
  }
  if (app_id === "browser") {
    openGenericBrowserWindow();
    return;
  }

  const existing = desktop.windows.find((item) => item.app_id === app_id);
  if (existing) {
    focusWindow(existing.id);
    return existing;
  }

  const config = apps_config[app_id];
  const window = createWindow(app_id, config.title, config.width!, config.height!);
  window.resizable = config.resizable!;
  window.expandable = config.expandable!;
  assignWindowFocus(window);
  desktop.windows.push(window);
  return window;
}

export function selectLauncherSection(section: LauncherSection) {
  desktop.launcher_section = section;
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

export function openGenericBrowserWindow() {
  const browserConfig = apps_config.browser;
  const window = createWindow("browser", browserConfig.title, browserConfig.width!, browserConfig.height!);
  window.browser = {
    history: ["http://localhost:3000"],
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
        mode: "embedded",
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
  return [...desktop.windows].sort((left, right) => right.z_index - left.z_index)[0] ?? null;
}

export function windowForApp(app_id: WindowAppID | "browser") {
  return desktop.windows.find((item) => item.app_id === app_id) ?? null;
}

export function sessionForApp(app_id: string) {
  return desktop.sessions.find((item) => item.app_id === app_id) ?? null;
}

export function isDockAppOpen(app_id: StaticAppID) {
  if (app_id === "launchpad") {
    return desktop.launchpad_open;
  }
  if (app_id === "browser") {
    return desktop.windows.some((item) => item.app_id === "browser");
  }
  return desktop.windows.some((item) => item.app_id === app_id);
}

export type { StaticAppID };
