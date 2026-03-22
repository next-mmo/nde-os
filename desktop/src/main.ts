import { mount } from "svelte";
import Desktop from "🍎/components/Desktop/Desktop.svelte";
import "🍎/css/global.css";

mount(Desktop, {
  target: document.getElementById("root")!,
});
