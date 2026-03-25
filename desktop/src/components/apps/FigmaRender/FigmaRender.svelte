<svelte:options runes={true} />

<script lang="ts">
  import DocumentRenderer from "$lib/figma-json/DocumentRenderer.svelte";
  import { SAMPLE_DOCUMENT } from "$lib/figma-json/sample-document";
  import { convertFigmaFile } from "$lib/figma-json/figma-converter";
  import { FIGMA_JSON_SYSTEM_PROMPT } from "$lib/figma-json/llm-prompt";
  import type { FDocument } from "$lib/figma-json/types";

  type ViewMode = "preview" | "json" | "figma-import" | "llm-prompt";

  let viewMode = $state<ViewMode>("preview");
  let document = $state<FDocument>(SAMPLE_DOCUMENT);
  let jsonInput = $state("");
  let figmaFileKey = $state("");
  let figmaToken = $state("");
  let llmDescription = $state("");
  let importError = $state("");
  let isLoading = $state(false);
  let previewScale = $state(1);

  // ─── Tauri IPC helpers ─────────────────────────────────────────────
  // Try Tauri invoke first (Rust perf), fall back to TS for browser dev

  async function invokeRust<T>(cmd: string, args: Record<string, unknown>): Promise<T | null> {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      return await invoke<T>(cmd, args);
    } catch {
      return null; // Not in Tauri — fallback to TS
    }
  }

  // Load the sample into the JSON editor
  $effect(() => {
    if (!jsonInput) {
      jsonInput = JSON.stringify(document, null, 2);
    }
  });

  function handleClick(actionId: string) {
    if (actionId === "action:import") {
      viewMode = "figma-import";
    } else if (actionId === "action:view-json") {
      viewMode = "json";
    }
  }

  async function loadFromJson() {
    importError = "";
    try {
      // Try Rust conversion first (handles both FDocument and raw Figma JSON)
      const rustResult = await invokeRust<FDocument>("convert_figma_json", {
        jsonStr: jsonInput,
      });
      if (rustResult) {
        document = rustResult;
        jsonInput = JSON.stringify(document, null, 2);
        viewMode = "preview";
        return;
      }

      // Fallback: TS parser
      const parsed = JSON.parse(jsonInput);
      if (parsed.version && parsed.children) {
        document = parsed as FDocument;
        viewMode = "preview";
      } else if (parsed.document || parsed.children) {
        document = convertFigmaFile(parsed);
        jsonInput = JSON.stringify(document, null, 2);
        viewMode = "preview";
      } else {
        importError = "Invalid JSON: must be an FDocument or Figma file response";
      }
    } catch (e) {
      importError = `Parse error: ${(e as Error).message}`;
    }
  }

  async function importFromFigma() {
    if (!figmaFileKey || !figmaToken) {
      importError = "File key and token are required";
      return;
    }
    importError = "";
    isLoading = true;
    try {
      // Try Rust fetch (network + parse in Rust for perf)
      const rustResult = await invokeRust<FDocument>("fetch_figma_file", {
        fileKey: figmaFileKey,
        token: figmaToken,
        pageIndex: 0,
      });
      if (rustResult) {
        document = rustResult;
        jsonInput = JSON.stringify(document, null, 2);
        viewMode = "preview";
        return;
      }

      // Fallback: TS fetch
      const { fetchAndConvertFigmaFile } = await import("$lib/figma-json/figma-converter");
      document = await fetchAndConvertFigmaFile(figmaFileKey, figmaToken);
      jsonInput = JSON.stringify(document, null, 2);
      viewMode = "preview";
    } catch (e) {
      importError = `Figma import error: ${(e as Error).message}`;
    } finally {
      isLoading = false;
    }
  }

  async function loadSample() {
    // Try Rust sample
    const rustSample = await invokeRust<FDocument>("get_figma_sample", {});
    document = rustSample ?? SAMPLE_DOCUMENT;
    jsonInput = JSON.stringify(document, null, 2);
    viewMode = "preview";
  }

  async function copyLLMPrompt() {
    const desc = llmDescription || "A modern settings panel with toggles, sliders, and a save button";

    // Try Rust prompt builder
    const rustPrompt = await invokeRust<{ system: string; user: string }>(
      "build_figma_llm_prompt",
      { description: desc },
    );

    if (rustPrompt) {
      const full = `SYSTEM:\n${rustPrompt.system}\n\nUSER:\n${rustPrompt.user}`;
      navigator.clipboard.writeText(full);
    } else {
      // Fallback: TS
      const { buildLLMPrompt } = await import("$lib/figma-json/llm-prompt");
      const prompt = buildLLMPrompt(desc);
      const full = `SYSTEM:\n${prompt.system}\n\nUSER:\n${prompt.user}`;
      navigator.clipboard.writeText(full);
    }
  }

  function copySchemaPrompt() {
    navigator.clipboard.writeText(FIGMA_JSON_SYSTEM_PROMPT);
  }
</script>

<div class="flex h-full flex-col bg-gray-950 text-white">
  <!-- Toolbar -->
  <div class="flex items-center gap-1 border-b border-white/10 bg-gray-900/80 px-3 py-2">
    <button
      class="rounded-md px-3 py-1.5 text-xs font-medium transition-colors {viewMode === 'preview' ? 'bg-white/10 text-white' : 'text-white/50 hover:text-white/80'}"
      onclick={() => viewMode = "preview"}
    >
      Preview
    </button>
    <button
      class="rounded-md px-3 py-1.5 text-xs font-medium transition-colors {viewMode === 'json' ? 'bg-white/10 text-white' : 'text-white/50 hover:text-white/80'}"
      onclick={() => {
        jsonInput = JSON.stringify(document, null, 2);
        viewMode = "json";
      }}
    >
      JSON Editor
    </button>
    <button
      class="rounded-md px-3 py-1.5 text-xs font-medium transition-colors {viewMode === 'figma-import' ? 'bg-white/10 text-white' : 'text-white/50 hover:text-white/80'}"
      onclick={() => viewMode = "figma-import"}
    >
      Figma Import
    </button>
    <button
      class="rounded-md px-3 py-1.5 text-xs font-medium transition-colors {viewMode === 'llm-prompt' ? 'bg-white/10 text-white' : 'text-white/50 hover:text-white/80'}"
      onclick={() => viewMode = "llm-prompt"}
    >
      LLM Agent
    </button>

    <div class="ml-auto flex items-center gap-2">
      {#if viewMode === "preview"}
        <span class="text-[10px] text-white/30">{Math.round(previewScale * 100)}%</span>
        <input
          type="range"
          min="0.25"
          max="2"
          step="0.05"
          bind:value={previewScale}
          class="h-1 w-20 accent-blue-500"
        />
      {/if}
      <button
        class="rounded-md bg-white/5 px-3 py-1.5 text-xs text-white/60 transition-colors hover:bg-white/10 hover:text-white"
        onclick={loadSample}
      >
        Load Sample
      </button>
    </div>
  </div>

  <!-- Content -->
  <div class="min-h-0 flex-1 overflow-auto">
    {#if viewMode === "preview"}
      <div class="flex h-full items-center justify-center p-8" style="background: repeating-conic-gradient(rgba(255,255,255,0.03) 0% 25%, transparent 0% 50%) 0 0 / 20px 20px;">
        <DocumentRenderer {document} onClick={handleClick} scale={previewScale} />
      </div>

    {:else if viewMode === "json"}
      <div class="flex h-full flex-col">
        <div class="flex items-center gap-2 border-b border-white/5 px-4 py-2">
          <span class="text-xs text-white/40">FDocument JSON</span>
          <div class="ml-auto flex gap-2">
            <button
              class="rounded-md bg-white/5 px-3 py-1 text-xs text-white/60 transition-colors hover:bg-white/10 hover:text-white"
              onclick={() => navigator.clipboard.writeText(jsonInput)}
            >
              Copy
            </button>
            <button
              class="rounded-md bg-blue-600/80 px-3 py-1 text-xs font-medium text-white transition-colors hover:bg-blue-600"
              onclick={loadFromJson}
            >
              Render
            </button>
          </div>
        </div>
        {#if importError}
          <div class="border-b border-red-500/20 bg-red-500/10 px-4 py-2 text-xs text-red-400">
            {importError}
          </div>
        {/if}
        <textarea
          bind:value={jsonInput}
          class="min-h-0 flex-1 resize-none border-0 bg-transparent p-4 font-mono text-xs leading-relaxed text-white/80 outline-none placeholder:text-white/20"
          placeholder="Paste FDocument JSON or raw Figma node JSON here..."
          spellcheck="false"
        ></textarea>
      </div>

    {:else if viewMode === "figma-import"}
      <div class="mx-auto flex max-w-lg flex-col gap-6 p-8">
        <div>
          <h3 class="mb-1 text-sm font-semibold text-white/90">Import from Figma</h3>
          <p class="text-xs leading-relaxed text-white/40">
            Enter your Figma file key and personal access token to import a design.
            The file key is the part after <code class="rounded bg-white/5 px-1">/file/</code> in the Figma URL.
          </p>
        </div>

        <div class="flex flex-col gap-3">
          <label class="flex flex-col gap-1">
            <span class="text-xs text-white/50">File Key</span>
            <input
              type="text"
              bind:value={figmaFileKey}
              placeholder="e.g. abc123XYZ..."
              class="rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm text-white outline-none transition-colors focus:border-blue-500/50 placeholder:text-white/20"
            />
          </label>
          <label class="flex flex-col gap-1">
            <span class="text-xs text-white/50">Personal Access Token</span>
            <input
              type="password"
              bind:value={figmaToken}
              placeholder="figd_..."
              class="rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm text-white outline-none transition-colors focus:border-blue-500/50 placeholder:text-white/20"
            />
          </label>
        </div>

        {#if importError}
          <div class="rounded-lg bg-red-500/10 px-4 py-2 text-xs text-red-400">
            {importError}
          </div>
        {/if}

        <button
          class="rounded-lg bg-blue-600 px-4 py-2.5 text-sm font-medium text-white transition-colors hover:bg-blue-500 disabled:cursor-not-allowed disabled:opacity-50"
          onclick={importFromFigma}
          disabled={isLoading || !figmaFileKey || !figmaToken}
        >
          {isLoading ? "Importing..." : "Import File"}
        </button>

        <div class="rounded-lg border border-white/5 bg-white/[0.02] p-4">
          <p class="mb-2 text-xs font-medium text-white/60">Or paste raw Figma JSON</p>
          <p class="text-[11px] leading-relaxed text-white/30">
            You can also copy JSON from the Figma API inspector or DevTools network tab
            and paste it in the JSON Editor tab. The converter will auto-detect Figma format.
          </p>
        </div>
      </div>

    {:else if viewMode === "llm-prompt"}
      <div class="mx-auto flex max-w-lg flex-col gap-6 p-8">
        <div>
          <h3 class="mb-1 text-sm font-semibold text-white/90">LLM Auto-Generate</h3>
          <p class="text-xs leading-relaxed text-white/40">
            Describe a UI and copy the prompt to send to an LLM. The LLM will generate
            valid FDocument JSON that you can paste back into the JSON Editor.
          </p>
        </div>

        <label class="flex flex-col gap-1">
          <span class="text-xs text-white/50">UI Description</span>
          <textarea
            bind:value={llmDescription}
            rows="4"
            placeholder="e.g. A modern login form with email, password, and a gradient submit button..."
            class="rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm leading-relaxed text-white outline-none transition-colors focus:border-purple-500/50 placeholder:text-white/20"
          ></textarea>
        </label>

        <div class="flex gap-2">
          <button
            class="flex-1 rounded-lg bg-purple-600 px-4 py-2.5 text-sm font-medium text-white transition-colors hover:bg-purple-500"
            onclick={copyLLMPrompt}
          >
            Copy Full Prompt
          </button>
          <button
            class="rounded-lg bg-white/5 px-4 py-2.5 text-sm text-white/60 transition-colors hover:bg-white/10 hover:text-white"
            onclick={copySchemaPrompt}
          >
            Copy Schema Only
          </button>
        </div>

        <div class="rounded-lg border border-white/5 bg-white/[0.02] p-4">
          <p class="mb-2 text-xs font-medium text-white/60">How it works</p>
          <ol class="flex flex-col gap-1.5 text-[11px] leading-relaxed text-white/30">
            <li>1. Describe the UI you want above</li>
            <li>2. Click "Copy Full Prompt" to copy the system + user prompt</li>
            <li>3. Paste into ChatGPT, Claude, or any LLM</li>
            <li>4. Copy the JSON output from the LLM</li>
            <li>5. Paste into the JSON Editor tab and click "Render"</li>
          </ol>
        </div>
      </div>
    {/if}
  </div>

  <!-- Status bar -->
  <div class="flex items-center justify-between border-t border-white/5 bg-gray-900/60 px-3 py-1.5">
    <span class="text-[10px] text-white/25">
      {document.name}
      {#if document.meta?.generator}
        · {document.meta.generator}
      {/if}
    </span>
    <span class="text-[10px] text-white/25">
      {document.children.length} root node{document.children.length !== 1 ? "s" : ""}
      · v{document.version}
    </span>
  </div>
</div>
