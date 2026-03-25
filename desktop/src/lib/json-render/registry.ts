/**
 * NDE-OS Component Registry
 *
 * Maps catalog component names to Svelte 5 render functions.
 * These are the actual implementations that @json-render/svelte's
 * <Renderer> uses to turn JSON specs into live UI.
 */

import { defineRegistry } from "@json-render/svelte";
import { catalog } from "./catalog";

// ── Gap / size maps ──────────────────────────────────────────────────

const gapMap: Record<string, string> = {
  none: "0", xs: "0.25rem", sm: "0.5rem", md: "0.75rem", lg: "1.25rem", xl: "2rem",
};

const padMap: Record<string, string> = {
  none: "0", sm: "0.5rem", md: "1rem", lg: "1.5rem",
};

const fontSizeMap: Record<string, string> = {
  xs: "0.72rem", sm: "0.82rem", md: "0.92rem", lg: "1.1rem",
};

const headingSize: Record<string, string> = {
  "1": "1.5rem", "2": "1.2rem", "3": "1rem", "4": "0.88rem",
};

const btnSizeMap: Record<string, string> = {
  sm: "0.35rem 0.7rem", md: "0.5rem 1rem", lg: "0.65rem 1.3rem",
};

const statusColorMap: Record<string, string> = {
  online: "hsl(145 65% 50%)", offline: "hsl(0 0% 55%)",
  warning: "hsl(40 90% 55%)", busy: "hsl(0 70% 55%)",
};

const alertColors: Record<string, { bg: string; border: string; text: string }> = {
  info:    { bg: "hsla(210 80% 55%/0.08)", border: "hsla(210 80% 55%/0.2)", text: "hsl(210 80% 65%)" },
  success: { bg: "hsla(145 65% 50%/0.08)", border: "hsla(145 65% 50%/0.2)", text: "hsl(145 65% 55%)" },
  warning: { bg: "hsla(40 90% 55%/0.08)",  border: "hsla(40 90% 55%/0.2)",  text: "hsl(40 90% 60%)" },
  error:   { bg: "hsla(0 70% 55%/0.08)",   border: "hsla(0 70% 55%/0.2)",   text: "hsl(0 70% 60%)" },
};

const badgeColors: Record<string, { bg: string; text: string }> = {
  default: { bg: "hsla(0 0% 50%/0.12)", text: "var(--system-color-text-muted)" },
  success: { bg: "hsla(145 65% 50%/0.12)", text: "hsl(145 65% 55%)" },
  warning: { bg: "hsla(40 90% 55%/0.12)", text: "hsl(40 90% 60%)" },
  danger:  { bg: "hsla(0 70% 55%/0.12)", text: "hsl(0 70% 60%)" },
  info:    { bg: "hsla(210 80% 55%/0.12)", text: "hsl(210 80% 65%)" },
};

const trendColor: Record<string, string> = {
  up: "hsl(145 65% 55%)", down: "hsl(0 70% 60%)", neutral: "var(--system-color-text-muted)",
};

const btnVariants: Record<string, string> = {
  primary: "background:hsl(215 90% 55%);color:#fff;border:none",
  secondary: "background:var(--system-color-panel);color:var(--system-color-text);border:1px solid var(--system-color-border)",
  outline: "background:transparent;color:var(--system-color-text);border:1px solid var(--system-color-border)",
  ghost: "background:transparent;color:var(--system-color-text);border:none",
  danger: "background:hsl(0 70% 55%);color:#fff;border:none",
};

const appStatusColors: Record<string, string> = {
  available: "var(--system-color-text-muted)",
  installed: "hsl(40 90% 55%)",
  running: "hsl(145 65% 55%)",
  error: "hsl(0 70% 55%)",
};

// ── Registry ─────────────────────────────────────────────────────────

export const { registry } = defineRegistry(catalog, {
  components: {
    // ── Layout ───────────────────────────────────────────────
    Card: ({ props, children }) => {
      const p = props as any;
      const base = "border-radius:1rem;";
      const padding = `padding:${padMap[p.padding ?? "md"] ?? "1rem"};`;
      const variantStyle = p.variant === "glass"
        ? "background:hsla(0 0% 100%/0.04);backdrop-filter:blur(12px);border:1px solid hsla(0 0% 100%/0.08)"
        : p.variant === "outlined"
          ? "background:transparent;border:1px solid var(--system-color-border)"
          : "background:var(--system-color-panel);border:1px solid var(--system-color-border)";
      return {
        tag: "div",
        style: `${base}${padding}${variantStyle}`,
        children: p.title
          ? [{ tag: "strong", style: "display:block;margin-bottom:0.5rem;font-size:0.88rem", text: p.title }, children]
          : [children],
      };
    },

    Stack: ({ props, children }) => {
      const p = props as any;
      const dir = p.direction === "horizontal" ? "row" : "column";
      const g = gapMap[p.gap ?? "md"] ?? "0.75rem";
      const alignMap: Record<string, string> = { start: "flex-start", center: "center", end: "flex-end", stretch: "stretch" };
      const justMap: Record<string, string> = { start: "flex-start", center: "center", end: "flex-end", between: "space-between", around: "space-around" };
      return {
        tag: "div",
        style: `display:flex;flex-direction:${dir};gap:${g};align-items:${alignMap[p.align ?? "start"]};justify-content:${justMap[p.justify ?? "start"]}${p.wrap ? ";flex-wrap:wrap" : ""}`,
        children: [children],
      };
    },

    Grid: ({ props, children }) => {
      const p = props as any;
      const g = gapMap[p.gap ?? "md"] ?? "0.75rem";
      return {
        tag: "div",
        style: `display:grid;grid-template-columns:repeat(${p.columns ?? 2},1fr);gap:${g}`,
        children: [children],
      };
    },

    Divider: ({ props }) => {
      const p = props as any;
      if (p.label) {
        return {
          tag: "div",
          style: "display:flex;align-items:center;gap:0.75rem;margin:0.25rem 0",
          children: [
            { tag: "div", style: "flex:1;height:1px;background:var(--system-color-border)" },
            { tag: "span", style: "font-size:0.72rem;color:var(--system-color-text-muted);text-transform:uppercase;letter-spacing:0.1em", text: p.label },
            { tag: "div", style: "flex:1;height:1px;background:var(--system-color-border)" },
          ],
        };
      }
      return { tag: "hr", style: "border:none;height:1px;background:var(--system-color-border);margin:0.25rem 0" };
    },

    // ── Typography ────────────────────────────────────────────
    Heading: ({ props }) => {
      const p = props as any;
      const level = p.level ?? "2";
      const tag = `h${level}` as "h1" | "h2" | "h3" | "h4";
      return {
        tag,
        style: `margin:0;font-size:${headingSize[level]};font-weight:600${p.muted ? ";color:var(--system-color-text-muted)" : ""}`,
        text: p.text,
      };
    },

    Text: ({ props }) => {
      const p = props as any;
      const styles = [
        `font-size:${fontSizeMap[p.size ?? "md"]}`,
        p.muted ? "color:var(--system-color-text-muted)" : "",
        p.bold ? "font-weight:600" : "",
        p.mono ? "font-family:monospace" : "",
        "margin:0",
      ].filter(Boolean).join(";");
      return { tag: "p", style: styles, text: p.text };
    },

    Code: ({ props }) => {
      const p = props as any;
      return {
        tag: "pre",
        style: "margin:0;padding:0.75rem 1rem;border-radius:0.5rem;background:hsla(0 0% 0%/0.3);font-family:monospace;font-size:0.78rem;overflow-x:auto;white-space:pre-wrap;color:hsl(145 65% 70%)",
        children: [{ tag: "code", text: p.code }],
      };
    },

    Badge: ({ props }) => {
      const p = props as any;
      const c = badgeColors[p.variant ?? "default"] ?? badgeColors.default;
      return {
        tag: "span",
        style: `display:inline-block;padding:0.15rem 0.5rem;border-radius:999px;font-size:0.7rem;font-weight:500;background:${c.bg};color:${c.text}`,
        text: p.text,
      };
    },

    // ── Data Display ──────────────────────────────────────────
    Metric: ({ props }) => {
      const p = props as any;
      const tc = trendColor[p.trend ?? "neutral"];
      return {
        tag: "div",
        style: "display:flex;flex-direction:column;gap:0.15rem",
        children: [
          { tag: "span", style: "font-size:0.72rem;color:var(--system-color-text-muted);text-transform:uppercase;letter-spacing:0.08em", text: p.label },
          {
            tag: "div", style: "display:flex;align-items:baseline;gap:0.4rem",
            children: [
              { tag: "strong", style: "font-size:1.4rem", text: p.value },
              ...(p.change ? [{ tag: "span", style: `font-size:0.75rem;color:${tc}`, text: p.change }] : []),
            ],
          },
        ],
      };
    },

    Table: ({ props }) => {
      const p = props as any;
      const headerCells = (p.headers ?? []).map((h: string) => ({
        tag: "th" as const, style: "padding:0.5rem 0.75rem;text-align:left;font-size:0.72rem;text-transform:uppercase;letter-spacing:0.08em;color:var(--system-color-text-muted);border-bottom:1px solid var(--system-color-border)", text: h,
      }));
      const rows = (p.rows ?? []).map((row: string[], i: number) => ({
        tag: "tr" as const,
        style: p.striped && i % 2 ? "background:hsla(0 0% 50%/0.04)" : "",
        children: row.map((cell: string) => ({
          tag: "td" as const, style: "padding:0.5rem 0.75rem;font-size:0.82rem;border-bottom:1px solid var(--system-color-border)", text: cell,
        })),
      }));
      return {
        tag: "table",
        style: "width:100%;border-collapse:collapse",
        children: [
          { tag: "thead", children: [{ tag: "tr", children: headerCells }] },
          { tag: "tbody", children: rows },
        ],
      };
    },

    List: ({ props }) => {
      const p = props as any;
      return {
        tag: "div",
        style: "display:flex;flex-direction:column",
        children: (p.items ?? []).map((item: any) => ({
          tag: "div",
          style: "display:flex;align-items:center;gap:0.6rem;padding:0.55rem 0.5rem;border-bottom:1px solid var(--system-color-border);cursor:default",
          children: [
            ...(item.icon ? [{ tag: "span", style: "font-size:1.1rem", text: item.icon }] : []),
            {
              tag: "div", style: "flex:1;min-width:0",
              children: [
                { tag: "strong", style: "font-size:0.85rem;display:block", text: item.label },
                ...(item.description ? [{ tag: "span", style: "font-size:0.72rem;color:var(--system-color-text-muted)", text: item.description }] : []),
              ],
            },
          ],
        })),
      };
    },

    Progress: ({ props }) => {
      const p = props as any;
      const pct = Math.min(100, Math.max(0, (p.value / (p.max ?? 100)) * 100));
      const barColor = p.variant === "success" ? "hsl(145 65% 50%)" : p.variant === "warning" ? "hsl(40 90% 55%)" : p.variant === "danger" ? "hsl(0 70% 55%)" : "hsl(215 90% 55%)";
      return {
        tag: "div",
        style: "display:flex;flex-direction:column;gap:0.3rem",
        children: [
          ...(p.label ? [{
            tag: "div" as const, style: "display:flex;justify-content:space-between;font-size:0.75rem",
            children: [
              { tag: "span", text: p.label },
              { tag: "span", style: "color:var(--system-color-text-muted)", text: `${Math.round(pct)}%` },
            ],
          }] : []),
          {
            tag: "div", style: "height:6px;border-radius:3px;background:hsla(0 0% 50%/0.15);overflow:hidden",
            children: [{ tag: "div", style: `width:${pct}%;height:100%;border-radius:3px;background:${barColor};transition:width 0.3s ease` }],
          },
        ],
      };
    },

    StatusDot: ({ props }) => {
      const p = props as any;
      const c = statusColorMap[p.status] ?? "gray";
      return {
        tag: "div",
        style: "display:inline-flex;align-items:center;gap:0.35rem",
        children: [
          { tag: "span", style: `width:8px;height:8px;border-radius:50%;background:${c}` },
          ...(p.label ? [{ tag: "span", style: "font-size:0.78rem", text: p.label }] : []),
        ],
      };
    },

    // ── Interactive ───────────────────────────────────────────
    Button: ({ props, emit }) => {
      const p = props as any;
      const vs = btnVariants[p.variant ?? "primary"];
      return {
        tag: "button",
        style: `${vs};padding:${btnSizeMap[p.size ?? "md"]};border-radius:0.5rem;font-size:0.82rem;font-weight:500;cursor:pointer;transition:opacity 0.15s${p.disabled ? ";opacity:0.4;pointer-events:none" : ""}`,
        text: p.label,
        onclick: () => emit("press"),
      };
    },

    Input: ({ props, emit }) => {
      const p = props as any;
      return {
        tag: "div",
        style: "display:flex;flex-direction:column;gap:0.25rem",
        children: [
          ...(p.label ? [{ tag: "label", style: "font-size:0.75rem;color:var(--system-color-text-muted)", text: p.label }] : []),
          {
            tag: "input",
            style: "padding:0.5rem 0.75rem;border-radius:0.5rem;border:1px solid var(--system-color-border);background:var(--system-color-panel);color:var(--system-color-text);font-size:0.85rem;outline:none",
            attrs: { type: p.type ?? "text", placeholder: p.placeholder ?? "" },
            oninput: (e: Event) => emit("change", { value: (e.target as HTMLInputElement).value }),
          },
        ],
      };
    },

    Toggle: ({ props, emit }) => {
      const p = props as any;
      return {
        tag: "label",
        style: "display:flex;align-items:center;gap:0.5rem;cursor:pointer;font-size:0.85rem",
        children: [
          { tag: "input", attrs: { type: "checkbox" }, style: "accent-color:hsl(215 90% 55%)", onchange: () => emit("toggle") },
          { tag: "span", text: p.label },
        ],
      };
    },

    Select: ({ props, emit }) => {
      const p = props as any;
      return {
        tag: "div",
        style: "display:flex;flex-direction:column;gap:0.25rem",
        children: [
          ...(p.label ? [{ tag: "label", style: "font-size:0.75rem;color:var(--system-color-text-muted)", text: p.label }] : []),
          {
            tag: "select",
            style: "padding:0.5rem 0.75rem;border-radius:0.5rem;border:1px solid var(--system-color-border);background:var(--system-color-panel);color:var(--system-color-text);font-size:0.85rem",
            children: (p.options ?? []).map((opt: string) => ({ tag: "option", attrs: { value: opt }, text: opt })),
            onchange: (e: Event) => emit("change", { value: (e.target as HTMLSelectElement).value }),
          },
        ],
      };
    },

    // ── Feedback ──────────────────────────────────────────────
    Alert: ({ props }) => {
      const p = props as any;
      const c = alertColors[p.variant ?? "info"];
      return {
        tag: "div",
        style: `padding:0.75rem 1rem;border-radius:0.75rem;background:${c.bg};border:1px solid ${c.border};color:${c.text};font-size:0.82rem`,
        children: [
          ...(p.title ? [{ tag: "strong", style: "display:block;margin-bottom:0.2rem;font-size:0.85rem", text: p.title }] : []),
          { tag: "span", text: p.message },
        ],
      };
    },

    Spinner: ({ props }) => {
      const p = props as any;
      const sizeMap: Record<string, string> = { sm: "16px", md: "24px", lg: "36px" };
      const s = sizeMap[p.size ?? "md"];
      return {
        tag: "div",
        style: "display:flex;align-items:center;gap:0.5rem",
        children: [
          { tag: "div", style: `width:${s};height:${s};border:2px solid var(--system-color-border);border-top-color:hsl(215 90% 55%);border-radius:50%;animation:spin 0.8s linear infinite` },
          ...(p.label ? [{ tag: "span", style: "font-size:0.82rem;color:var(--system-color-text-muted)", text: p.label }] : []),
        ],
      };
    },

    Empty: ({ props, emit }) => {
      const p = props as any;
      return {
        tag: "div",
        style: "display:flex;flex-direction:column;align-items:center;gap:0.5rem;padding:2rem;text-align:center",
        children: [
          ...(p.icon ? [{ tag: "span", style: "font-size:2.5rem", text: p.icon }] : []),
          { tag: "strong", style: "font-size:0.92rem", text: p.title },
          ...(p.description ? [{ tag: "p", style: "margin:0;font-size:0.82rem;color:var(--system-color-text-muted);max-width:280px", text: p.description }] : []),
          ...(p.actionLabel ? [{
            tag: "button",
            style: "margin-top:0.5rem;padding:0.45rem 1rem;border-radius:999px;background:hsl(215 90% 55%);color:#fff;border:none;font-size:0.8rem;cursor:pointer",
            text: p.actionLabel,
            onclick: () => emit("press"),
          }] : []),
        ],
      };
    },

    // ── NDE-OS Specific ───────────────────────────────────────
    AppTile: ({ props, emit }) => {
      const p = props as any;
      const sc = appStatusColors[p.status] ?? "gray";
      return {
        tag: "div",
        style: "display:flex;align-items:center;gap:0.75rem;padding:0.75rem;border-radius:0.85rem;border:1px solid var(--system-color-border);background:var(--system-color-panel);cursor:pointer;transition:border-color 0.15s",
        onclick: () => emit("press"),
        children: [
          { tag: "span", style: "font-size:1.5rem", text: p.icon ?? "📦" },
          {
            tag: "div", style: "flex:1;min-width:0",
            children: [
              { tag: "strong", style: "font-size:0.85rem;display:block", text: p.name },
              { tag: "span", style: "font-size:0.72rem;color:var(--system-color-text-muted)", text: p.description },
            ],
          },
          { tag: "span", style: `font-size:0.7rem;font-weight:500;color:${sc};text-transform:uppercase`, text: p.status },
        ],
      };
    },

    Terminal: ({ props }) => {
      const p = props as any;
      return {
        tag: "div",
        style: `border-radius:0.75rem;background:hsl(0 0% 8%);border:1px solid hsl(0 0% 15%);overflow:hidden;max-height:${p.maxHeight ?? 300}px`,
        children: [
          ...(p.title ? [{
            tag: "div", style: "padding:0.4rem 0.75rem;font-size:0.72rem;color:hsl(0 0% 50%);border-bottom:1px solid hsl(0 0% 15%);font-family:monospace",
            text: p.title,
          }] : []),
          {
            tag: "div", style: "padding:0.5rem 0.75rem;font-family:monospace;font-size:0.75rem;color:hsl(145 65% 70%);overflow-y:auto;white-space:pre-wrap",
            children: (p.lines ?? []).map((line: string) => ({ tag: "div", text: line })),
          },
        ],
      };
    },
  },
});
