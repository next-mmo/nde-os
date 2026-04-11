/**
 * @json-render integration for NDE-OS
 *
 * Re-exports the catalog, registry, and Vercel's Renderer + streaming
 * so consumers only need: import { ... } from "$lib/json-render"
 */

// NDE-OS catalog & registry
export { catalog, systemPrompt } from "./catalog";
export { registry, handlers, executeAction } from "./registry";

// Re-export Vercel's Svelte renderer + streaming
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

// Re-export the official shadcn-svelte preset
export {
  shadcnComponents,
  shadcnComponentDefinitions,
} from "@json-render/shadcn-svelte";

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
