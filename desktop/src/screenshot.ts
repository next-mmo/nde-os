import { mount } from "svelte";
import ScreenshotOverlay from "🍎/ScreenshotOverlay.svelte";
import "🍎/css/global.css";

mount(ScreenshotOverlay, {
  target: document.getElementById("root")!,
});
