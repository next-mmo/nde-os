import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { desktop, focusWindow, openStaticApp } from "🍎/state/desktop.svelte";

let overlayWindow: WebviewWindow | null = null;
let unlistenSelected: UnlistenFn | null = null;
let unlistenCancelled: UnlistenFn | null = null;

export function initScreenshotHotkeys() {
  window.addEventListener("keydown", (e) => {
    if (e.ctrlKey && e.shiftKey && e.key.toLowerCase() === "s") {
      e.preventDefault();
      triggerScreenshotMode().catch(console.error);
    }
  });
}

export async function triggerScreenshotMode() {
  if (overlayWindow) return;

  // Capture the screen BEFORE showing the overlay so it never appears in the image
  let fullCapture: string | null = null;
  try {
    const response = await invoke<{base64_image: string | null, text: string | null}>("capture_screenshot", { ocr: false });
    fullCapture = response.base64_image;
  } catch (e) {
    console.error("Pre-capture failed", e);
    return;
  }
  if (!fullCapture) return;

  overlayWindow = new WebviewWindow("screenshot-overlay", {
    url: "/screenshot.html",
    transparent: true,
    decorations: false,
    alwaysOnTop: true,
    fullscreen: true,
    skipTaskbar: true,
  });

  unlistenSelected = await listen<{x: number, y: number, width: number, height: number}>("screenshot-selected", async (event) => {
    try { await closeOverlay(); } catch (error) {}

    try {
      let finalImage = fullCapture!;

      if (event.payload.width > 0 && event.payload.height > 0) {
        // Crop the pre-captured image on the frontend using canvas
        finalImage = await cropBase64Image(fullCapture!, event.payload);
      }

      showScreenshotResult(finalImage);
    } catch (error) {
      console.error("Failed to process screenshot", error);
      showScreenshotFallback();
    }
  });

  unlistenCancelled = await listen("screenshot-cancelled", async () => {
    try { await closeOverlay(); } catch (error) {}
  });
}

/** Crop a base64 data-URL image to the given logical-pixel area. */
function cropBase64Image(
  dataUrl: string,
  area: { x: number; y: number; width: number; height: number },
): Promise<string> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => {
      // Overlay coordinates are in logical (CSS) pixels.
      // The captured image is at physical resolution.
      // devicePixelRatio maps logical -> physical.
      const dpr = window.devicePixelRatio || 1;
      const sx = Math.round(area.x * dpr);
      const sy = Math.round(area.y * dpr);
      const sw = Math.round(area.width * dpr);
      const sh = Math.round(area.height * dpr);

      const canvas = document.createElement("canvas");
      canvas.width = sw;
      canvas.height = sh;
      const ctx = canvas.getContext("2d");
      if (!ctx) { reject(new Error("No canvas context")); return; }
      ctx.drawImage(img, sx, sy, sw, sh, 0, 0, sw, sh);
      resolve(canvas.toDataURL("image/png"));
    };
    img.onerror = () => reject(new Error("Failed to load image for crop"));
    img.src = dataUrl;
  });
}

function showScreenshotResult(imageData: string) {
  let windowId = "";
  for (const [, win] of desktop.windows.entries()) {
    if (win.app_id === "screenshot") {
      windowId = win.id;
      break;
    }
  }

  if (windowId) {
    const win = desktop.windows.find(w => w.id === windowId);
    if (win) {
      if (!win.data) win.data = {};
      win.data.image = imageData;
    }
    focusWindow(windowId);
  } else {
    const win = openStaticApp("screenshot" as any);
    if (win) {
      const w = desktop.windows.find(x => x.id === win.id);
      if (w) {
        if (!w.data) w.data = {};
        w.data.image = imageData;
      }
    }
  }
}

function showScreenshotFallback() {
  const fallbackWin = openStaticApp("screenshot" as any);
  if (fallbackWin) {
    const fw = desktop.windows.find(w => w.id === fallbackWin.id);
    if (fw) {
      if (!fw.data) fw.data = {};
      fw.data.image = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";
    }
  }
}

async function closeOverlay() {
  if (unlistenSelected) unlistenSelected();
  if (unlistenCancelled) unlistenCancelled();
  unlistenSelected = null;
  unlistenCancelled = null;

  if (overlayWindow) {
    try { await overlayWindow.close(); } catch(e) {}
    overlayWindow = null;
  }
}
