/**
 * @json-render integration for NDE-OS
 *
 * Re-exports the catalog, registry, and vercel's Renderer + streaming
 * so consumers only need: import { ... } from "$lib/json-render"
 */

// NDE-OS catalog & registry
export { catalog, systemPrompt } from "./catalog";
export { registry } from "./registry";

// Re-export vercel's Svelte renderer + streaming
export {
  Renderer,
  CatalogRenderer,
  JsonUIProvider,
  createUIStream,
  createChatUI,
  defineRegistry,
  flatToTree,
  buildSpecFromParts,
  getTextFromParts,
} from "@json-render/svelte";

// Re-export core utilities
export {
  defineCatalog,
  createSpecStreamCompiler,
  buildUserPrompt,
  buildEditUserPrompt,
  validateSpec,
  createStateStore,
  createJsonRenderTransform,
} from "@json-render/core";

// Re-export types
export type {
  Spec,
  UIElement,
  ActionBinding,
  ActionHandler,
} from "@json-render/core";

export type {
  UIStreamOptions,
  UIStreamReturn,
  UIStreamState,
  ChatUIOptions,
  ChatUIReturn,
  ChatMessage,
} from "@json-render/svelte";
