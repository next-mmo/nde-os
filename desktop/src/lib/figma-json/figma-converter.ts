/**
 * Figma REST API → FDocument converter
 *
 * Takes raw Figma file/node JSON from the REST API and converts it
 * to our FDocument schema that the renderer understands.
 *
 * Usage:
 *   const doc = convertFigmaFile(figmaApiResponse, imageMap);
 *
 * The imageMap is obtained from the Figma Images API:
 *   GET /v1/files/:key/images → { meta: { images: { "hash": "url" } } }
 *   GET /v1/images/:key?ids=...&format=png → { images: { "nodeId": "url" } }
 */

import type {
  FColor,
  FDocument,
  FFill,
  FGradientStop,
  FNode,
  FStroke,
  FShadow,
  FBlur,
  FFrameNode,
  FTextNode,
  FRectangleNode,
  FEllipseNode,
  FVectorNode,
  FImageNode,
  FLayoutMode,
  FLayoutAlign,
  FLayoutSizing,
  FTextAlignHorizontal,
  FTextAlignVertical,
} from "./types";

// ─── Helpers ─────────────────────────────────────────────────────────

function color(c: any): FColor {
  if (!c) return { r: 0, g: 0, b: 0, a: 1 };
  return {
    r: c.r ?? 0,
    g: c.g ?? 0,
    b: c.b ?? 0,
    a: c.a ?? 1,
  };
}

function gradientStops(stops: any[]): FGradientStop[] {
  if (!stops) return [];
  return stops.map((s) => ({
    position: s.position ?? 0,
    color: color(s.color),
  }));
}

function gradientAngle(handles: any[]): number {
  if (!handles || handles.length < 2) return 180;
  const dx = handles[1].x - handles[0].x;
  const dy = handles[1].y - handles[0].y;
  return Math.round((Math.atan2(dy, dx) * 180) / Math.PI + 90);
}

// ─── Fill converter ──────────────────────────────────────────────────

function convertFills(
  fills: any[],
  imageMap?: Record<string, string>,
): FFill[] {
  if (!fills) return [];
  return fills
    .filter((f) => f.visible !== false)
    .map((f): FFill | null => {
      switch (f.type) {
        case "SOLID":
          return {
            type: "SOLID",
            color: color(f.color),
            opacity: f.opacity ?? 1,
          };
        case "GRADIENT_LINEAR":
          return {
            type: "LINEAR_GRADIENT",
            stops: gradientStops(f.gradientStops),
            angle: gradientAngle(f.gradientHandlePositions),
          };
        case "GRADIENT_RADIAL":
        case "GRADIENT_ANGULAR":
        case "GRADIENT_DIAMOND":
          return {
            type: "RADIAL_GRADIENT",
            stops: gradientStops(f.gradientStops),
          };
        case "IMAGE":
          return {
            type: "IMAGE",
            src: imageMap?.[f.imageRef] ?? `figma://image/${f.imageRef}`,
            scaleMode: f.scaleMode ?? "FILL",
          };
        default:
          return null;
      }
    })
    .filter(Boolean) as FFill[];
}

// ─── Stroke converter ───────────────────────────────────────────────

function convertStrokes(strokes: any[], strokeWeight: any, strokeAlign: any): FStroke[] {
  if (!strokes || strokes.length === 0) return [];
  return strokes
    .filter((s) => s.visible !== false)
    .map((s): FStroke => ({
      color: color(s.color),
      weight: typeof strokeWeight === "number" ? strokeWeight : 1,
      align: (strokeAlign as FStroke["align"]) ?? "CENTER",
      dashPattern: s.dashPattern,
    }));
}

// ─── Effect converter ───────────────────────────────────────────────

function convertEffects(effects: any[]): (FShadow | FBlur)[] {
  if (!effects) return [];
  return effects
    .filter((e) => e.visible !== false)
    .map((e): FShadow | FBlur | null => {
      if (e.type === "DROP_SHADOW" || e.type === "INNER_SHADOW") {
        return {
          type: e.type,
          color: color(e.color),
          offset: { x: e.offset?.x ?? 0, y: e.offset?.y ?? 0 },
          radius: e.radius ?? 0,
          spread: e.spread ?? 0,
        };
      }
      if (e.type === "LAYER_BLUR") {
        return { type: "LAYER", radius: e.radius ?? 0 };
      }
      if (e.type === "BACKGROUND_BLUR") {
        return { type: "BACKGROUND", radius: e.radius ?? 0 };
      }
      return null;
    })
    .filter(Boolean) as (FShadow | FBlur)[];
}

// ─── Border radius ──────────────────────────────────────────────────

function convertBorderRadius(
  node: any,
): number | [number, number, number, number] | undefined {
  if (node.rectangleCornerRadii) {
    const r = node.rectangleCornerRadii as number[];
    if (r.every((v) => v === r[0])) return r[0] > 0 ? r[0] : undefined;
    return [r[0], r[1], r[2], r[3]];
  }
  if (typeof node.cornerRadius === "number" && node.cornerRadius > 0) {
    return node.cornerRadius;
  }
  return undefined;
}

// ─── Layout mapping ─────────────────────────────────────────────────

function mapLayoutMode(mode: string | undefined): FLayoutMode {
  if (mode === "HORIZONTAL") return "HORIZONTAL";
  if (mode === "VERTICAL") return "VERTICAL";
  return "NONE";
}

function mapSizingMode(mode: string | undefined): FLayoutSizing {
  if (mode === "HUG") return "HUG";
  if (mode === "FILL" || mode === "STRETCH") return "FILL";
  return "FIXED";
}

function mapLayoutAlign(align: string | undefined): FLayoutAlign {
  switch (align) {
    case "MIN": return "MIN";
    case "MAX": return "MAX";
    case "CENTER": return "CENTER";
    case "STRETCH": return "STRETCH";
    case "SPACE_BETWEEN": return "MAX"; // approximate
    default: return "MIN";
  }
}

// ─── Node converter ─────────────────────────────────────────────────

let nodeIdCounter = 0;

function ensureId(node: any): string {
  return node.id ?? `gen-${++nodeIdCounter}`;
}

function convertNode(
  node: any,
  imageMap?: Record<string, string>,
): FNode | null {
  const base = {
    id: ensureId(node),
    name: node.name ?? "Untitled",
    visible: node.visible !== false,
    opacity: node.opacity ?? 1,
    rotation: node.rotation ?? undefined,
    borderRadius: convertBorderRadius(node),
    clipsContent: node.clipsContent ?? false,
    blendMode: node.blendMode ?? undefined,
    fills: convertFills(node.fills, imageMap),
    strokes: convertStrokes(node.strokes, node.strokeWeight, node.strokeAlign),
    effects: convertEffects(node.effects),
  };

  const type = node.type;

  // Frame-like nodes (FRAME, GROUP, COMPONENT, COMPONENT_SET, INSTANCE, SECTION)
  if (
    [
      "FRAME",
      "GROUP",
      "COMPONENT",
      "COMPONENT_SET",
      "INSTANCE",
      "SECTION",
    ].includes(type)
  ) {
    const frame: FFrameNode = {
      ...base,
      type: type === "COMPONENT_SET" || type === "SECTION" ? "FRAME" : type,
      width: node.absoluteBoundingBox?.width ?? node.size?.x,
      height: node.absoluteBoundingBox?.height ?? node.size?.y,
      layoutMode: mapLayoutMode(node.layoutMode),
      layoutWrap: node.layoutWrap ?? "NO_WRAP",
      primaryAxisAlignItems: mapLayoutAlign(node.primaryAxisAlignItems),
      counterAxisAlignItems: mapLayoutAlign(node.counterAxisAlignItems),
      primaryAxisSizingMode: mapSizingMode(node.primaryAxisSizingMode),
      counterAxisSizingMode: mapSizingMode(node.counterAxisSizingMode),
      layoutGrow: node.layoutGrow,
      itemSpacing: node.itemSpacing,
      counterAxisSpacing: node.counterAxisSpacing,
      paddingLeft: node.paddingLeft ?? node.horizontalPadding,
      paddingRight: node.paddingRight ?? node.horizontalPadding,
      paddingTop: node.paddingTop ?? node.verticalPadding,
      paddingBottom: node.paddingBottom ?? node.verticalPadding,
      children: convertChildren(node.children, imageMap),
    } as FFrameNode;
    return frame;
  }

  // Text
  if (type === "TEXT") {
    const text: FTextNode = {
      ...base,
      type: "TEXT",
      characters: node.characters ?? "",
      fontFamily: node.style?.fontFamily,
      fontWeight: node.style?.fontWeight,
      fontSize: node.style?.fontSize,
      lineHeight:
        node.style?.lineHeightUnit === "AUTO"
          ? "AUTO"
          : node.style?.lineHeightPx,
      letterSpacing: node.style?.letterSpacing,
      textDecoration: node.style?.textDecoration ?? "NONE",
      textCase: node.style?.textCase ?? "ORIGINAL",
      textAlignHorizontal: (node.style?.textAlignHorizontal as FTextAlignHorizontal) ?? "LEFT",
      textAlignVertical: (node.style?.textAlignVertical as FTextAlignVertical) ?? "TOP",
      width: node.absoluteBoundingBox?.width ?? node.size?.x,
    };
    return text;
  }

  // Rectangle
  if (type === "RECTANGLE") {
    const rect: FRectangleNode = {
      ...base,
      type: "RECTANGLE",
      width: node.absoluteBoundingBox?.width ?? node.size?.x ?? 0,
      height: node.absoluteBoundingBox?.height ?? node.size?.y ?? 0,
    };

    // Check if it has image fills — treat as IMAGE node instead
    const imgFill = node.fills?.find((f: any) => f.type === "IMAGE" && f.visible !== false);
    if (imgFill) {
      const image: FImageNode = {
        ...base,
        type: "IMAGE",
        width: rect.width,
        height: rect.height,
        src: imageMap?.[imgFill.imageRef] ?? `figma://image/${imgFill.imageRef}`,
        objectFit: imgFill.scaleMode === "FIT" ? "contain" : "cover",
      };
      return image;
    }

    return rect;
  }

  // Ellipse
  if (type === "ELLIPSE") {
    return {
      ...base,
      type: "ELLIPSE",
      width: node.absoluteBoundingBox?.width ?? node.size?.x ?? 0,
      height: node.absoluteBoundingBox?.height ?? node.size?.y ?? 0,
    } as FEllipseNode;
  }

  // Vector types
  if (["VECTOR", "LINE", "STAR", "REGULAR_POLYGON", "BOOLEAN_OPERATION"].includes(type)) {
    return {
      ...base,
      type: type === "REGULAR_POLYGON" ? "POLYGON" : type === "BOOLEAN_OPERATION" ? "VECTOR" : type,
      width: node.absoluteBoundingBox?.width ?? node.size?.x ?? 0,
      height: node.absoluteBoundingBox?.height ?? node.size?.y ?? 0,
      svgPath: node.fillGeometry?.[0]?.path,
    } as FVectorNode;
  }

  // Fallback: treat unknown types as invisible frames
  if (node.children) {
    return {
      ...base,
      type: "FRAME",
      width: node.absoluteBoundingBox?.width ?? node.size?.x,
      height: node.absoluteBoundingBox?.height ?? node.size?.y,
      children: convertChildren(node.children, imageMap),
    } as FFrameNode;
  }

  return null;
}

function convertChildren(
  children: any[] | undefined,
  imageMap?: Record<string, string>,
): FNode[] {
  if (!children) return [];
  return children.map((c) => convertNode(c, imageMap)).filter(Boolean) as FNode[];
}

// ─── Public API ──────────────────────────────────────────────────────

/**
 * Convert a Figma REST API file response into an FDocument.
 *
 * @param figmaResponse - The JSON response from GET /v1/files/:key
 * @param imageMap - Map of image refs to URLs from the Images API
 * @param pageIndex - Which page to convert (default: 0)
 */
export function convertFigmaFile(
  figmaResponse: any,
  imageMap?: Record<string, string>,
  pageIndex = 0,
): FDocument {
  nodeIdCounter = 0;

  const doc = figmaResponse.document ?? figmaResponse;
  const pages = doc.children ?? [];
  const page = pages[pageIndex] ?? pages[0];

  if (!page) {
    return {
      version: "1.0",
      name: figmaResponse.name ?? "Empty Document",
      children: [],
      meta: { generator: "figma-api" },
    };
  }

  const children = convertChildren(page.children, imageMap);
  const bg = page.backgroundColor
    ? color(page.backgroundColor)
    : undefined;

  return {
    version: "1.0",
    name: page.name ?? figmaResponse.name ?? "Untitled",
    background: bg,
    children,
    meta: {
      figmaFileKey: figmaResponse.key,
      exportedAt: new Date().toISOString(),
      generator: "figma-api",
    },
  };
}

/**
 * Convert a single Figma node response into an FDocument.
 * Used with GET /v1/files/:key/nodes?ids=...
 */
export function convertFigmaNode(
  nodeResponse: any,
  imageMap?: Record<string, string>,
): FDocument {
  nodeIdCounter = 0;

  // The nodes endpoint returns { nodes: { "id": { document: {...} } } }
  const nodesMap = nodeResponse.nodes ?? {};
  const firstKey = Object.keys(nodesMap)[0];
  const nodeData = nodesMap[firstKey]?.document;

  if (!nodeData) {
    return {
      version: "1.0",
      name: "Empty Node",
      children: [],
      meta: { generator: "figma-api" },
    };
  }

  const converted = convertNode(nodeData, imageMap);
  return {
    version: "1.0",
    name: nodeData.name ?? "Untitled",
    width: nodeData.absoluteBoundingBox?.width,
    height: nodeData.absoluteBoundingBox?.height,
    children: converted ? [converted] : [],
    meta: {
      figmaNodeId: firstKey,
      exportedAt: new Date().toISOString(),
      generator: "figma-api",
    },
  };
}

/**
 * Fetch a Figma file and convert it to FDocument.
 * Requires a Figma Personal Access Token.
 */
export async function fetchAndConvertFigmaFile(
  fileKey: string,
  token: string,
  pageIndex = 0,
): Promise<FDocument> {
  const headers = { "X-Figma-Token": token };

  // Fetch file
  const fileRes = await fetch(
    `https://api.figma.com/v1/files/${fileKey}?geometry=paths`,
    { headers },
  );
  if (!fileRes.ok) throw new Error(`Figma API error: ${fileRes.status} ${fileRes.statusText}`);
  const fileData = await fileRes.json();

  // Fetch images
  const imgRes = await fetch(
    `https://api.figma.com/v1/files/${fileKey}/images`,
    { headers },
  );
  let imageMap: Record<string, string> = {};
  if (imgRes.ok) {
    const imgData = await imgRes.json();
    imageMap = imgData.meta?.images ?? {};
  }

  return convertFigmaFile(fileData, imageMap, pageIndex);
}
