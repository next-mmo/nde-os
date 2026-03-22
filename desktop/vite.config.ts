import { svelte } from "@sveltejs/vite-plugin-svelte";
import UnpluginIcons from "unplugin-icons/vite";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [
    svelte(),
    UnpluginIcons({
      compiler: "svelte",
    }),
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
