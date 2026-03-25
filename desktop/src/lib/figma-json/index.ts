/**
 * Barrel export for the figma-json render engine
 */

// Types
export type {
  FDocument,
  FNode,
  FFrameNode,
  FTextNode,
  FRectangleNode,
  FEllipseNode,
  FVectorNode,
  FImageNode,
  FColor,
  FFill,
  FStroke,
  FShadow,
  FBlur,
  FGradientStop,
  FConstraint,
  FLayoutMode,
  FLayoutAlign,
  FLayoutSizing,
} from "./types";

// Style resolver
export {
  resolveNodeStyles,
  stylesToString,
  fColorToCSS,
  resolveFills,
  resolveStrokes,
  resolveEffects,
  resolveBorderRadius,
  resolveLayout,
  resolveTextStyles,
} from "./style-resolver";

// Figma converter
export {
  convertFigmaFile,
  convertFigmaNode,
  fetchAndConvertFigmaFile,
} from "./figma-converter";

// LLM prompt
export {
  FIGMA_JSON_SYSTEM_PROMPT,
  FIGMA_JSON_USER_PROMPT_TEMPLATE,
  buildLLMPrompt,
} from "./llm-prompt";
