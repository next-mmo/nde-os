<svelte:options runes={true} />

<script module lang="ts">
  // Declaring apps and config in a module block ensures they are only allocated ONCE in memory
  // for the entire application, rather than re-allocated for every single window instance.
  const apps: Record<string, () => Promise<any>> = {
    "ai-launcher": () => import("🍎/components/apps/Launcher/Launcher.svelte"),
    "browser": () => import("🍎/components/apps/Browser/Browser.svelte"),
    "logs": () => import("🍎/components/apps/Logs/Logs.svelte"),
    "chat": () => import("🍎/components/apps/Chat/Chat.svelte"),
    "settings": () => import("🍎/components/apps/Settings/Settings.svelte"),
    "code-editor": () => import("🍎/components/apps/CodeEditor/CodeEditor.svelte"),
    "command-center": () => import("🍎/components/apps/CommandCenter/CommandCenter.svelte"),
    "model-settings": () => import("🍎/components/apps/ModelSettings/ModelSettings.svelte"),
    "plugins": () => import("🍎/components/apps/Plugins/Plugins.svelte"),
    "channels": () => import("🍎/components/apps/Channels/Channels.svelte"),
    "mcp-tools": () => import("🍎/components/apps/McpTools/McpTools.svelte"),
    "skills": () => import("🍎/components/apps/Skills/Skills.svelte"),
    "knowledge": () => import("🍎/components/apps/Knowledge/Knowledge.svelte"),
    "shield-browser": () => import("🍎/components/apps/ShieldBrowser/ShieldBrowser.svelte"),
    "file-explorer": () => import("🍎/components/apps/FileExplorer/FileExplorer.svelte"),
    "architecture": () => import("🍎/components/apps/Architecture/Architecture.svelte"),
    "vibe-studio": () => import("🍎/components/apps/VibeCodeStudio/VibeCodeStudio.svelte"),
    "screenshot": () => import("🍎/components/apps/Screenshot/Screenshot.svelte"),
    "terminal": () => import("🍎/components/apps/Terminal/Terminal.svelte"),
    "freecut": () => import("🍎/components/apps/FreeCut/FreeCut.svelte"),
    "service-hub": () => import("🍎/components/apps/ServiceHub/ServiceHub.svelte")
  };

  const needsWindowProp = new Set(["browser", "vibe-studio", "screenshot", "terminal", "service-hub"]);

  const getAppPromise = (app_id: string) => {
    if (app_id in apps) {
      return apps[app_id]();
    }
    return import("🍎/components/apps/Placeholder/Placeholder.svelte");
  };
</script>

<script lang="ts">
  import type { DesktopWindow } from "🍎/state/desktop.svelte";

  interface Props {
    window: DesktopWindow;
  }

  let { window }: Props = $props();

  let appPromise = $derived(getAppPromise(window.app_id));
</script>

{#await appPromise then { default: App }}
  {#if needsWindowProp.has(window.app_id)}
    <App {window} />
  {:else if !(window.app_id in apps)}
    <App app_id={window.app_id} />
  {:else}
    <App />
  {/if}
{/await}
