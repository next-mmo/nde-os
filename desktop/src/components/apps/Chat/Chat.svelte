<svelte:options runes={true} />

<script lang="ts">
  import * as api from "$lib/api/backend";
  import type { ConversationSummary } from "$lib/api/types";

  type ChatMessage = {
    id: number;
    role: "user" | "assistant" | "system";
    content: string;
    timestamp: string;
  };

  let messages = $state<ChatMessage[]>([]);
  let input = $state("");
  let loading = $state(false);
  let conversationId = $state<string | null>(null);
  let conversations = $state<ConversationSummary[]>([]);
  let sidebarOpen = $state(false);
  let agentInfo = $state<{ provider: string; model: string; name: string } | null>(null);
  let messagesEl: HTMLDivElement | undefined = $state();
  let msgCounter = $state(0);

  // Load agent config on mount
  $effect(() => {
    api.agentConfig().then((config) => {
      agentInfo = { provider: config.provider, model: config.model, name: config.name };
    }).catch(() => {});

    api.agentConversations().then((convs) => {
      conversations = convs;
    }).catch(() => {});
  });

  // Auto-scroll
  $effect(() => {
    // Access messages.length to create dependency
    const _len = messages.length;
    if (messagesEl) {
      requestAnimationFrame(() => {
        messagesEl!.scrollTop = messagesEl!.scrollHeight;
      });
    }
  });

  async function sendMessage() {
    const text = input.trim();
    if (!text || loading) return;

    msgCounter++;
    messages.push({
      id: msgCounter,
      role: "user",
      content: text,
      timestamp: new Date().toISOString(),
    });
    input = "";
    loading = true;

    try {
      const resp = await api.agentChat(text, conversationId ?? undefined);
      conversationId = resp.conversation_id;
      msgCounter++;
      messages.push({
        id: msgCounter,
        role: "assistant",
        content: resp.response,
        timestamp: new Date().toISOString(),
      });

      // Refresh conversation list
      api.agentConversations().then((convs) => { conversations = convs; }).catch(() => {});
    } catch (e: any) {
      msgCounter++;
      messages.push({
        id: msgCounter,
        role: "system",
        content: `Error: ${e.message || e}`,
        timestamp: new Date().toISOString(),
      });
    } finally {
      loading = false;
    }
  }

  async function loadConversation(convId: string) {
    try {
      const msgs = await api.agentMessages(convId);
      conversationId = convId;
      msgCounter = 0;
      messages = msgs
        .filter((m) => m.content)
        .map((m) => {
          msgCounter++;
          return {
            id: msgCounter,
            role: m.role as "user" | "assistant",
            content: m.content!,
            timestamp: m.created_at,
          };
        });
      sidebarOpen = false;
    } catch (e: any) {
      console.error("Failed to load conversation:", e);
    }
  }

  function newChat() {
    messages = [];
    conversationId = null;
    input = "";
    sidebarOpen = false;
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  }
</script>

<section class="chat-app">
  <!-- Sidebar -->
  {#if sidebarOpen}
    <aside class="sidebar">
      <div class="sidebar-header">
        <h3>Conversations</h3>
        <button class="new-btn" onclick={newChat}>+ New</button>
      </div>
      <div class="conv-list">
        {#each conversations as conv (conv.id)}
          <button
            class="conv-item"
            class:active={conversationId === conv.id}
            onclick={() => loadConversation(conv.id)}
          >
            <span class="conv-title">{conv.title}</span>
            <span class="conv-time">{new Date(conv.updated_at).toLocaleDateString()}</span>
          </button>
        {:else}
          <p class="no-convs">No conversations yet.</p>
        {/each}
      </div>
    </aside>
  {/if}

  <!-- Main chat area -->
  <div class="chat-main">
    <!-- Header -->
    <div class="chat-header">
      <button class="menu-btn" onclick={() => sidebarOpen = !sidebarOpen}>
        ☰
      </button>
      <div class="header-info">
        <h2>NDE Chat</h2>
        {#if agentInfo}
          <span class="provider-badge">{agentInfo.provider} · {agentInfo.model}</span>
        {/if}
      </div>
      <button class="new-chat-btn" onclick={newChat}>New Chat</button>
    </div>

    <!-- Messages -->
    <div class="messages" bind:this={messagesEl}>
      {#if messages.length === 0}
        <div class="empty-state">
          <div class="empty-icon">🤖</div>
          <h3>NDE-OS Agent</h3>
          <p>Ask me anything. I can read/write files and run commands in the sandbox.</p>
          <div class="suggestions">
            <button class="suggestion" onclick={() => { input = "What tools do you have?"; sendMessage(); }}>
              What tools do you have?
            </button>
            <button class="suggestion" onclick={() => { input = "List files in the workspace"; sendMessage(); }}>
              List files in workspace
            </button>
            <button class="suggestion" onclick={() => { input = "Hello! Tell me about NDE-OS."; sendMessage(); }}>
              Tell me about NDE-OS
            </button>
          </div>
        </div>
      {:else}
        {#each messages as msg (msg.id)}
          <div class="msg" class:user={msg.role === "user"} class:assistant={msg.role === "assistant"} class:system-msg={msg.role === "system"}>
            <div class="msg-avatar">
              {#if msg.role === "user"}👤{:else if msg.role === "assistant"}🤖{:else}⚠️{/if}
            </div>
            <div class="msg-content">
              <div class="msg-role">{msg.role === "user" ? "You" : msg.role === "assistant" ? "Agent" : "System"}</div>
              <div class="msg-text">{msg.content}</div>
            </div>
          </div>
        {/each}

        {#if loading}
          <div class="msg assistant">
            <div class="msg-avatar">🤖</div>
            <div class="msg-content">
              <div class="msg-role">Agent</div>
              <div class="msg-text typing">
                <span class="dot"></span><span class="dot"></span><span class="dot"></span>
              </div>
            </div>
          </div>
        {/if}
      {/if}
    </div>

    <!-- Input -->
    <div class="input-area">
      <textarea
        class="chat-input"
        placeholder="Send a message..."
        bind:value={input}
        onkeydown={handleKeyDown}
        rows="1"
        disabled={loading}
      ></textarea>
      <button class="send-btn" onclick={sendMessage} disabled={!input.trim() || loading}>
        {loading ? "..." : "↑"}
      </button>
    </div>
  </div>
</section>

<style>
  .chat-app {
    height: 100%;
    display: flex;
    overflow: hidden;
    background: var(--system-color-bg, hsl(220 14% 10%));
  }

  /* Sidebar */
  .sidebar {
    width: 260px;
    border-right: 1px solid var(--system-color-border, hsla(0 0% 100% / 0.08));
    background: hsla(220 20% 8% / 0.95);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }

  .sidebar-header {
    padding: 1rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid hsla(0 0% 100% / 0.06);
  }

  .sidebar-header h3 {
    margin: 0;
    font-size: 0.9rem;
    color: var(--system-color-text, #e0e0e8);
  }

  .new-btn {
    padding: 0.35rem 0.7rem;
    border-radius: 8px;
    border: 1px solid hsla(0 0% 100% / 0.1);
    background: hsla(220 80% 55% / 0.2);
    color: hsl(220 80% 70%);
    font-size: 0.78rem;
    cursor: pointer;
  }

  .conv-list {
    overflow-y: auto;
    flex: 1;
    padding: 0.5rem;
  }

  .conv-item {
    display: block;
    width: 100%;
    text-align: left;
    padding: 0.65rem 0.75rem;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--system-color-text, #c0c0cc);
    cursor: pointer;
    margin-bottom: 2px;
  }

  .conv-item:hover {
    background: hsla(0 0% 100% / 0.05);
  }

  .conv-item.active {
    background: hsla(220 80% 55% / 0.15);
  }

  .conv-title {
    display: block;
    font-size: 0.82rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .conv-time {
    font-size: 0.7rem;
    color: var(--system-color-text-muted, hsla(0 0% 100% / 0.4));
  }

  .no-convs {
    text-align: center;
    color: var(--system-color-text-muted, hsla(0 0% 100% / 0.4));
    font-size: 0.82rem;
    padding: 2rem 1rem;
  }

  /* Main chat */
  .chat-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .chat-header {
    padding: 0.75rem 1rem;
    display: flex;
    align-items: center;
    gap: 0.75rem;
    border-bottom: 1px solid var(--system-color-border, hsla(0 0% 100% / 0.08));
    background: hsla(220 14% 12% / 0.9);
    backdrop-filter: blur(16px);
  }

  .menu-btn, .new-chat-btn {
    padding: 0.4rem 0.65rem;
    border-radius: 8px;
    border: 1px solid hsla(0 0% 100% / 0.08);
    background: transparent;
    color: var(--system-color-text, #c0c0cc);
    cursor: pointer;
    font-size: 0.85rem;
  }

  .menu-btn:hover, .new-chat-btn:hover {
    background: hsla(0 0% 100% / 0.06);
  }

  .header-info {
    flex: 1;
  }

  .header-info h2 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
  }

  .provider-badge {
    font-size: 0.7rem;
    color: hsl(160 60% 60%);
    background: hsla(160 60% 60% / 0.1);
    padding: 0.15rem 0.5rem;
    border-radius: 999px;
  }

  /* Messages */
  .messages {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
    scroll-behavior: smooth;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 0.5rem;
    color: var(--system-color-text-muted, hsla(0 0% 100% / 0.5));
  }

  .empty-icon {
    font-size: 3rem;
    margin-bottom: 0.5rem;
  }

  .empty-state h3 {
    margin: 0;
    color: var(--system-color-text, #e0e0e8);
    font-size: 1.2rem;
  }

  .empty-state p {
    margin: 0;
    text-align: center;
    font-size: 0.85rem;
    max-width: 360px;
  }

  .suggestions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-top: 1rem;
    justify-content: center;
  }

  .suggestion {
    padding: 0.5rem 1rem;
    border-radius: 999px;
    border: 1px solid hsla(0 0% 100% / 0.1);
    background: hsla(0 0% 100% / 0.04);
    color: var(--system-color-text, #c0c0cc);
    font-size: 0.8rem;
    cursor: pointer;
    transition: background 0.15s;
  }

  .suggestion:hover {
    background: hsla(220 80% 55% / 0.15);
    border-color: hsla(220 80% 55% / 0.3);
  }

  .msg {
    display: flex;
    gap: 0.75rem;
    margin-bottom: 1rem;
    align-items: flex-start;
  }

  .msg-avatar {
    width: 30px;
    height: 30px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1rem;
    background: hsla(0 0% 100% / 0.06);
    flex-shrink: 0;
  }

  .msg.user .msg-avatar {
    background: hsla(220 80% 55% / 0.2);
  }

  .msg.assistant .msg-avatar {
    background: hsla(160 60% 50% / 0.2);
  }

  .msg-role {
    font-size: 0.72rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--system-color-text-muted, hsla(0 0% 100% / 0.5));
    margin-bottom: 0.25rem;
  }

  .msg-text {
    font-size: 0.88rem;
    line-height: 1.55;
    color: var(--system-color-text, #e0e0e8);
    white-space: pre-wrap;
    word-break: break-word;
  }

  .system-msg .msg-text {
    color: hsl(0 70% 65%);
    font-style: italic;
  }

  /* Typing indicator */
  .typing {
    display: flex;
    gap: 4px;
    align-items: center;
    padding: 4px 0;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: hsla(0 0% 100% / 0.4);
    animation: blink 1.4s infinite;
  }

  .dot:nth-child(2) { animation-delay: 0.2s; }
  .dot:nth-child(3) { animation-delay: 0.4s; }

  @keyframes blink {
    0%, 100% { opacity: 0.3; }
    50% { opacity: 1; }
  }

  /* Input */
  .input-area {
    padding: 0.75rem 1rem;
    display: flex;
    gap: 0.5rem;
    align-items: flex-end;
    border-top: 1px solid var(--system-color-border, hsla(0 0% 100% / 0.08));
    background: hsla(220 14% 12% / 0.9);
  }

  .chat-input {
    flex: 1;
    padding: 0.7rem 1rem;
    border-radius: 12px;
    border: 1px solid hsla(0 0% 100% / 0.1);
    background: hsla(0 0% 100% / 0.05);
    color: var(--system-color-text, #e0e0e8);
    font-size: 0.88rem;
    font-family: inherit;
    resize: none;
    outline: none;
    line-height: 1.4;
  }

  .chat-input:focus {
    border-color: hsla(220 80% 55% / 0.5);
    box-shadow: 0 0 0 2px hsla(220 80% 55% / 0.15);
  }

  .chat-input::placeholder {
    color: hsla(0 0% 100% / 0.3);
  }

  .send-btn {
    width: 38px;
    height: 38px;
    border-radius: 10px;
    border: none;
    background: hsl(220 80% 55%);
    color: white;
    font-size: 1.1rem;
    font-weight: 700;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s;
    flex-shrink: 0;
  }

  .send-btn:hover:not(:disabled) {
    background: hsl(220 80% 60%);
  }

  .send-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
