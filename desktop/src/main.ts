import { mount } from "svelte";
import Desktop from "🍎/components/Desktop/Desktop.svelte";
import { initScreenshotHotkeys } from "🍎/lib/tauri/screenshot";
import { openStaticApp, openServiceHub, addNotification, desktop, setVikingInstalled, type StaticAppID } from "🍎/state/desktop.svelte";
import { fetchPendingDesktopActions } from "$lib/api/backend";
import { apps_config } from "🍎/configs/apps/apps-config";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import "🍎/css/global.css";

initScreenshotHotkeys();

// ── Remote Desktop Action Poller ──
// Polls the server every 3s for actions pushed by gateways (Telegram /app:vibe-studio).
const POLL_INTERVAL_MS = 3000;
const validAppIds = new Set(Object.keys(apps_config));

setInterval(async () => {
  try {
    const actions = await fetchPendingDesktopActions();
    for (const action of actions) {
      if (action.kind === "open_app" && validAppIds.has(action.app_id)) {
        openStaticApp(action.app_id as StaticAppID);
      }
    }
  } catch {
    // Silently ignore — server may be offline
  }
}, POLL_INTERVAL_MS);

// Suppress default browser context menu globally — the app uses its own Svelte context menus
document.addEventListener("contextmenu", (e) => e.preventDefault());

// NDE-OS is a virtual desktop — the root viewport must NEVER scroll.
// Browser focus-scroll and scrollIntoView() can bypass overflow:hidden.
// This guard immediately resets any root scroll to 0, preventing the top bar
// from being pushed off-screen.
document.addEventListener("scroll", () => {
  if (document.documentElement.scrollTop !== 0) {
    document.documentElement.scrollTop = 0;
  }
  if (document.body.scrollTop !== 0) {
    document.body.scrollTop = 0;
  }
}, { passive: true, capture: true });

mount(Desktop, {
  target: document.getElementById("root")!,
});

// Expose desktop API for E2E tests (Playwright evaluate calls).
// Only in dev mode — not bundled in production.
if (!import.meta.env.PROD) {
  (window as any).__svelteDesktop = {
    openStaticApp,
    openServiceHub,
    /** Force the Kanban board to re-read tasks from disk. */
    refreshKanban: async () => {
      await invoke("get_agent_tasks").catch(() => {});
      await emit("tasks://updated", null).catch(() => {});
    },
  };
}

if (typeof window !== "undefined") {
  // Wait a moment for UI to settle, then initialize agent context tools
  setTimeout(async () => {
    try {
      const { vikingStatus, vikingInstall, vikingStart } = await import("🍎/lib/api/backend");
      const status = await vikingStatus();
      
      if (!status.connected) {
        if (!desktop.viking_is_installed) {
          desktop.viking_onboard_state = { stage: "installing", message: "Auto-onboarding in background" };
          addNotification({
            app: "OpenViking",
            title: "Installing...",
            message: "Auto-onboarding in background",
            icon: "⚙️",
            action: "open-service-hub"
          });
          
          await vikingInstall();
          setVikingInstalled();
        }
        
        desktop.viking_onboard_state = { stage: "starting", message: "Launching server" };
        addNotification({
          app: "OpenViking",
          title: "Starting...",
          message: "Launching server",
          icon: "⚙️",
          action: "open-service-hub"
        });
        
        await vikingStart();
        
        desktop.viking_onboard_state = { stage: "ready", message: "Ready for agent context" };
        addNotification({
          app: "OpenViking",
          title: "Server Started",
          message: "Ready for agent context",
          icon: "🗄️",
          action: "open-service-hub"
        });
      } else {
        setVikingInstalled();
        desktop.viking_onboard_state = { stage: "ready", message: "Already running" };
        // Only notify if we wanted to
        addNotification({
          app: "OpenViking",
          title: "Server Connected",
          message: "Already running on port 1933",
          icon: "🗄️",
          action: "open-service-hub"
        });
      }
    } catch (e) {
      desktop.viking_onboard_state = { stage: "error", message: String(e) };
      addNotification({
        app: "OpenViking",
        title: "Failed",
        message: String(e),
        icon: "⚠️",
        action: "open-service-hub"
      });
    }
  }, 1500);
}
