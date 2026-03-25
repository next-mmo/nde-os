<svelte:options runes={true} />

<script lang="ts">
  import { onMount } from "svelte";
  import { Renderer } from "@json-render/svelte";
  import { registry } from "🍎/lib/json-render/registry";
  import { systemPrompt } from "🍎/lib/json-render/catalog";
  import { validateSpec } from "@json-render/core";

  // ── Config from Vite env ───────────────────────────────────────────
  const ENV_KEY  = import.meta.env.VITE_DASHSCOPE_KEY  ?? "";
  const ENV_BASE = import.meta.env.VITE_DASHSCOPE_BASE ?? "https://dashscope.aliyuncs.com/compatible-mode/v1";

  // ── State ──────────────────────────────────────────────────────────
  let prompt = $state("");
  let selectedModel = $state("qwen2.5-coder-32b-instruct");
  let apiKey = $state(ENV_KEY);

  let isGenerating = $state(false);
  let currentSpec = $state<any>(null);
  let errorMsg = $state<string | null>(null);
  let chatPanelOpen = $state(true);

  const API_BASE = ENV_BASE;

  interface ChatMsg {
    role: "user" | "assistant";
    content: string;
    spec?: any;
    streaming?: boolean;
    error?: boolean;
  }

  let messages = $state<ChatMsg[]>([
    {
      role: "assistant",
      content:
        "Hi! Describe any UI you want and I'll generate it as a live interactive interface using the NDE-OS component system.",
    },
  ]);

  let chatBottom: HTMLDivElement | null = null;

  function scrollToBottom() {
    requestAnimationFrame(() => {
      chatBottom?.scrollIntoView({ behavior: "smooth" });
    });
  }

  // Auto-run card demo on first load
  onMount(() => {
    prompt = "Build a real-time system health dashboard with CPU usage metric at 73%, memory at 58%, disk at 42%, a running processes list (ollama, comfyui, whisper), and a stop button for each process. Use cards, metrics, progress bars, and status dots.";
    send();
  });

  // ── Parse JSONL patch stream ───────────────────────────────────────
  function parseStreamedSpec(raw: string): any {
    let content = raw.trim();
    const fenceMatch = content.match(/```(?:jsonl?|text)?\s*([\s\S]*?)```/);
    if (fenceMatch) content = fenceMatch[1].trim();
    try {
      const parsed = JSON.parse(content);
      if (parsed.root && parsed.elements) return parsed;
    } catch {}

    const lines = content.split("\n").filter((l) => l.trim().startsWith("{"));
    let spec: any = {};
    for (const line of lines) {
      try {
        const patch = JSON.parse(line.trim());
        if (patch.op === "add" || patch.op === "replace") {
          const parts = patch.path.split("/").filter(Boolean);
          let current = spec;
          for (let i = 0; i < parts.length - 1; i++) {
            const key = parts[i];
            if (!(key in current)) current[key] = {};
            current = current[key];
          }
          const lastKey = parts[parts.length - 1];
          if (lastKey !== undefined) current[lastKey] = patch.value;
        }
      } catch {}
    }
    return spec;
  }

  // ── Send message ───────────────────────────────────────────────────
  async function send() {
    const text = prompt.trim();
    if (!text || isGenerating) return;
    prompt = "";
    isGenerating = true;
    errorMsg = null;

    messages.push({ role: "user", content: text });
    const assistantIdx = messages.length;
    messages.push({ role: "assistant", content: "", streaming: true });
    scrollToBottom();

    try {
      // Build conversation history for context
      const history = messages
        .slice(0, assistantIdx)
        .filter((m) => m.role === "user" || (m.role === "assistant" && m.content && !m.streaming))
        .map((m) => ({ role: m.role, content: m.content }));

      const res = await fetch(`${API_BASE}/chat/completions`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${apiKey}`,
        },
        body: JSON.stringify({
          model: selectedModel,
          messages: [{ role: "system", content: systemPrompt }, ...history],
          temperature: 0.3,
          stream: true,
        }),
      });

      if (!res.ok) throw new Error(`API error ${res.status}`);

      const reader = res.body?.getReader();
      const decoder = new TextDecoder();
      let fullContent = "";

      if (reader) {
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;
          const chunk = decoder.decode(value, { stream: true });
          for (const line of chunk.split("\n")) {
            if (line === "data: [DONE]" || !line.startsWith("data: ")) continue;
            try {
              const data = JSON.parse(line.slice(6));
              const delta = data.choices?.[0]?.delta?.content || "";
              fullContent += delta;
              messages[assistantIdx].content = fullContent;

              // Real-time progressive rendering
              try {
                const spec = parseStreamedSpec(fullContent);
                if (spec.root && spec.elements) {
                  currentSpec = { ...spec };
                  messages[assistantIdx].spec = spec;
                }
              } catch {}

              scrollToBottom();
            } catch {}
          }
        }
      }

      // Final parse
      const spec = parseStreamedSpec(fullContent);
      const result = validateSpec(spec);
      if (result.valid) {
        currentSpec = spec;
        messages[assistantIdx].spec = spec;
      } else {
        errorMsg = "Generated spec had issues: " + result.issues.map((i: any) => i.message).join(", ");
      }
      messages[assistantIdx].streaming = false;
    } catch (e: any) {
      messages[assistantIdx].content = e.message;
      messages[assistantIdx].error = true;
      messages[assistantIdx].streaming = false;
      errorMsg = e.message;
    } finally {
      isGenerating = false;
      scrollToBottom();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }

  function usePrompt(text: string) {
    prompt = text;
    send();
  }

  // When a past message has a spec, click to restore it
  function restoreSpec(spec: any) {
    if (spec) currentSpec = spec;
  }
</script>

<div class="pg-root">
  <!-- ── LEFT: Live Preview (80%) ────────────────────────────────── -->
  <main class="preview-area">
    <!-- Top bar -->
    <div class="preview-topbar">
      <div class="preview-title">
        <span class="logo-pill">AI → UI</span>
        {#if errorMsg}
          <span class="error-tag">{errorMsg}</span>
        {:else if isGenerating}
          <span class="generating-tag">
            <span class="dot-pulse"></span>
            Generating…
          </span>
        {/if}
      </div>
      <div class="topbar-right">
        <select bind:value={selectedModel} class="model-select">
          <option value="qwen2.5-coder-32b-instruct">Qwen 2.5 Coder 32B</option>
          <option value="deepseek-v3">DeepSeek V3</option>
          <option value="deepseek-r1">DeepSeek R1</option>
        </select>
        <input type="password" bind:value={apiKey} placeholder="API Key" class="key-input" />
        <button
          class="toggle-chat-btn"
          onclick={() => (chatPanelOpen = !chatPanelOpen)}
          title={chatPanelOpen ? "Hide chat" : "Show chat"}
        >
          {#if chatPanelOpen}
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M9 18l6-6-6-6" />
            </svg>
          {:else}
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
            </svg>
          {/if}
        </button>
      </div>
    </div>

    <!-- Canvas -->
    <div class="preview-canvas {!currentSpec ? 'empty' : ''}">
      {#if currentSpec}
        <Renderer spec={currentSpec} {registry} />
      {:else}
        <div class="empty-state">
          <div class="empty-icon">✦</div>
          <h2>Describe any UI</h2>
          <p>Type a prompt in the chat panel to generate a live interface</p>
          <div class="chips">
            <button class="chip" onclick={() => usePrompt("Create a login form with email and password")}>Login form</button>
            <button class="chip" onclick={() => usePrompt("System health dashboard with CPU, memory and disk")}>System dashboard</button>
            <button class="chip" onclick={() => usePrompt("Show installed AI apps with status badges")}>App catalog</button>
          </div>
        </div>
      {/if}
    </div>
  </main>

  <!-- ── RIGHT: Chat Slider (20%) ───────────────────────────────── -->
  <aside class="chat-panel {chatPanelOpen ? 'open' : 'closed'}">
    <div class="chat-header">
      <span class="chat-title">Chat</span>
      <span class="chat-sub">Powered by {selectedModel.split("-").slice(0, 2).join(" ")}</span>
    </div>

    <div class="chat-messages">
      {#each messages as msg}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="msg {msg.role} {msg.error ? 'error' : ''}"
          onclick={() => msg.spec && restoreSpec(msg.spec)}
          title={msg.spec ? "Click to restore this UI" : undefined}
          style={msg.spec ? "cursor:pointer" : ""}
        >
          {#if msg.role === "assistant"}
            <div class="msg-avatar">✦</div>
          {/if}
          <div class="msg-body">
            {#if msg.content}
              <div class="msg-text">{msg.content}</div>
            {/if}
            {#if msg.streaming && !msg.content}
              <div class="msg-typing">
                <span></span><span></span><span></span>
              </div>
            {/if}
            {#if msg.spec}
              <div class="msg-spec-badge">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M9 16h6l1-4H8l1 4zm1-8h4v2h-4V8z"/>
                </svg>
                UI generated · click to restore
              </div>
            {/if}
          </div>
          {#if msg.role === "user"}
            <div class="msg-avatar user-avatar">U</div>
          {/if}
        </div>
      {/each}
      <div bind:this={chatBottom}></div>
    </div>

    <div class="chat-input-area">
      <textarea
        bind:value={prompt}
        onkeydown={handleKeydown}
        placeholder="Describe a UI… (Enter to send)"
        disabled={isGenerating}
        rows="3"
      ></textarea>
      <button class="send-btn" onclick={send} disabled={isGenerating || !prompt.trim()}>
        {#if isGenerating}
          <div class="send-spinner"></div>
        {:else}
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
            <path d="M22 2L11 13" /><path d="M22 2L15 22 11 13 2 9l20-7z" />
          </svg>
        {/if}
      </button>
    </div>
  </aside>
</div>

<style>
  /* ── Root Layout ─────────────────────────────────────────── */
  .pg-root {
    display: flex;
    height: 100%;
    overflow: hidden;
    background: hsl(228 8% 7%);
    color: hsl(0 0% 92%);
    font-family: var(--font-sans, system-ui);
  }

  /* ── Preview Area (left, 80%) ───────────────────────────── */
  .preview-area {
    flex: 0 0 80%;
    width: 80%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-width: 0;
    transition: flex-basis 0.25s cubic-bezier(0.4, 0, 0.2, 1),
                width 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  }

  /* When chat is closed, preview takes full width */
  .pg-root:has(.chat-panel.closed) .preview-area {
    flex-basis: 100%;
    width: 100%;
  }

  .preview-topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.6rem 1.25rem;
    border-bottom: 1px solid hsl(0 0% 100% / 0.07);
    background: hsl(228 8% 9%);
    gap: 1rem;
    flex-shrink: 0;
  }

  .preview-title {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    min-width: 0;
  }

  .logo-pill {
    font-size: 0.7rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 0.2rem 0.6rem;
    border-radius: 999px;
    background: hsl(250 80% 60% / 0.15);
    color: hsl(250 80% 75%);
    border: 1px solid hsl(250 80% 60% / 0.25);
    white-space: nowrap;
  }

  .error-tag {
    font-size: 0.72rem;
    color: hsl(0 70% 65%);
    background: hsl(0 70% 55% / 0.1);
    border: 1px solid hsl(0 70% 55% / 0.2);
    border-radius: 4px;
    padding: 0.15rem 0.5rem;
    max-width: 300px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .generating-tag {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.75rem;
    color: hsl(250 80% 75%);
  }

  .dot-pulse {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: hsl(250 80% 65%);
    animation: pulse 1s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.4; transform: scale(0.9); }
    50% { opacity: 1; transform: scale(1.1); }
  }

  .topbar-right {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .model-select,
  .key-input {
    background: hsl(0 0% 100% / 0.06);
    border: 1px solid hsl(0 0% 100% / 0.1);
    color: hsl(0 0% 85%);
    padding: 0.3rem 0.6rem;
    border-radius: 6px;
    font-size: 0.78rem;
  }

  .model-select {
    cursor: pointer;
  }

  .key-input {
    width: 160px;
    font-family: monospace;
  }

  .toggle-chat-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border-radius: 6px;
    background: hsl(0 0% 100% / 0.06);
    border: 1px solid hsl(0 0% 100% / 0.1);
    color: hsl(0 0% 80%);
    cursor: pointer;
    transition: background 0.15s;
  }

  .toggle-chat-btn:hover {
    background: hsl(0 0% 100% / 0.12);
  }

  .preview-canvas {
    flex: 1;
    overflow-y: auto;
    padding: 2rem;
  }

  .preview-canvas.empty {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  /* ── Empty State ──────────────────────────────────────────── */
  .empty-state {
    text-align: center;
    max-width: 420px;
  }

  .empty-icon {
    font-size: 2.5rem;
    margin-bottom: 1rem;
    color: hsl(250 80% 65%);
    animation: float 3s ease-in-out infinite;
  }

  @keyframes float {
    0%, 100% { transform: translateY(0); }
    50% { transform: translateY(-6px); }
  }

  .empty-state h2 {
    margin: 0 0 0.5rem;
    font-size: 1.4rem;
    font-weight: 700;
    letter-spacing: -0.02em;
    color: hsl(0 0% 95%);
  }

  .empty-state p {
    margin: 0 0 1.5rem;
    font-size: 0.9rem;
    color: hsl(0 0% 60%);
    line-height: 1.5;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    justify-content: center;
  }

  .chip {
    background: transparent;
    border: 1px solid hsl(250 60% 60% / 0.35);
    color: hsl(250 70% 75%);
    padding: 0.3rem 0.85rem;
    border-radius: 999px;
    font-size: 0.78rem;
    cursor: pointer;
    transition: all 0.15s;
  }

  .chip:hover {
    background: hsl(250 60% 60% / 0.15);
    border-color: hsl(250 60% 65% / 0.6);
    color: hsl(250 80% 85%);
  }

  /* ── Chat Panel (right, 20%) ─────────────────────────────── */
  .chat-panel {
    display: flex;
    flex-direction: column;
    flex: 0 0 20%;
    width: 20%;
    min-width: 240px;
    max-width: 400px;
    background: hsl(228 8% 10%);
    border-left: 1px solid hsl(0 0% 100% / 0.07);
    overflow: hidden;
    transition: flex-basis 0.25s cubic-bezier(0.4, 0, 0.2, 1),
                width 0.25s cubic-bezier(0.4, 0, 0.2, 1),
                opacity 0.2s ease,
                min-width 0.25s ease;
  }

  .chat-panel.closed {
    flex-basis: 0;
    width: 0;
    min-width: 0;
    opacity: 0;
    border-left-width: 0;
    pointer-events: none;
  }

  .chat-header {
    display: flex;
    flex-direction: column;
    padding: 0.9rem 1rem 0.75rem;
    border-bottom: 1px solid hsl(0 0% 100% / 0.07);
    flex-shrink: 0;
  }

  .chat-title {
    font-size: 0.88rem;
    font-weight: 700;
    color: hsl(0 0% 95%);
  }

  .chat-sub {
    font-size: 0.7rem;
    color: hsl(0 0% 50%);
    margin-top: 0.1rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* ── Messages ─────────────────────────────────────────────── */
  .chat-messages {
    flex: 1;
    overflow-y: auto;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    scrollbar-width: thin;
    scrollbar-color: hsl(0 0% 30%) transparent;
  }

  .msg {
    display: flex;
    gap: 0.5rem;
    align-items: flex-start;
    animation: fadeUp 0.2s ease;
  }

  @keyframes fadeUp {
    from { opacity: 0; transform: translateY(6px); }
    to { opacity: 1; transform: translateY(0); }
  }

  .msg.user {
    flex-direction: row-reverse;
  }

  .msg-avatar {
    font-size: 0.75rem;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    background: hsl(250 80% 55% / 0.15);
    color: hsl(250 80% 75%);
    border: 1px solid hsl(250 80% 55% / 0.2);
  }

  .user-avatar {
    background: hsl(210 80% 55% / 0.15);
    color: hsl(210 80% 75%);
    border-color: hsl(210 80% 55% / 0.2);
    font-size: 0.65rem;
    font-weight: 700;
  }

  .msg-body {
    max-width: 100%;
    min-width: 0;
  }

  .msg.user .msg-body {
    text-align: right;
  }

  .msg-text {
    font-size: 0.8rem;
    line-height: 1.5;
    padding: 0.5rem 0.7rem;
    border-radius: 10px;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .msg.assistant .msg-text {
    background: hsl(0 0% 100% / 0.05);
    border: 1px solid hsl(0 0% 100% / 0.07);
    color: hsl(0 0% 88%);
    max-height: 180px;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: hsl(0 0% 30%) transparent;
    font-family: ui-monospace, monospace;
    font-size: 0.72rem;
  }

  .msg.user .msg-text {
    background: hsl(250 70% 55% / 0.2);
    border: 1px solid hsl(250 70% 55% / 0.3);
    color: hsl(250 60% 90%);
  }

  .msg.error .msg-text {
    background: hsl(0 70% 55% / 0.12);
    border-color: hsl(0 70% 55% / 0.25);
    color: hsl(0 70% 70%);
  }

  /* Typing indicator */
  .msg-typing {
    display: flex;
    gap: 3px;
    padding: 0.6rem 0.8rem;
    background: hsl(0 0% 100% / 0.05);
    border: 1px solid hsl(0 0% 100% / 0.07);
    border-radius: 10px;
    width: fit-content;
  }

  .msg-typing span {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: hsl(0 0% 60%);
    animation: blink 1.2s ease-in-out infinite;
  }

  .msg-typing span:nth-child(2) { animation-delay: 0.2s; }
  .msg-typing span:nth-child(3) { animation-delay: 0.4s; }

  @keyframes blink {
    0%, 80%, 100% { opacity: 0.3; transform: scale(0.9); }
    40% { opacity: 1; transform: scale(1.1); }
  }

  .msg-spec-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    margin-top: 0.35rem;
    font-size: 0.65rem;
    color: hsl(145 65% 55%);
    background: hsl(145 65% 50% / 0.1);
    border: 1px solid hsl(145 65% 50% / 0.2);
    border-radius: 4px;
    padding: 0.15rem 0.45rem;
  }

  /* ── Chat Input ───────────────────────────────────────────── */
  .chat-input-area {
    display: flex;
    gap: 0.5rem;
    align-items: flex-end;
    padding: 0.75rem;
    border-top: 1px solid hsl(0 0% 100% / 0.07);
    flex-shrink: 0;
  }

  .chat-input-area textarea {
    flex: 1;
    background: hsl(0 0% 100% / 0.06);
    border: 1px solid hsl(0 0% 100% / 0.12);
    color: hsl(0 0% 90%);
    padding: 0.55rem 0.7rem;
    border-radius: 8px;
    font-size: 0.8rem;
    line-height: 1.5;
    resize: none;
    font-family: inherit;
    transition: border-color 0.15s;
    max-height: 120px;
    scrollbar-width: thin;
  }

  .chat-input-area textarea:focus {
    outline: none;
    border-color: hsl(250 70% 60% / 0.5);
  }

  .chat-input-area textarea::placeholder {
    color: hsl(0 0% 45%);
  }

  .chat-input-area textarea:disabled {
    opacity: 0.5;
  }

  .send-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    border-radius: 8px;
    background: hsl(250 80% 60%);
    border: none;
    color: white;
    cursor: pointer;
    flex-shrink: 0;
    transition: opacity 0.15s, background 0.15s;
  }

  .send-btn:hover:not(:disabled) {
    background: hsl(250 80% 55%);
  }

  .send-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .send-spinner {
    width: 14px;
    height: 14px;
    border: 2px solid hsl(0 0% 100% / 0.3);
    border-top-color: white;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
