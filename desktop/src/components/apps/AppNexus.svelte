<svelte:options runes={true} />

<script lang="ts">
  import type { DesktopWindow } from "🍎/state/desktop.svelte";

  interface Props {
    window: DesktopWindow;
  }

  let { window }: Props = $props();
</script>

{#if window.app_id === "ai-launcher"}
  {#await import("🍎/components/apps/Launcher/Launcher.svelte") then { default: Launcher }}
    <Launcher />
  {/await}
{:else if window.app_id === "browser"}
  {#await import("🍎/components/apps/Browser/Browser.svelte") then { default: Browser }}
    <Browser {window} />
  {/await}
{:else if window.app_id === "logs"}
  {#await import("🍎/components/apps/Logs/Logs.svelte") then { default: Logs }}
    <Logs />
  {/await}
{:else if window.app_id === "chat"}
  {#await import("🍎/components/apps/Chat/Chat.svelte") then { default: Chat }}
    <Chat />
  {/await}
{:else if window.app_id === "settings"}
  {#await import("🍎/components/apps/Settings/Settings.svelte") then { default: Settings }}
    <Settings />
  {/await}
{:else if window.app_id === "code-editor"}
  {#await import("🍎/components/apps/CodeEditor/CodeEditor.svelte") then { default: CodeEditor }}
    <CodeEditor />
  {/await}
{:else if window.app_id === "command-center"}
  {#await import("🍎/components/apps/CommandCenter/CommandCenter.svelte") then { default: CommandCenter }}
    <CommandCenter />
  {/await}
{:else if window.app_id === "model-settings"}
  {#await import("🍎/components/apps/ModelSettings/ModelSettings.svelte") then { default: ModelSettings }}
    <ModelSettings />
  {/await}
{:else if window.app_id === "plugins"}
  {#await import("🍎/components/apps/Plugins/Plugins.svelte") then { default: Plugins }}
    <Plugins />
  {/await}
{:else if window.app_id === "channels"}
  {#await import("🍎/components/apps/Channels/Channels.svelte") then { default: Channels }}
    <Channels />
  {/await}
{:else if window.app_id === "mcp-tools"}
  {#await import("🍎/components/apps/McpTools/McpTools.svelte") then { default: McpTools }}
    <McpTools />
  {/await}
{:else if window.app_id === "skills"}
  {#await import("🍎/components/apps/Skills/Skills.svelte") then { default: Skills }}
    <Skills />
  {/await}
{:else if window.app_id === "knowledge"}
  {#await import("🍎/components/apps/Knowledge/Knowledge.svelte") then { default: Knowledge }}
    <Knowledge />
  {/await}
{:else}
  {#await import("🍎/components/apps/Placeholder/Placeholder.svelte") then { default: Placeholder }}
    <Placeholder app_id={window.app_id} />
  {/await}
{/if}

