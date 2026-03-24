import type { AppManifest, InstalledApp } from "$lib/api/types";
import { installApp, launchApp, stopApp } from "$lib/stores/state";
import {
  focusSessionDetails,
  openGenericBrowserWindow,
  openSessionInDashboard,
  openSessionInWindow,
  selectLauncherSection,
  selectManifest,
  upsertSession,
  sendSessionToDashboard,
  type DefaultSessionMode,
} from "🍎/state/desktop.svelte";

export type OpenTarget = DefaultSessionMode;

function runningPort(installed: InstalledApp | null | undefined) {
  return installed?.status.state === "Running" ? installed.status.port ?? null : null;
}

export async function revealOrLaunchManifest(
  manifest: AppManifest,
  installed: InstalledApp | null,
  target: OpenTarget,
) {
  if (!installed) {
    selectLauncherSection("catalog");
    selectManifest(manifest.id);
    return null;
  }

  const activePort = runningPort(installed);
  if (typeof activePort === "number") {
    const session = upsertSession({
      app_id: manifest.id,
      title: manifest.name,
      port: activePort,
      mode: target,
    });
    focusSessionDetails(session.id);
    if (target === "embedded") {
      openSessionInDashboard(session.id);
    } else {
      openSessionInWindow(session.id);
    }
    return session;
  }

  const launched = await launchApp(manifest.id);
  const session = upsertSession({
    app_id: manifest.id,
    title: manifest.name,
    port: launched.port,
    mode: target,
  });
  focusSessionDetails(session.id);
  if (target === "embedded") {
    openSessionInDashboard(session.id);
  } else {
    openSessionInWindow(session.id);
  }
  return session;
}

export async function installAndFocusCatalog(manifest: AppManifest) {
  await installApp(manifest);
  selectLauncherSection("catalog");
  selectManifest(manifest.id);
}

export async function stopManifest(installed: InstalledApp) {
  await stopApp(installed.manifest.id);
}

export function openSessionInNewOsWindow(port: number, title?: string) {
  const url = `http://localhost:${port}`;
  openGenericBrowserWindow(url, title ?? url.replace(/^https?:\/\//, ""));
}

export function moveSessionToDashboard(session_id: string) {
  sendSessionToDashboard(session_id);
}
