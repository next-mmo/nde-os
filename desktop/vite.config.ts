import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [
    svelte(),
    tailwindcss(),
  ],
  resolve: {
    alias: {
      "🍎": new URL("./src/", import.meta.url).pathname,
      $lib: new URL("./src/lib/", import.meta.url).pathname,
    },
  },
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    outDir: "build",
    emptyOutDir: true,
    cssMinify: "lightningcss",
  },
  css: {
    transformer: "lightningcss",
  },
});
