/**
 * NDE-OS Component Catalog
 *
 * Uses the official @json-render/shadcn-svelte catalog definitions,
 * extended with NDE-OS-specific components and actions.
 */

import { defineCatalog } from "@json-render/core";
import { schema } from "@json-render/svelte/schema";
import { shadcnComponentDefinitions } from "@json-render/shadcn-svelte/catalog";
import { z } from "zod";

export const catalog = defineCatalog(schema, {
  components: {
    // ── All shadcn-svelte components from the official package ────────
    ...shadcnComponentDefinitions,

    // ── NDE-OS Specific ──────────────────────────────────────────────
    AppTile: {
      props: z.object({
        name: z.string(),
        description: z.string(),
        icon: z.string().optional(),
        status: z.enum(["available", "installed", "running", "error"]),
        action: z.string(),
      }),
      description:
        "AI app tile showing name, status, and install/launch action",
    },
    Terminal: {
      props: z.object({
        lines: z.array(z.string()),
        title: z.string().optional(),
        maxHeight: z.number().default(300),
      }),
      description:
        "Terminal-style output display with scrollable lines",
    },
    Code: {
      props: z.object({
        code: z.string(),
        language: z.string().default("text"),
      }),
      description: "Syntax-highlighted code block",
    },
    StatusDot: {
      props: z.object({
        status: z.enum(["online", "offline", "warning", "busy"]),
        label: z.string().optional(),
      }),
      description: "Colored status indicator dot with optional label",
    },
    Metric: {
      props: z.object({
        label: z.string(),
        value: z.string(),
        change: z.string().optional(),
        trend: z.enum(["up", "down", "neutral"]).optional(),
        format: z
          .enum(["number", "currency", "percent", "bytes"])
          .optional(),
      }),
      description:
        "KPI metric display with label, value, and optional trend indicator",
    },
    Empty: {
      props: z.object({
        icon: z.string().optional(),
        title: z.string(),
        description: z.string().optional(),
        action: z.string().optional(),
        actionLabel: z.string().optional(),
      }),
      description:
        "Empty state placeholder with icon, message, and optional CTA",
    },
    List: {
      props: z.object({
        items: z.array(
          z.object({
            label: z.string(),
            description: z.string().optional(),
            icon: z.string().optional(),
            action: z.string().optional(),
          })
        ),
      }),
      description:
        "Interactive list of items with optional icons and actions",
    },
  },

  actions: {
    // ── Navigation ────────────────────────────────────────────
    navigate: {
      description:
        "Navigate to a launcher section (catalog, installed, running, plugins, etc.)",
    },
    open_app: {
      description:
        "Open a static app window (settings, terminal, browser, etc.)",
    },
    select_manifest: {
      description:
        "Select an app manifest in the catalog to show details",
    },

    // ── App Lifecycle ─────────────────────────────────────────
    install_app: {
      description: "Install an app from catalog by ID",
    },
    launch_app: { description: "Launch an installed app" },
    stop_app: { description: "Stop a running app" },
    uninstall_app: { description: "Uninstall an app" },

    // ── System ────────────────────────────────────────────────
    refresh: {
      description:
        "Refresh workspace data (catalog, installed, running)",
    },
    copy_to_clipboard: {
      description: "Copy text to clipboard",
    },
    open_url: {
      description: "Open a URL in the browser",
    },

    // ── Plugin ────────────────────────────────────────────────
    discover_plugins: {
      description: "Scan for available plugins",
    },
    install_plugin: {
      description: "Install a discovered plugin",
    },
    start_plugin: {
      description: "Start an installed plugin",
    },
    stop_plugin: {
      description: "Stop a running plugin",
    },
  },
});

/** Generate the system prompt for the LLM to produce valid specs */
export const systemPrompt = catalog.prompt();
