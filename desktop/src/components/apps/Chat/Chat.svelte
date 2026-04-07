<svelte:options runes={true} />

<script lang="ts">
  import * as api from "$lib/api/backend";
  import type { ConversationSummary } from "$lib/api/types";

  type ChatMessage = {
    id: number;
    role: "user" | "assistant" | "system";
    content: string;
    timestamp: string;
    streaming?: boolean;
  };

  // ── Slash command definitions ──────────────────────────────────────────────
  const SLASH_COMMANDS = [
    { cmd: "/todo_add",     emoji: "✅", label: "Add a new todo task",          example: "/todo_add Buy groceries" },
    { cmd: "/todo_list",    emoji: "📋", label: "List all kanban tasks",        example: "/todo_list" },
    { cmd: "/todo_done",    emoji: "✔️", label: "Mark a task as done",          example: "/todo_done my-task.md" },
    { cmd: "/openviking",   emoji: "🕹️", label: "OpenViking server control",    example: "/openviking status" },
    { cmd: "/apps",         emoji: "🚀", label: "List or manage installed apps", example: "/apps" },
    { cmd: "/system",       emoji: "💻", label: "Show system info & resources",  example: "/system" },
    { cmd: "/files",        emoji: "📁", label: "List workspace files",         example: "/files" },
    { cmd: "/shell",        emoji: "🖥️", label: "Run a sandbox shell command",  example: "/shell ls -la" },
    { cmd: "/help",         emoji: "❓", label: "Show all available commands",   example: "/help" },
  ] as const;

  let messages = $state<ChatMessage[]>([]);
  let input = $state("");
  let loading = $state(false);
  let conversationId = $state<string | null>(null);
  let conversations = $state<ConversationSummary[]>([]);
  let sidebarOpen = $state(false);
  let agentInfo = $state<{ provider: string; model: string; name: string } | null>(null);
  let messagesEl: HTMLDivElement | undefined = $state();
  let msgCounter = $state(0);
  let streamingContent = $state("");
  let slashMenuOpen = $state(false);
  let slashMenuIndex = $state(0);
  let filteredCommands = $state(SLASH_COMMANDS.slice());


  // Load agent config on mount
  $effect(() => {
    api.agentConfig().then((config) => {
      agentInfo = { provider: config.provider, model: config.model, name: config.name };
    }).catch(() => {});

    api.agentConversations().then((convs) => {
      conversations = convs;
    }).catch(() => {});
  });

  // Auto-scroll when messages change or streaming content updates
  $effect(() => {
    const _len = messages.length;
    const _stream = streamingContent;
    if (messagesEl) {
      requestAnimationFrame(() => {
        messagesEl!.scrollTop = messagesEl!.scrollHeight;
      });
    }
  });

  /** Translate slash commands to full agent prompts (for LLM-dependent commands) */
  function resolveSlashCommand(text: string): string {
    if (text === "/openviking" || text.startsWith("/openviking ")) {
      const sub = text.slice("/openviking".length).trim() || "status";
      return `OpenViking action: ${sub}. Use the http_fetch tool to check http://127.0.0.1:3000/health for server status, or the shell_exec tool to check if the openviking-server process is running. Report the full status.`;
    }
    if (text === "/apps" || text.startsWith("/apps ")) {
      return `List all installed NDE-OS apps and their status. Use the app_list tool with include_catalog=true.`;
    }
    if (text === "/system") {
      return `Show NDE-OS system information — OS, memory, disk, GPU, and sandbox status. Use the system_info tool.`;
    }
    if (text === "/files" || text.startsWith("/files ")) {
      const path = text.slice("/files".length).trim() || ".";
      return `List the files in the sandbox workspace directory: "${path}". Use the file_list tool.`;
    }
    if (text.startsWith("/shell ")) {
      const cmd = text.slice("/shell ".length).trim();
      return `Run this shell command inside the NDE-OS sandbox: ${cmd}. Use the shell_exec tool. Show the full output.`;
    }
    if (text === "/help") {
      const cmdList = SLASH_COMMANDS.map(c => "  " + c.emoji + " " + c.cmd + " — " + c.label + " (example: " + c.example + ")").join("\n");
      return "The user asked for help. List all available slash commands:\n" + cmdList + "\n\nAlso mention the user can type any natural language question and the agent has 30+ built-in tools including file I/O, shell exec, web search, git, kanban, app management, and more.";
    }
    return text;
  }

  /** Try to handle a slash command directly via REST API (no LLM needed). Returns null if not a direct command. */
  async function tryDirectExecution(text: string): Promise<string | null> {
    // /todo_list — list all kanban tasks via REST
    if (text === "/todo_list" || text.startsWith("/todo_list ")) {
      try {
        const tasks = await api.kanbanGetTasks();
        if (tasks.length === 0) return "📋 **No tasks found.** Use `/todo_add <title>` to create one.";

        const statusEmoji: Record<string, string> = {
          "Plan": "🔴", "YOLO mode": "🟡", "Done by AI": "🟢",
          "Verified": "✅", "Re-open": "🔴", "Waiting Approval": "🟠",
        };
        const lines = tasks.map(t => {
          const emoji = statusEmoji[t.status] ?? "⚪";
          return `${emoji} **${t.title}** — \`${t.status}\` (\`${t.filename}\`)`;
        });
        return `📋 **Kanban Board** (${tasks.length} task${tasks.length > 1 ? "s" : ""})\n\n${lines.join("\n")}`;
      } catch (e: any) {
        return `❌ Failed to list tasks: ${e.message || e}`;
      }
    }

    // /todo_add <title> — create a new task
    if (text.startsWith("/todo_add ")) {
      const title = text.slice("/todo_add ".length).trim();
      if (!title) return "❌ Usage: `/todo_add <task title>`";
      try {
        const result = await api.kanbanCreateTask(title);
        return `✅ **Task created:** ${result.title}\n📄 File: \`${result.filename}\``;
      } catch (e: any) {
        return `❌ Failed to create task: ${e.message || e}`;
      }
    }

    // /todo_done <filename> — mark task as done
    if (text.startsWith("/todo_done ")) {
      const filename = text.slice("/todo_done ".length).trim();
      if (!filename) return "❌ Usage: `/todo_done <filename.md>`";
      const fname = filename.endsWith(".md") ? filename : filename + ".md";
      try {
        await api.kanbanUpdateTask(fname, "Done by AI");
        return `✔️ **Task marked as done:** \`${fname}\``;
      } catch (e: any) {
        return `❌ Failed to update task: ${e.message || e}`;
      }
    }

    return null; // Not a direct command — fall through to LLM
  }

  async function sendMessage(directText?: string) {
    let text = (directText ?? input).trim();
    if (!text || loading) return;

    // Close slash menu
    slashMenuOpen = false;

    msgCounter++;
    messages.push({
      id: msgCounter,
      role: "user",
      content: text,
      timestamp: new Date().toISOString(),
    });
    input = "";
    loading = true;
    streamingContent = "";

    // Try direct execution first (no LLM needed)
    const directResult = await tryDirectExecution(text);
    if (directResult !== null) {
      msgCounter++;
      messages.push({
        id: msgCounter,
        role: "assistant",
        content: directResult,
        timestamp: new Date().toISOString(),
      });
      loading = false;
      return;
    }

    // Resolve slash command to agent prompt (for LLM-dependent commands)
    const agentPrompt = resolveSlashCommand(text);

    // Add a placeholder assistant message for streaming
    msgCounter++;
    const assistantMsgId = msgCounter;
    messages.push({
      id: assistantMsgId,
      role: "assistant",
      content: "",
      timestamp: new Date().toISOString(),
      streaming: true,
    });

    try {
      // Try streaming first, fall back to regular — send agentPrompt, not raw text
      const success = await streamChat(agentPrompt, assistantMsgId);
      if (!success) {
        // Remove the streaming placeholder
        messages = messages.filter(m => m.id !== assistantMsgId);
        msgCounter--;
        // Fall back to non-streaming
        await regularChat(agentPrompt);
      }
    } catch (e: any) {
      // Update the streaming message with the error, or add error message
      const idx = messages.findIndex(m => m.id === assistantMsgId);
      if (idx >= 0) {
        messages[idx] = {
          ...messages[idx],
          role: "system",
          content: `Error: ${e.message || e}`,
          streaming: false,
        };
      } else {
        msgCounter++;
        messages.push({
          id: msgCounter,
          role: "system",
          content: `Error: ${e.message || e}`,
          timestamp: new Date().toISOString(),
        });
      }
    } finally {
      loading = false;
      streamingContent = "";

      // Refresh conversation list
      api.agentConversations().then((convs) => { conversations = convs; }).catch(() => {});
    }
  }

  /** Stream chat via SSE endpoint */
  async function streamChat(text: string, assistantMsgId: number): Promise<boolean> {
    try {
      const resp = await fetch("http://localhost:8080/api/agent/chat/stream", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          message: text,
          conversation_id: conversationId,
        }),
      });

      if (!resp.ok || !resp.body) return false;

      const contentType = resp.headers.get("content-type") ?? "";
      if (!contentType.includes("text/event-stream")) {
        // Not a stream response — try to read as JSON fallback
        const json = await resp.json();
        if (json.data?.response) {
          updateAssistantMessage(assistantMsgId, json.data.response);
          if (json.data?.conversation_id) conversationId = json.data.conversation_id;
          return true;
        }
        return false;
      }

      const reader = resp.body.getReader();
      const decoder = new TextDecoder();
      let accumulated = "";
      let buffer = "";

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split("\n");
        buffer = lines.pop() ?? ""; // Keep incomplete line in buffer

        for (const line of lines) {
          const trimmed = line.trim();
          if (!trimmed || trimmed.startsWith(":")) continue;
          if (trimmed === "data: [DONE]") continue;

          const dataPrefix = "data: ";
          if (!trimmed.startsWith(dataPrefix)) continue;

          const jsonStr = trimmed.slice(dataPrefix.length);
          try {
            const chunk = JSON.parse(jsonStr);

            if (chunk.type === "text_delta" && chunk.content) {
              accumulated += chunk.content;
              streamingContent = accumulated;
              updateAssistantMessage(assistantMsgId, accumulated);
            } else if (chunk.type === "done") {
              const finalContent = chunk.content || accumulated;
              updateAssistantMessage(assistantMsgId, finalContent, false);
              if (chunk.conversation_id) conversationId = chunk.conversation_id;
            } else if (chunk.type === "error") {
              updateAssistantMessage(assistantMsgId, `Error: ${chunk.message}`, false);
              const idx = messages.findIndex(m => m.id === assistantMsgId);
              if (idx >= 0) messages[idx].role = "system";
            }
          } catch {
            // Skip malformed JSON
          }
        }
      }

      // Finalize
      if (accumulated) {
        updateAssistantMessage(assistantMsgId, accumulated, false);
      }

      return true;
    } catch {
      return false;
    }
  }

  /** Regular non-streaming chat */
  async function regularChat(text: string) {
    const resp = await api.agentChat(text, conversationId ?? undefined);
    conversationId = resp.conversation_id;
    msgCounter++;
    messages.push({
      id: msgCounter,
      role: "assistant",
      content: resp.response,
      timestamp: new Date().toISOString(),
    });
  }

  function updateAssistantMessage(id: number, content: string, streaming = true) {
    const idx = messages.findIndex(m => m.id === id);
    if (idx >= 0) {
      messages[idx] = { ...messages[idx], content, streaming };
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
    // Slash command autocomplete navigation
    if (slashMenuOpen) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        slashMenuIndex = (slashMenuIndex + 1) % filteredCommands.length;
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        slashMenuIndex = (slashMenuIndex - 1 + filteredCommands.length) % filteredCommands.length;
        return;
      }
      if (e.key === "Tab" || (e.key === "Enter" && !e.shiftKey)) {
        e.preventDefault();
        if (filteredCommands.length > 0) {
          const selected = filteredCommands[slashMenuIndex];
          input = selected.cmd + " ";
          slashMenuOpen = false;
        }
        return;
      }
      if (e.key === "Escape") {
        slashMenuOpen = false;
        return;
      }
    }

    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  }

  function handleInput() {
    const val = input;
    if (val.startsWith("/") && !val.includes(" ")) {
      // Filter commands matching partial input
      const query = val.toLowerCase();
      filteredCommands = SLASH_COMMANDS.filter(c => c.cmd.startsWith(query));
      slashMenuOpen = filteredCommands.length > 0;
      slashMenuIndex = 0;
    } else {
      slashMenuOpen = false;
    }
  }

  function selectSlashCommand(cmd: string) {
    input = cmd + " ";
    slashMenuOpen = false;
  }

  /** Simple markdown-like rendering for assistant messages */
  function renderMarkdown(text: string): string {
    return text
      // Code blocks (triple backtick)
      .replace(/```(\w*)\n([\s\S]*?)```/g, '<pre class="code-block"><code>$2</code></pre>')
      // Inline code
      .replace(/`([^`]+)`/g, '<code class="inline-code">$1</code>')
      // Bold
      .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
      // Italic
      .replace(/\*(.+?)\*/g, '<em>$1</em>')
      // Line breaks
      .replace(/\n/g, '<br/>');
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
          <p>Ask anything, or type <code>/</code> for slash commands. I have 30+ tools for files, shell, apps, kanban, web, and more.</p>
          <div class="suggestions">
            <button class="suggestion" onclick={() => sendMessage("/todo_add Build a sample app")}>
              ✅ /todo_add
            </button>
            <button class="suggestion" onclick={() => sendMessage("/todo_list")}>
              📋 /todo_list
            </button>
            <button class="suggestion" onclick={() => sendMessage("/openviking status")}>
              🕹️ /openviking
            </button>
            <button class="suggestion" onclick={() => sendMessage("/apps")}>
              🚀 /apps
            </button>
            <button class="suggestion" onclick={() => sendMessage("/system")}>
              💻 /system
            </button>
            <button class="suggestion" onclick={() => sendMessage("/help")}>
              ❓ /help
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
              <div class="msg-role">
                {msg.role === "user" ? "You" : msg.role === "assistant" ? "Agent" : "System"}
                {#if msg.streaming}
                  <span class="streaming-badge">streaming</span>
                {/if}
              </div>
              {#if msg.role === "assistant" && msg.content}
                <div class="msg-text">{@html renderMarkdown(msg.content)}{#if msg.streaming}<span class="cursor-blink">▊</span>{/if}</div>
              {:else}
                <div class="msg-text">{msg.content}</div>
              {/if}
            </div>
          </div>
        {/each}

        {#if loading && !messages.some(m => m.streaming)}
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
      {#if slashMenuOpen}
        <div class="slash-menu">
          {#each filteredCommands as cmd, i (cmd.cmd)}
            <button
              class="slash-item"
              class:active={i === slashMenuIndex}
              onclick={() => selectSlashCommand(cmd.cmd)}
            >
              <span class="slash-emoji">{cmd.emoji}</span>
              <span class="slash-cmd">{cmd.cmd}</span>
              <span class="slash-desc">{cmd.label}</span>
            </button>
          {/each}
        </div>
      {/if}
      <textarea
        class="chat-input"
        placeholder="Type a message or / for commands..."
        bind:value={input}
        onkeydown={handleKeyDown}
        oninput={handleInput}
        rows="1"
        disabled={loading}
      ></textarea>
      <button class="send-btn" onclick={() => sendMessage()} disabled={!input.trim() || loading}>
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
    max-width: 400px;
    line-height: 1.5;
  }

  .suggestions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-top: 1rem;
    justify-content: center;
    max-width: 500px;
  }

  .suggestion {
    padding: 0.5rem 1rem;
    border-radius: 999px;
    border: 1px solid hsla(0 0% 100% / 0.1);
    background: hsla(0 0% 100% / 0.04);
    color: var(--system-color-text, #c0c0cc);
    font-size: 0.8rem;
    cursor: pointer;
    transition: all 0.15s;
  }

  .suggestion:hover {
    background: hsla(220 80% 55% / 0.15);
    border-color: hsla(220 80% 55% / 0.3);
    transform: translateY(-1px);
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
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .streaming-badge {
    font-size: 0.6rem;
    padding: 0.1rem 0.35rem;
    border-radius: 999px;
    background: hsla(160 60% 50% / 0.15);
    color: hsl(160 60% 60%);
    text-transform: lowercase;
    letter-spacing: 0;
    animation: pulse 1.5s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  .msg-text {
    font-size: 0.88rem;
    line-height: 1.55;
    color: var(--system-color-text, #e0e0e8);
    word-break: break-word;
  }

  .msg-text :global(.code-block) {
    background: hsla(220 20% 8% / 0.8);
    border: 1px solid hsla(0 0% 100% / 0.08);
    border-radius: 8px;
    padding: 0.75rem 1rem;
    overflow-x: auto;
    font-family: ui-monospace, 'SF Mono', monospace;
    font-size: 0.82rem;
    margin: 0.5rem 0;
  }

  .msg-text :global(.inline-code) {
    background: hsla(0 0% 100% / 0.08);
    padding: 0.12rem 0.4rem;
    border-radius: 4px;
    font-family: ui-monospace, 'SF Mono', monospace;
    font-size: 0.84rem;
  }

  .cursor-blink {
    animation: blink-cursor 0.7s step-end infinite;
    color: hsl(220 80% 60%);
  }

  @keyframes blink-cursor {
    0%, 100% { opacity: 1; }
    50% { opacity: 0; }
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
    animation: dot-blink 1.4s infinite;
  }

  .dot:nth-child(2) { animation-delay: 0.2s; }
  .dot:nth-child(3) { animation-delay: 0.4s; }

  @keyframes dot-blink {
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
    position: relative;
  }

  /* ── Slash command autocomplete ─────────────────────────────────────────── */
  .slash-menu {
    position: absolute;
    bottom: 100%;
    left: 1rem;
    right: 1rem;
    background: hsl(220 20% 14%);
    border: 1px solid hsla(0 0% 100% / 0.12);
    border-radius: 12px;
    box-shadow: 0 -8px 32px hsla(0 0% 0% / 0.5);
    padding: 0.35rem;
    display: flex;
    flex-direction: column;
    gap: 2px;
    z-index: 10;
    backdrop-filter: blur(20px);
    max-height: 280px;
    overflow-y: auto;
    animation: slash-appear 0.12s ease-out;
  }

  @keyframes slash-appear {
    from { opacity: 0; transform: translateY(6px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  .slash-item {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.55rem 0.75rem;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--system-color-text, #c0c0cc);
    cursor: pointer;
    text-align: left;
    width: 100%;
    transition: background 0.1s;
  }

  .slash-item:hover,
  .slash-item.active {
    background: hsla(220 80% 55% / 0.18);
  }

  .slash-emoji {
    font-size: 1rem;
    width: 24px;
    text-align: center;
    flex-shrink: 0;
  }

  .slash-cmd {
    font-size: 0.82rem;
    font-weight: 600;
    color: hsl(220 80% 72%);
    font-family: ui-monospace, 'SF Mono', monospace;
    min-width: 100px;
  }

  .slash-desc {
    font-size: 0.78rem;
    color: hsla(0 0% 100% / 0.45);
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

  /* Highlight slash commands in user messages */
  .msg.user .msg-text {
    font-family: inherit;
  }
</style>
