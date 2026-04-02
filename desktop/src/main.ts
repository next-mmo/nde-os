import { mount } from "svelte";
import Desktop from "🍎/components/Desktop/Desktop.svelte";
import { initScreenshotHotkeys } from "🍎/lib/tauri/screenshot";
import { openStaticApp } from "🍎/state/desktop.svelte";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import "🍎/css/global.css";

initScreenshotHotkeys();

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
    /** Force the Kanban board to re-read tasks from disk. */
    refreshKanban: async () => {
      await invoke("get_agent_tasks").catch(() => {});
      await emit("tasks://updated", null).catch(() => {});
    },
  };
}
