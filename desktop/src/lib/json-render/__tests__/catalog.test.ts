/**
 * Unit tests for @json-render integration — catalog definition
 *
 * Tests that the NDE-OS catalog is correctly defined and usable
 * with @json-render/core utilities (prompt generation, spec validation, streaming).
 */

import { describe, it, expect } from "vitest";
import { validateSpec, buildUserPrompt } from "@json-render/core";
import { catalog, systemPrompt } from "../catalog";

// ── Catalog definition ───────────────────────────────────────────────

describe("catalog definition", () => {
  it("catalog is a valid defineCatalog result", () => {
    expect(catalog).toBeDefined();
    expect(typeof catalog.prompt).toBe("function");
  });

  it("has all component definitions visible in system prompt", () => {
    // The catalog's internal structure is opaque, but we can verify
    // all components appear in the generated system prompt
    const expected = [
      "Card", "Stack", "Grid", "Divider",
      "Heading", "Text", "Code", "Badge",
      "Metric", "Table", "List", "Progress", "StatusDot",
      "Button", "Input", "Toggle", "Select",
      "Alert", "Spinner", "Empty",
      "AppTile", "Terminal",
    ];
    for (const name of expected) {
      expect(systemPrompt).toContain(name);
    }
  });

  it("has all action definitions visible in system prompt", () => {
    // Verify actions appear in the generated prompt
    const expected = [
      "navigate", "open_app", "select_manifest",
      "install_app", "launch_app", "stop_app", "uninstall_app",
      "refresh", "copy_to_clipboard", "open_url",
      "discover_plugins", "install_plugin", "start_plugin", "stop_plugin",
    ];
    for (const name of expected) {
      expect(systemPrompt).toContain(name);
    }
  });
});

// ── Prompt generation ────────────────────────────────────────────────

describe("LLM prompt generation", () => {
  it("catalog.prompt() returns a non-empty string", () => {
    expect(typeof systemPrompt).toBe("string");
    expect(systemPrompt.length).toBeGreaterThan(100);
  });

  it("system prompt includes component names", () => {
    expect(systemPrompt).toContain("Card");
    expect(systemPrompt).toContain("Button");
    expect(systemPrompt).toContain("Metric");
    expect(systemPrompt).toContain("Terminal");
    expect(systemPrompt).toContain("AppTile");
  });

  it("system prompt includes action names", () => {
    expect(systemPrompt).toContain("install_app");
    expect(systemPrompt).toContain("navigate");
    expect(systemPrompt).toContain("refresh");
  });

  it("system prompt includes component descriptions", () => {
    expect(systemPrompt).toContain("card container");
    expect(systemPrompt).toContain("KPI metric");
    expect(systemPrompt).toContain("AI app tile");
  });
});

// ── Spec validation ──────────────────────────────────────────────────

describe("spec validation", () => {
  it("validates a valid spec", () => {
    const spec = {
      root: "card-1",
      elements: {
        "card-1": {
          type: "Card",
          props: { title: "Hello" },
          children: ["text-1"],
        },
        "text-1": {
          type: "Text",
          props: { text: "World" },
          children: [],
        },
      },
    };
    const result = validateSpec(spec, catalog);
    expect(result.valid).toBe(true);
    expect(result.issues).toHaveLength(0);
  });

  it("accepts unknown component types (lenient validation)", () => {
    // @json-render/core validateSpec is lenient — unknown types pass
    // because the renderer may handle them as custom components
    const spec = {
      root: "unknown-1",
      elements: {
        "unknown-1": {
          type: "DoesNotExist",
          props: {},
          children: [],
        },
      },
    };
    const result = validateSpec(spec, catalog);
    expect(result.valid).toBe(true);
  });

  it("rejects a spec with missing root reference", () => {
    const spec = {
      root: "nonexistent-id",
      elements: {
        "real-1": {
          type: "Text",
          props: { text: "Hello" },
          children: [],
        },
      },
    };
    const result = validateSpec(spec, catalog);
    // Root references a non-existent element
    expect(result.valid).toBe(false);
  });

  it("validates a complex nested spec", () => {
    const spec = {
      root: "stack-1",
      elements: {
        "stack-1": {
          type: "Stack",
          props: { direction: "vertical", gap: "lg" },
          children: ["heading-1", "card-1", "btn-1"],
        },
        "heading-1": {
          type: "Heading",
          props: { text: "Dashboard", level: "1" },
          children: [],
        },
        "card-1": {
          type: "Card",
          props: { title: "Stats", variant: "glass" },
          children: ["metric-1", "metric-2"],
        },
        "metric-1": {
          type: "Metric",
          props: { label: "CPU", value: "45%", trend: "up", change: "+5%" },
          children: [],
        },
        "metric-2": {
          type: "Metric",
          props: { label: "RAM", value: "8.2 GB", trend: "down", change: "-0.3 GB" },
          children: [],
        },
        "btn-1": {
          type: "Button",
          props: { label: "Refresh", action: "refresh", variant: "secondary" },
          children: [],
        },
      },
    };
    const result = validateSpec(spec, catalog);
    expect(result.valid).toBe(true);
  });

  it("validates a spec with NDE-OS specific components", () => {
    const spec = {
      root: "stack-1",
      elements: {
        "stack-1": {
          type: "Stack",
          props: { direction: "vertical" },
          children: ["app-1", "terminal-1"],
        },
        "app-1": {
          type: "AppTile",
          props: {
            name: "Stable Diffusion",
            description: "Generate images with AI",
            icon: "🎨",
            status: "installed",
            action: "launch_app",
          },
          children: [],
        },
        "terminal-1": {
          type: "Terminal",
          props: {
            title: "Install Log",
            lines: ["[1/4] Creating sandbox...", "[2/4] Setting up Python...", "[done] Installed"],
          },
          children: [],
        },
      },
    };
    const result = validateSpec(spec, catalog);
    expect(result.valid).toBe(true);
  });
});

// ── Streaming compiler ───────────────────────────────────────────────

describe("spec streaming compiler", () => {
  it("createSpecStreamCompiler exists and is callable", () => {
    const { createSpecStreamCompiler } = require("@json-render/core") as any;
    expect(typeof createSpecStreamCompiler).toBe("function");
    const compiler = createSpecStreamCompiler();
    expect(typeof compiler.push).toBe("function");
    expect(typeof compiler.getResult).toBe("function");
    expect(typeof compiler.reset).toBe("function");
  });

  it("compiler resets cleanly", () => {
    const { createSpecStreamCompiler } = require("@json-render/core") as any;
    const compiler = createSpecStreamCompiler();
    compiler.push('{"root": "a"}');
    compiler.reset();
    const result = compiler.getResult();
    expect(result).toBeDefined();
  });
});

// ── buildUserPrompt ──────────────────────────────────────────────────

describe("buildUserPrompt", () => {
  it("generates a user prompt from catalog", () => {
    const prompt = buildUserPrompt(catalog, "Show system health dashboard with CPU, RAM metrics");
    expect(typeof prompt).toBe("string");
    expect(prompt.length).toBeGreaterThan(0);
  });
});

// ── Edge cases ───────────────────────────────────────────────────────

describe("edge cases", () => {
  it("validates empty spec", () => {
    const spec = { root: "", elements: {} };
    const result = validateSpec(spec, catalog);
    // Empty root is invalid
    expect(result.valid).toBe(false);
  });

  it("validates spec with all interactive components", () => {
    const spec = {
      root: "stack-1",
      elements: {
        "stack-1": {
          type: "Stack",
          props: { direction: "vertical", gap: "md" },
          children: ["input-1", "toggle-1", "select-1", "btn-1"],
        },
        "input-1": {
          type: "Input",
          props: { label: "Name", placeholder: "Enter name", type: "text", statePath: "/form/name" },
          children: [],
        },
        "toggle-1": {
          type: "Toggle",
          props: { label: "Enable GPU", statePath: "/settings/gpu" },
          children: [],
        },
        "select-1": {
          type: "Select",
          props: { label: "Model", options: ["GPT-4", "Claude", "Llama"], statePath: "/settings/model" },
          children: [],
        },
        "btn-1": {
          type: "Button",
          props: { label: "Save", action: "refresh", variant: "primary" },
          children: [],
        },
      },
    };
    const result = validateSpec(spec, catalog);
    expect(result.valid).toBe(true);
  });

  it("validates spec with feedback components", () => {
    const spec = {
      root: "stack-1",
      elements: {
        "stack-1": {
          type: "Stack",
          props: { direction: "vertical" },
          children: ["alert-1", "progress-1", "spinner-1", "empty-1"],
        },
        "alert-1": {
          type: "Alert",
          props: { message: "Something went wrong", variant: "error", title: "Error" },
          children: [],
        },
        "progress-1": {
          type: "Progress",
          props: { value: 75, max: 100, label: "Installing..." },
          children: [],
        },
        "spinner-1": {
          type: "Spinner",
          props: { size: "lg", label: "Loading models..." },
          children: [],
        },
        "empty-1": {
          type: "Empty",
          props: { icon: "📦", title: "No apps installed", description: "Browse the catalog to get started" },
          children: [],
        },
      },
    };
    const result = validateSpec(spec, catalog);
    expect(result.valid).toBe(true);
  });

  it("validates spec with data display components", () => {
    const spec = {
      root: "stack-1",
      elements: {
        "stack-1": {
          type: "Stack",
          props: { direction: "vertical" },
          children: ["table-1", "list-1", "badge-1", "code-1", "divider-1", "dot-1"],
        },
        "table-1": {
          type: "Table",
          props: { headers: ["App", "Status", "Port"], rows: [["ComfyUI", "Running", "8188"], ["Ollama", "Stopped", "11434"]], striped: true },
          children: [],
        },
        "list-1": {
          type: "List",
          props: { items: [{ label: "ComfyUI", description: "Node-based SD UI", icon: "🧩" }] },
          children: [],
        },
        "badge-1": {
          type: "Badge",
          props: { text: "GPU", variant: "success" },
          children: [],
        },
        "code-1": {
          type: "Code",
          props: { code: "python3 main.py --listen 0.0.0.0", language: "bash" },
          children: [],
        },
        "divider-1": {
          type: "Divider",
          props: { label: "System Info" },
          children: [],
        },
        "dot-1": {
          type: "StatusDot",
          props: { status: "online", label: "Server" },
          children: [],
        },
      },
    };
    const result = validateSpec(spec, catalog);
    expect(result.valid).toBe(true);
  });
});
