/**
 * Figma-JSON → CSS style resolver
 *
 * Converts our FNode properties into inline CSS style objects
 * that the Svelte renderer applies to DOM elements.
 */

import type {
  FBaseNode,
  FBlur,
  FColor,
  FFill,
  FFrameNode,
  FNode,
  FShadow,
  FStroke,
  FTextNode,
} from "./types";

// ─── Color helpers ───────────────────────────────────────────────────

export function fColorToCSS(c: FColor): string {
  const r = Math.round(c.r * 255);
  const g = Math.round(c.g * 255);
  const b = Math.round(c.b * 255);
  return `rgba(${r}, ${g}, ${b}, ${c.a})`;
}

// ─── Fill → CSS background ──────────────────────────────────────────

function fillToCSS(fill: FFill): string | null {
  if (fill.type === "SOLID") {
    const opacity = fill.opacity ?? fill.color.a;
    const c = { ...fill.color, a: opacity };
    return fColorToCSS(c);
  }
  if (fill.type === "LINEAR_GRADIENT") {
    const angle = fill.angle ?? 180;
    const stops = fill.stops
      .map((s) => `${fColorToCSS(s.color)} ${(s.position * 100).toFixed(1)}%`)
      .join(", ");
    return `linear-gradient(${angle}deg, ${stops})`;
  }
  if (fill.type === "RADIAL_GRADIENT") {
    const stops = fill.stops
      .map((s) => `${fColorToCSS(s.color)} ${(s.position * 100).toFixed(1)}%`)
      .join(", ");
    return `radial-gradient(circle, ${stops})`;
  }
  if (fill.type === "IMAGE") {
    const mode =
      fill.scaleMode === "FIT"
        ? "contain"
        : fill.scaleMode === "CROP"
          ? "cover"
          : fill.scaleMode === "TILE"
            ? "repeat"
            : "cover";
    return `url(${fill.src})`;
  }
  return null;
}

export function resolveFills(fills?: FFill[]): Record<string, string> {
  if (!fills || fills.length === 0) return {};

  // Single solid fill → background-color (most common, most performant)
  if (fills.length === 1 && fills[0].type === "SOLID") {
    return { "background-color": fillToCSS(fills[0])! };
  }

  // Multiple fills or gradients → background (layered)
  const backgrounds = fills
    .map(fillToCSS)
    .filter(Boolean)
    .reverse(); // Figma: last fill is on top
  if (backgrounds.length === 0) return {};

  // Check if any is an image
  const hasImage = fills.some((f) => f.type === "IMAGE");
  if (hasImage) {
    const imgFill = fills.find((f) => f.type === "IMAGE") as Extract<FFill, { type: "IMAGE" }>;
    const mode =
      imgFill.scaleMode === "FIT"
        ? "contain"
        : imgFill.scaleMode === "CROP"
          ? "cover"
          : "cover";
    return {
      background: backgrounds.join(", "),
      "background-size": mode === "cover" || mode === "contain" ? mode : "auto",
      "background-position": "center",
      "background-repeat": imgFill.scaleMode === "TILE" ? "repeat" : "no-repeat",
    };
  }

  return { background: backgrounds.join(", ") };
}

// ─── Strokes → CSS borders ──────────────────────────────────────────

export function resolveStrokes(strokes?: FStroke[]): Record<string, string> {
  if (!strokes || strokes.length === 0) return {};
  const s = strokes[0]; // CSS only supports one border natively
  const color = fColorToCSS(s.color);
  const style = s.dashPattern && s.dashPattern.length > 0 ? "dashed" : "solid";

  if (s.align === "INSIDE") {
    return {
      "box-shadow": `inset 0 0 0 ${s.weight}px ${color}`,
    };
  }
  if (s.align === "OUTSIDE") {
    return {
      "box-shadow": `0 0 0 ${s.weight}px ${color}`,
    };
  }
  return {
    border: `${s.weight}px ${style} ${color}`,
  };
}

// ─── Effects → CSS shadows + filters ────────────────────────────────

export function resolveEffects(
  effects?: (FShadow | FBlur)[],
): Record<string, string> {
  if (!effects || effects.length === 0) return {};

  const shadows: string[] = [];
  const filters: string[] = [];
  const backdropFilters: string[] = [];

  for (const e of effects) {
    if ("offset" in e) {
      // It's a shadow
      const shadow = e as FShadow;
      const color = fColorToCSS(shadow.color);
      const spread = shadow.spread ?? 0;
      const inset = shadow.type === "INNER_SHADOW" ? "inset " : "";
      shadows.push(
        `${inset}${shadow.offset.x}px ${shadow.offset.y}px ${shadow.radius}px ${spread}px ${color}`,
      );
    } else {
      // It's a blur
      const blur = e as FBlur;
      if (blur.type === "LAYER") {
        filters.push(`blur(${blur.radius}px)`);
      } else {
        backdropFilters.push(`blur(${blur.radius}px)`);
      }
    }
  }

  const result: Record<string, string> = {};
  if (shadows.length) result["box-shadow"] = shadows.join(", ");
  if (filters.length) result.filter = filters.join(" ");
  if (backdropFilters.length)
    result["backdrop-filter"] = backdropFilters.join(" ");

  return result;
}

// ─── Border radius ──────────────────────────────────────────────────

export function resolveBorderRadius(
  br?: number | [number, number, number, number],
): Record<string, string> {
  if (br === undefined) return {};
  if (typeof br === "number") {
    return br > 0 ? { "border-radius": `${br}px` } : {};
  }
  return {
    "border-radius": br.map((v) => `${v}px`).join(" "),
  };
}

// ─── Auto-layout → CSS flexbox ──────────────────────────────────────

function mapAlignToCSS(
  align?: string,
): string {
  switch (align) {
    case "MIN":
      return "flex-start";
    case "MAX":
      return "flex-end";
    case "CENTER":
      return "center";
    case "STRETCH":
      return "stretch";
    case "BASELINE":
      return "baseline";
    default:
      return "flex-start";
  }
}

export function resolveLayout(node: FFrameNode): Record<string, string> {
  const style: Record<string, string> = {};

  if (
    node.layoutMode &&
    node.layoutMode !== "NONE"
  ) {
    style.display = "flex";
    style["flex-direction"] =
      node.layoutMode === "HORIZONTAL" ? "row" : "column";

    if (node.layoutWrap === "WRAP") {
      style["flex-wrap"] = "wrap";
    }

    style["justify-content"] = mapAlignToCSS(node.primaryAxisAlignItems);
    style["align-items"] = mapAlignToCSS(node.counterAxisAlignItems);

    if (node.itemSpacing !== undefined && node.itemSpacing > 0) {
      style.gap = `${node.itemSpacing}px`;
    }
    if (node.counterAxisSpacing !== undefined && node.counterAxisSpacing > 0) {
      style["row-gap"] = `${node.counterAxisSpacing}px`;
    }
  }

  // Padding
  const pl = node.paddingLeft ?? 0;
  const pr = node.paddingRight ?? 0;
  const pt = node.paddingTop ?? 0;
  const pb = node.paddingBottom ?? 0;
  if (pl || pr || pt || pb) {
    style.padding = `${pt}px ${pr}px ${pb}px ${pl}px`;
  }

  // Sizing
  if (node.primaryAxisSizingMode === "HUG" || node.counterAxisSizingMode === "HUG") {
    // HUG = intrinsic sizing, don't set width/height
  } else {
    if (node.width !== undefined) style.width = `${node.width}px`;
    if (node.height !== undefined) style.height = `${node.height}px`;
  }

  if (node.layoutGrow && node.layoutGrow > 0) {
    style["flex-grow"] = `${node.layoutGrow}`;
  }

  return style;
}

// ─── Text styles ────────────────────────────────────────────────────

export function resolveTextStyles(node: FTextNode): Record<string, string> {
  const style: Record<string, string> = {};

  if (node.fontFamily) style["font-family"] = `'${node.fontFamily}', sans-serif`;
  if (node.fontWeight) style["font-weight"] = `${node.fontWeight}`;
  if (node.fontSize) style["font-size"] = `${node.fontSize}px`;

  if (node.lineHeight !== undefined) {
    style["line-height"] =
      node.lineHeight === "AUTO" ? "normal" : `${node.lineHeight}px`;
  }
  if (node.letterSpacing !== undefined && node.letterSpacing !== 0) {
    style["letter-spacing"] = `${node.letterSpacing}px`;
  }
  if (node.textDecoration && node.textDecoration !== "NONE") {
    style["text-decoration"] = node.textDecoration === "UNDERLINE" ? "underline" : "line-through";
  }
  if (node.textCase && node.textCase !== "ORIGINAL") {
    const map: Record<string, string> = {
      UPPER: "uppercase",
      LOWER: "lowercase",
      TITLE: "capitalize",
    };
    style["text-transform"] = map[node.textCase] ?? "none";
  }
  if (node.textAlignHorizontal) {
    const align: Record<string, string> = {
      LEFT: "left",
      CENTER: "center",
      RIGHT: "right",
      JUSTIFIED: "justify",
    };
    style["text-align"] = align[node.textAlignHorizontal] ?? "left";
  }
  if (node.width) {
    style["max-width"] = `${node.width}px`;
  }

  return style;
}

// ─── Master resolver ────────────────────────────────────────────────

export function resolveNodeStyles(node: FNode): Record<string, string> {
  let style: Record<string, string> = {};

  // Visibility
  if (node.visible === false) {
    return { display: "none" };
  }

  // Opacity
  if (node.opacity !== undefined && node.opacity < 1) {
    style.opacity = `${node.opacity}`;
  }

  // Rotation
  if (node.rotation) {
    style.transform = `rotate(${node.rotation}deg)`;
  }

  // Blend mode
  if (node.blendMode && node.blendMode !== "NORMAL" && node.blendMode !== "PASS_THROUGH") {
    style["mix-blend-mode"] = node.blendMode.toLowerCase().replace(/_/g, "-");
  }

  // Common visual
  style = { ...style, ...resolveFills(node.fills) };
  style = { ...style, ...resolveStrokes(node.strokes) };
  style = { ...style, ...resolveEffects(node.effects) };
  style = { ...style, ...resolveBorderRadius(node.borderRadius) };

  // Clip
  if (node.clipsContent) {
    style.overflow = "hidden";
  }

  // Type-specific
  switch (node.type) {
    case "FRAME":
    case "GROUP":
    case "COMPONENT":
    case "INSTANCE": {
      const frameStyles = resolveLayout(node as FFrameNode);
      style = { ...style, ...frameStyles };
      // For frames without auto-layout, set position relative for children
      if (!node.layoutMode || node.layoutMode === "NONE") {
        style.position = "relative";
        if ((node as FFrameNode).width) style.width = `${(node as FFrameNode).width}px`;
        if ((node as FFrameNode).height) style.height = `${(node as FFrameNode).height}px`;
      }
      break;
    }
    case "TEXT": {
      const textStyles = resolveTextStyles(node as FTextNode);
      style = { ...style, ...textStyles };
      break;
    }
    case "RECTANGLE":
    case "ELLIPSE":
    case "VECTOR":
    case "LINE":
    case "STAR":
    case "POLYGON": {
      if ("width" in node) style.width = `${node.width}px`;
      if ("height" in node) style.height = `${node.height}px`;
      if (node.type === "ELLIPSE") {
        style["border-radius"] = "50%";
      }
      break;
    }
    case "IMAGE": {
      // Image styling handled by the renderer img tag
      if ("width" in node) style.width = `${node.width}px`;
      if ("height" in node) style.height = `${node.height}px`;
      break;
    }
  }

  return style;
}

/**
 * Convert a style record to a CSS string for use in `style:` attribute
 */
export function stylesToString(styles: Record<string, string>): string {
  return Object.entries(styles)
    .map(([k, v]) => `${k}: ${v}`)
    .join("; ");
}
