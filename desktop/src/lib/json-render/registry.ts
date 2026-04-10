import { defineRegistry } from "@json-render/svelte";
import { shadcnComponents } from "@json-render/shadcn-svelte";
import { catalog } from "./catalog";

// Custom NDE-OS Components
import AppTile from "./components/AppTile.svelte";
import Terminal from "./components/Terminal.svelte";
import Code from "./components/Code.svelte";
import StatusDot from "./components/StatusDot.svelte";
import Metric from "./components/Metric.svelte";
import Empty from "./components/Empty.svelte";
import List from "./components/List.svelte";

const { registry, handlers, executeAction } = defineRegistry(catalog, {
  components: {
    // Official shadcn-svelte components
    ...shadcnComponents,

    // Custom NDE-OS components
    AppTile,
    Terminal,
    Code,
    StatusDot,
    Metric,
    Empty,
    List,
  },
  actions: {
    navigate: async (params, setState) => {
      console.log("[json-render] Navigation action triggered:", params);
    },
    open_app: async (params) => {
      console.log("[json-render] Open app action:", params);
    },
    select_manifest: async (params) => {
      console.log("[json-render] Select manifest:", params);
    },
    install_app: async (params) => {
      console.log("[json-render] Install app:", params);
    },
    launch_app: async (params) => {
      console.log("[json-render] Launch app:", params);
    },
    stop_app: async (params) => {
      console.log("[json-render] Stop app:", params);
    },
    uninstall_app: async (params) => {
      console.log("[json-render] Uninstall app:", params);
    },
    refresh: async () => {
      console.log("[json-render] Refresh data");
    },
    copy_to_clipboard: async (params) => {
      if (params?.text && typeof params.text === "string") {
        navigator.clipboard.writeText(params.text);
      }
    },
    open_url: async (params) => {
      if (params?.url && typeof params.url === "string") {
        window.open(params.url, "_blank");
      }
    },
    discover_plugins: async () => {
      console.log("[json-render] Discover plugins");
    },
    install_plugin: async (params) => {
      console.log("[json-render] Install plugin:", params);
    },
    start_plugin: async (params) => {
      console.log("[json-render] Start plugin:", params);
    },
    stop_plugin: async (params) => {
      console.log("[json-render] Stop plugin:", params);
    },
  },
});

export { registry, handlers, executeAction };
