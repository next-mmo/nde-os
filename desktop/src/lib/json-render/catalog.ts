/**
 * NDE-OS Component Catalog
 *
 * Defines the set of UI components that the AI agent can generate.
 * Uses @json-render/core defineCatalog for schema validation + prompt generation.
 * Uses @json-render/svelte schema for Svelte 5 rendering.
 */

import { defineCatalog } from "@json-render/core";
import { schema } from "@json-render/svelte/schema";
import { z } from "zod";

export const catalog = defineCatalog(schema, {
  components: {
    // ── Layout ────────────────────────────────────────────────
    Card: {
      props: z.object({
        title: z.string().optional(),
        variant: z.enum(["default", "outlined", "glass"]).default("default"),
        padding: z.enum(["none", "sm", "md", "lg"]).default("md"),
      }),
      description: "A styled card container with optional title, supports glass/outlined variants",
    },
    Stack: {
      props: z.object({
        direction: z.enum(["horizontal", "vertical"]).default("vertical"),
        gap: z.enum(["none", "xs", "sm", "md", "lg", "xl"]).default("md"),
        align: z.enum(["start", "center", "end", "stretch"]).default("start"),
        justify: z.enum(["start", "center", "end", "between", "around"]).default("start"),
        wrap: z.boolean().default(false),
      }),
      description: "Flexbox layout container for arranging children",
    },
    Grid: {
      props: z.object({
        columns: z.number().default(2),
        gap: z.enum(["none", "xs", "sm", "md", "lg"]).default("md"),
      }),
      description: "CSS grid layout with configurable columns",
    },
    Divider: {
      props: z.object({
        label: z.string().optional(),
      }),
      description: "Horizontal separator, optionally with a centered label",
    },

    // ── Typography ────────────────────────────────────────────
    Heading: {
      props: z.object({
        text: z.string(),
        level: z.enum(["1", "2", "3", "4"]).default("2"),
        muted: z.boolean().default(false),
      }),
      description: "Heading text (h1-h4) for section titles",
    },
    Text: {
      props: z.object({
        text: z.string(),
        size: z.enum(["xs", "sm", "md", "lg"]).default("md"),
        muted: z.boolean().default(false),
        bold: z.boolean().default(false),
        mono: z.boolean().default(false),
      }),
      description: "Body text with size, muted, bold, and monospace options",
    },
    Code: {
      props: z.object({
        code: z.string(),
        language: z.string().default("text"),
      }),
      description: "Syntax-highlighted code block",
    },
    Badge: {
      props: z.object({
        text: z.string(),
        variant: z.enum(["default", "success", "warning", "danger", "info"]).default("default"),
      }),
      description: "Small labeled badge/tag for status or categories",
    },

    // ── Data Display ──────────────────────────────────────────
    Metric: {
      props: z.object({
        label: z.string(),
        value: z.string(),
        change: z.string().optional(),
        trend: z.enum(["up", "down", "neutral"]).optional(),
        format: z.enum(["number", "currency", "percent", "bytes"]).optional(),
      }),
      description: "KPI metric display with label, value, and optional trend indicator",
    },
    Table: {
      props: z.object({
        headers: z.array(z.string()),
        rows: z.array(z.array(z.string())),
        striped: z.boolean().default(false),
      }),
      description: "Data table with headers and rows",
    },
    List: {
      props: z.object({
        items: z.array(z.object({
          label: z.string(),
          description: z.string().optional(),
          icon: z.string().optional(),
          action: z.string().optional(),
        })),
      }),
      description: "Interactive list of items with optional icons and actions",
    },
    Progress: {
      props: z.object({
        value: z.number(),
        max: z.number().default(100),
        label: z.string().optional(),
        variant: z.enum(["default", "success", "warning", "danger"]).default("default"),
      }),
      description: "Progress bar with label and percentage",
    },
    StatusDot: {
      props: z.object({
        status: z.enum(["online", "offline", "warning", "busy"]),
        label: z.string().optional(),
      }),
      description: "Colored status indicator dot with optional label",
    },

    // ── Interactive ───────────────────────────────────────────
    Button: {
      props: z.object({
        label: z.string(),
        variant: z.enum(["primary", "secondary", "outline", "ghost", "danger"]).default("primary"),
        size: z.enum(["sm", "md", "lg"]).default("md"),
        icon: z.string().optional(),
        disabled: z.boolean().default(false),
        action: z.string(),
      }),
      description: "Clickable button that triggers an action",
    },
    Input: {
      props: z.object({
        label: z.string().optional(),
        placeholder: z.string().optional(),
        type: z.enum(["text", "number", "password", "email", "url"]).default("text"),
        statePath: z.string(),
      }),
      description: "Text input field bound to a state path",
    },
    Toggle: {
      props: z.object({
        label: z.string(),
        statePath: z.string(),
      }),
      description: "On/off toggle switch bound to a state path",
    },
    Select: {
      props: z.object({
        label: z.string().optional(),
        options: z.array(z.string()),
        statePath: z.string(),
      }),
      description: "Dropdown select bound to a state path",
    },

    // ── Feedback ──────────────────────────────────────────────
    Alert: {
      props: z.object({
        message: z.string(),
        variant: z.enum(["info", "success", "warning", "error"]).default("info"),
        title: z.string().optional(),
        dismissible: z.boolean().default(false),
      }),
      description: "Alert banner for notifications, warnings, and errors",
    },
    Spinner: {
      props: z.object({
        size: z.enum(["sm", "md", "lg"]).default("md"),
        label: z.string().optional(),
      }),
      description: "Loading spinner with optional label",
    },
    Empty: {
      props: z.object({
        icon: z.string().optional(),
        title: z.string(),
        description: z.string().optional(),
        action: z.string().optional(),
        actionLabel: z.string().optional(),
      }),
      description: "Empty state placeholder with icon, message, and optional CTA",
    },

    // ── NDE-OS Specific ───────────────────────────────────────
    AppTile: {
      props: z.object({
        name: z.string(),
        description: z.string(),
        icon: z.string().optional(),
        status: z.enum(["available", "installed", "running", "error"]),
        action: z.string(),
      }),
      description: "AI app tile showing name, status, and install/launch action",
    },
    Terminal: {
      props: z.object({
        lines: z.array(z.string()),
        title: z.string().optional(),
        maxHeight: z.number().default(300),
      }),
      description: "Terminal-style output display with scrollable lines",
    },
  },

  actions: {
    // ── Navigation ────────────────────────────────────────────
    navigate: { description: "Navigate to a launcher section (catalog, installed, running, plugins, etc.)" },
    open_app: { description: "Open a static app window (settings, terminal, browser, etc.)" },
    select_manifest: { description: "Select an app manifest in the catalog to show details" },

    // ── App Lifecycle ─────────────────────────────────────────
    install_app: { description: "Install an app from catalog by ID" },
    launch_app: { description: "Launch an installed app" },
    stop_app: { description: "Stop a running app" },
    uninstall_app: { description: "Uninstall an app" },

    // ── System ────────────────────────────────────────────────
    refresh: { description: "Refresh workspace data (catalog, installed, running)" },
    copy_to_clipboard: { description: "Copy text to clipboard" },
    open_url: { description: "Open a URL in the browser" },

    // ── Plugin ────────────────────────────────────────────────
    discover_plugins: { description: "Scan for available plugins" },
    install_plugin: { description: "Install a discovered plugin" },
    start_plugin: { description: "Start an installed plugin" },
    stop_plugin: { description: "Stop a running plugin" },
  },
});

/** Generate the system prompt for the LLM to produce valid specs */
export const systemPrompt = catalog.prompt();
