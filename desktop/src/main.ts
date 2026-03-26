import { mount } from "svelte";
import Desktop from "🍎/components/Desktop/Desktop.svelte";
import { initScreenshotHotkeys } from "🍎/lib/tauri/screenshot";
import "🍎/css/global.css";

initScreenshotHotkeys();

mount(Desktop, {
  target: document.getElementById("root")!,
});
