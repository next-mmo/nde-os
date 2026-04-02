<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { FDocument, FNode } from "$lib/figma-json/types";

  interface Props {
    document: FDocument;
    onApplyPatch?: (patch: any) => void;
    chatMode?: "figma" | "scrum" | "dev";
    activeFilePath?: string | null;
    fileContent?: string;
  }

  let { document, onApplyPatch, chatMode = "figma", activeFilePath = null, fileContent = "" }: Props = $props();

  let prompt = $state("");
  let isGenerating = $state(false);
  
  interface ChatMsg {
    role: "user" | "assistant" | "system";
    content: string;
    streaming?: boolean;
    error?: boolean;
    spec?: any;
    ticket?: { filename: string; title: string };
    tool?: string;
  }

  let messages = $state<ChatMsg[]>([
    {
      role: "assistant",
      content: "I'm your Vibe Studio agent. Tell me what UI to build, and I'll generate the HTML/Tailwind code to render it live."
    }
  ]);
  
  let chatBottom: HTMLDivElement | null = null;
  function scrollToBottom() {
    requestAnimationFrame(() => {
      const container = chatBottom?.parentElement;
      if (container) {
        container.scrollTop = container.scrollHeight;
      }
    });
  }

  const figmaPrompt = `You are the NDE Vibe Studio Agent (v0-style). You output ONLY valid HTML code describing a UI component.
You should use Tailwind CSS classes for styling. Do not use external CSS files.
Include responsive design if applicable. Keep the UI modern, sleek, and beautiful (similar to shadcn/ui).
Always wrap your final HTML in a markdown codeblock like \`\`\`html ... \`\`\`. Do not include explanations outside the codeblock, just the code.`;

  const scrumPrompt = `You are the Scrum Master Agent for NDE Vibe Code Studio. You manage the Kanban board, create tickets, and organize development tasks.

You have access to these Kanban tools. To use them, output a JSON codeblock containing a tool request EXACTLY like this:
\`\`\`json
{
  "tool": "nde_kanban_create_task",
  "args": { "title": "Setup Express Server", "description": "Configure express.js and basic routes", "checklist": ["Install dependencies", "Create index.js"] }
}
\`\`\`

Available Tools:
- nde_kanban_get_tasks: args: {}
- nde_kanban_create_task: args: { "title": string, "description": string, "checklist": string[] }
- nde_kanban_update_task: args: { "filename": string, "status": "Plan" | "YOLO mode" | "Done by AI" | "Verified by Human" | "Re-open" }
- nde_kanban_delete_task: args: { "filename": string }

When creating tasks, use the tickets-writer methodology with rich content:
1. Title and Purpose explaining WHAT and WHY
2. Description with technical context
3. Edge Cases & Security concerns  
4. Task Checklist with 3-8 specific steps
5. Definition of Done

Important Rules:
1. ONLY output the JSON block when using a tool. Do NOT wrap tool JSON payloads outside of the \`\`\`json block.
2. Only use ONE tool per response.
3. Once the tool returns a result, you will receive it as a System observation, and you can then provide your final answer.`;

  let devPrompt = $derived(`You are the OpenCode IDE Agent. You assist the user with writing and understanding code. You output explanations and code fragments. When providing code blocks, ensure they are properly formatted with the appropriate language.
${activeFilePath ? `\nActive File: ${activeFilePath}\nFile Content:\n\`\`\`\n${fileContent}\n\`\`\`` : ''}`);

  let activeSystemPrompt = $derived(
    chatMode === "figma" ? figmaPrompt : 
    chatMode === "scrum" ? scrumPrompt : devPrompt
  );

  // Autocomplete state
  let showAutocomplete = $state(false);
  let autocompletePrefix = $state("");
  let autocompleteIndex = $state(0);
  let mentionType = $state<"@" | "/">("@");
  let textareaRef: HTMLTextAreaElement | null = null;
  
  const skillsList = [
    { label: "/add", description: "Create task → /add My Task Title" },
    { label: "/list", description: "List all kanban tasks" },
    { label: "/move", description: "Move task → /move filename.md Done by AI" },
    { label: "/delete", description: "Delete task → /delete filename.md" },
    { label: "/tickets-writer", description: "Enforces 4-phase methodology" },
    { label: "/research", description: "Web search" },
  ];
  
  const mcpList = [
    { label: "@kanban", description: "Vibe Studio Kanban Tool" },
    { label: "@agent-native", description: "Native tools" }
  ];

  let filteredSuggestions = $derived(
    mentionType === "/" 
      ? skillsList.filter(s => s.label.toLowerCase().includes(autocompletePrefix.toLowerCase()))
      : mcpList.filter(s => s.label.toLowerCase().includes(autocompletePrefix.toLowerCase()))
  );

  function handleInput(e: Event) {
    const val = (e.target as HTMLTextAreaElement).value;
    const cursor = (e.target as HTMLTextAreaElement).selectionStart;
    
    const textBeforeCursor = val.slice(0, cursor);
    const match = textBeforeCursor.match(/([@/])([a-zA-Z0-9-]*)$/);
    
    if (match) {
      mentionType = match[1] as "@" | "/";
      autocompletePrefix = match[2];
      showAutocomplete = true;
      autocompleteIndex = 0;
    } else {
      showAutocomplete = false;
    }
  }

  function selectSuggestion(suggestion: any) {
    if (!textareaRef) return;
    const val = prompt;
    const cursor = textareaRef.selectionStart;
    const textBeforeCursor = val.slice(0, cursor);
    const textAfterCursor = val.slice(cursor);
    
    const replaced = textBeforeCursor.replace(/[@/][a-zA-Z0-9-]*$/, suggestion.label + " ");
    prompt = replaced + textAfterCursor;
    showAutocomplete = false;
    const newCursorPos = replaced.length;
    setTimeout(() => {
      if (textareaRef) {
        textareaRef.focus();
        textareaRef.selectionStart = newCursorPos;
        textareaRef.selectionEnd = newCursorPos;
      }
    }, 0);
  }

  // ── Direct slash-command executor ────────────────────────────────
  async function tryDirectCommand(text: string): Promise<boolean> {
    // /add <title> — LLM generates full markdown ticket, writes directly to file
    const addMatch = text.match(/^\/add\s+(.+)/i);
    if (addMatch) {
      const title = addMatch[1].trim();
      messages.push({ role: "user", content: text });
      messages.push({ role: "assistant", content: "", streaming: true });
      scrollToBottom();

      // Ask the LLM to generate a complete ticket (tickets-writer methodology)
      const enrichPrompt = `You are a Scrum Master using the tickets-writer methodology.
Generate a COMPLETE markdown task ticket for: "${title}"

Use this EXACT format — output ONLY the markdown, nothing else:

# ${title}

- **Status:** 🔴 \`plan\`
- **Feature:** ${title}
- **Purpose:** <1-2 sentence purpose explaining WHAT this achieves and WHY>

---

## Description

<2-4 sentences with technical context about the implementation approach>

---

## Edge Cases & Security

- <specific edge case or security concern>
- <another concern>

---

## Task Checklist

- [ ] <Specific actionable step 1>
- [ ] <Specific actionable step 2>
- [ ] <Specific actionable step 3>
- [ ] <...3-8 total steps>
- [ ] Write tests and verify

---

## Definition of Done

- [ ] All checklist items completed
- [ ] Code reviewed and tested
- [ ] No TODOs, no mocks, no hacks

Rules:
- Be technical and precise
- Each checklist step must be independently verifiable
- Include edge cases relevant to the specific task
- Output ONLY the markdown — no explanations before or after`;

      let ticketContent = "";

      try {
        const resp = await fetch("http://localhost:8080/api/agent/chat/stream", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ message: enrichPrompt }),
        });

        if (resp.ok) {
          const reader = resp.body?.getReader();
          const decoder = new TextDecoder();

          if (reader) {
            while (true) {
              const { done, value } = await reader.read();
              if (done) break;
              const chunk = decoder.decode(value, { stream: true });
              for (const line of chunk.split("\n")) {
                const trimmed = line.trim();
                if (!trimmed.startsWith("data: ") || trimmed === "data: [DONE]") continue;
                try {
                  const data = JSON.parse(trimmed.slice(6));
                  if (data.type === "text_delta" && data.content) {
                    ticketContent += data.content;
                    messages[messages.length - 1].content = `✨ Generating ticket...\n\n${ticketContent}`;
                    scrollToBottom();
                  }
                } catch {}
              }
            }
          }
        }
      } catch {
        // LLM unavailable — ticketContent stays empty, backend uses minimal fallback
      }

      // Create the task — pass full LLM markdown as content (Rust writes it directly)
      messages[messages.length - 1].streaming = false;
      try {
        const r: any = await invoke("create_agent_task", {
          title,
          description: "",
          checklist: [],
          content: ticketContent || null, // null → backend uses minimal fallback
        });
        messages[messages.length - 1].content = ticketContent
          ? `✅ Created **${title}** → \`${r.filename}\`\n\n${ticketContent}`
          : `✅ Created **${title}** → \`${r.filename}\``;
        messages[messages.length - 1].ticket = { filename: r.filename, title };
      } catch (e: any) {
        messages[messages.length - 1].content = `❌ Error: ${e}`;
        messages[messages.length - 1].error = true;
      }
      scrollToBottom();
      return true;
    }

    // /list
    if (/^\/list\s*$/i.test(text)) {
      messages.push({ role: "user", content: text });
      messages.push({ role: "system", content: "🔧 Listing tasks...", tool: "nde_kanban_get_tasks" });
      scrollToBottom();
      try {
        const tasks: any[] = await invoke("get_agent_tasks");
        if (tasks.length === 0) {
          messages.push({ role: "assistant", content: "📋 No tasks on the board." });
        } else {
          const lines = tasks.map(t => `• **${t.title}** — \`${t.status}\` → \`${t.filename}\``);
          messages.push({ role: "assistant", content: `📋 **${tasks.length} task(s):**\n${lines.join("\n")}` });
        }
      } catch (e: any) {
        messages.push({ role: "assistant", content: `❌ Error: ${e}`, error: true });
      }
      scrollToBottom();
      return true;
    }

    // /move <filename> <status>
    const moveMatch = text.match(/^\/move\s+(\S+\.md)\s+(.+)/i);
    if (moveMatch) {
      const filename = moveMatch[1].trim();
      const newStatus = moveMatch[2].trim();
      messages.push({ role: "user", content: text });
      messages.push({ role: "system", content: `🔧 Moving \`${filename}\` → **${newStatus}**...`, tool: "nde_kanban_update_task" });
      scrollToBottom();
      try {
        await invoke("update_agent_task_status", { filename, newStatus });
        messages.push({ role: "assistant", content: `✅ Moved \`${filename}\` to **${newStatus}**` });
      } catch (e: any) {
        messages.push({ role: "assistant", content: `❌ Error: ${e}`, error: true });
      }
      scrollToBottom();
      return true;
    }

    // /delete <filename>
    const delMatch = text.match(/^\/delete\s+(\S+\.md)/i);
    if (delMatch) {
      const filename = delMatch[1].trim();
      messages.push({ role: "user", content: text });
      messages.push({ role: "system", content: `🔧 Deleting \`${filename}\`...`, tool: "nde_kanban_delete_task" });
      scrollToBottom();
      try {
        await invoke("delete_agent_task", { filename });
        messages.push({ role: "assistant", content: `✅ Deleted \`${filename}\`` });
      } catch (e: any) {
        messages.push({ role: "assistant", content: `❌ Error: ${e}`, error: true });
      }
      scrollToBottom();
      return true;
    }

    return false;
  }

  async function send() {
    if (!prompt.trim() || isGenerating) return;
    const text = prompt.trim();
    prompt = "";

    // Try direct slash commands first (no LLM call needed)
    if (text.startsWith("/")) {
      const handled = await tryDirectCommand(text);
      if (handled) return;
    }

    isGenerating = true;

    messages.push({ role: "user", content: text });
    messages.push({ role: "assistant", content: "", streaming: true });
    scrollToBottom();

    let historyContext = activeSystemPrompt + "\n\nUser request: " + text;
    let turnCount = 0;

    try {
      while (turnCount < 5) {
        turnCount++;
        const resp = await fetch("http://localhost:8080/api/agent/chat/stream", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ message: historyContext }),
        });

        if (!resp.ok) throw new Error("Agent API Error");

        const reader = resp.body?.getReader();
        const decoder = new TextDecoder();
        let fullContent = "";

        if (reader) {
          while (true) {
            const { done, value } = await reader.read();
            if (done) break;
            const chunk = decoder.decode(value, { stream: true });
            const lines = chunk.split("\n");
            for (const line of lines) {
              const trimmed = line.trim();
              if (!trimmed || trimmed.startsWith(":") || trimmed === "data: [DONE]") continue;
              if (!trimmed.startsWith("data: ")) continue;

              try {
                const data = JSON.parse(trimmed.slice(6));
                if (data.type === "text_delta" && data.content) {
                  fullContent += data.content;
                  messages[messages.length - 1].content = fullContent;
                  scrollToBottom();
                } else if (data.type === "tool_call_start") {
                  // Native tool event from backend executor
                  messages[messages.length - 1].streaming = false;
                  messages.push({ role: "system", content: `🔧 Calling **${data.tool_name}**...`, tool: data.tool_name });
                  scrollToBottom();
                } else if (data.type === "tool_call_result") {
                  const preview = data.output?.length > 500 ? data.output.slice(0, 500) + "..." : data.output;
                  messages.push({
                    role: "system",
                    content: data.is_error
                      ? `❌ **${data.tool_name}** error:\n\`\`\`\n${preview}\n\`\`\``
                      : `✅ **${data.tool_name}** (${data.duration_ms}ms):\n\`\`\`\n${preview}\n\`\`\``,
                    tool: data.tool_name,
                  });
                  fullContent = "";
                  messages.push({ role: "assistant", content: "", streaming: true });
                  scrollToBottom();
                } else if (data.type === "done" && data.content) {
                  if (messages[messages.length - 1].role === "assistant" && !messages[messages.length - 1].content) {
                    messages[messages.length - 1].content = data.content;
                    fullContent = data.content;
                  }
                } else if (data.type === "error") {
                  throw new Error(data.message);
                }
              } catch {}
            }
          }
        }
        
        // Finalize current assistant message
        messages[messages.length - 1].streaming = false;

        // Frontend tool loop: parse JSON codeblocks for LLMs without native tool-calling
        const jsonMatch = fullContent.match(/```(?:json)?\s*(\{[\s\S]*?"tool"[\s\S]*?\})\s*```/);
        
        if (jsonMatch && chatMode === "scrum") {
          let toolCall: any;
          try { toolCall = JSON.parse(jsonMatch[1]); } catch(e) {}
          if (toolCall && toolCall.tool) {
            // Show collapsible tool step
            messages.push({ role: "system", content: `🔧 Calling **${toolCall.tool}**...`, tool: toolCall.tool });
            scrollToBottom();
            
            let resultString = "";
            try {
              if (toolCall.tool === "nde_kanban_create_task") {
                 let r: any = await invoke("create_agent_task", { title: toolCall.args.title||"Task", description: toolCall.args.description||"", checklist: toolCall.args.checklist||[], content: null });
                 resultString = `Success: Created ${r.filename}`;
              } else if (toolCall.tool === "nde_kanban_update_task") {
                 let stat = toolCall.args.status === "Verified" ? "Verified by Human" : toolCall.args.status;
                 await invoke("update_agent_task_status", { filename: toolCall.args.filename, newStatus: stat });
                 resultString = `Success: Updated ${toolCall.args.filename} to ${stat}`;
              } else if (toolCall.tool === "nde_kanban_delete_task") {
                 await invoke("delete_agent_task", { filename: toolCall.args.filename });
                 resultString = `Success: Deleted ${toolCall.args.filename}`;
              } else if (toolCall.tool === "nde_kanban_get_tasks") {
                 const tasks = await invoke("get_agent_tasks");
                 resultString = JSON.stringify(tasks, null, 2);
              } else {
                 resultString = `Unknown tool: ${toolCall.tool}`;
              }
            } catch (e: any) { resultString = `Error: ${e}`; }
            
            // Show result as collapsible step
            messages.push({
              role: "system",
              content: resultString.startsWith("Error") ? `❌ **${toolCall.tool}**:\n\`\`\`\n${resultString}\n\`\`\`` : `✅ **${toolCall.tool}**:\n\`\`\`\n${resultString}\n\`\`\``,
              tool: toolCall.tool,
            });
            
            historyContext += `\n\nAssistant: ${fullContent}\n\nSystem observation:\n\`\`\`\n${resultString}\n\`\`\`\n\nBased on the result above, provide a clear summary to the user.`;
            messages.push({ role: "assistant", content: "", streaming: true });
            scrollToBottom();
            continue;
          }
        }

        // Check for HTML (figma mode)
        const htmlMatch = fullContent.match(/```(?:html|svelte|vue|jsx|tsx)?\s*([\s\S]*?)```/);
        if (htmlMatch && onApplyPatch) {
          messages[messages.length - 1].spec = { code: htmlMatch[1] };
          onApplyPatch({ code: htmlMatch[1] });
        }
        break;
      }
      
    } catch (e: any) {
      messages[messages.length - 1].content = e.message;
      messages[messages.length - 1].error = true;
      messages[messages.length - 1].streaming = false;
    } finally {
      isGenerating = false;
      scrollToBottom();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (showAutocomplete && filteredSuggestions.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        autocompleteIndex = (autocompleteIndex + 1) % filteredSuggestions.length;
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        autocompleteIndex = (autocompleteIndex - 1 + filteredSuggestions.length) % filteredSuggestions.length;
        return;
      }
      if (e.key === "Enter" || e.key === "Tab") {
        e.preventDefault();
        selectSuggestion(filteredSuggestions[autocompleteIndex]);
        return;
      }
      if (e.key === "Escape") {
        showAutocomplete = false;
        return;
      }
    }

    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }
</script>

<div class="flex-1 overflow-y-auto p-4 flex flex-col gap-3 text-sm scrollbar-thin">
  {#each messages as msg, idx}
    {#if msg.role === 'system'}
      <!-- Tool/system messages: collapsible process step -->
      <details class="group tool-step">
        <summary class="flex items-center gap-2 cursor-pointer select-none px-2 py-1.5 rounded-lg bg-white/3 hover:bg-white/6 border border-white/6 transition-colors text-xs text-white/60">
          <svg class="w-3.5 h-3.5 shrink-0 transition-transform duration-200 group-open:rotate-90 text-white/40" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5"><path stroke-linecap="round" stroke-linejoin="round" d="M9 5l7 7-7 7"/></svg>
          {#if msg.tool}
            <span class="inline-flex items-center gap-1.5">
              {#if msg.content?.startsWith('✅')}
                <span class="w-4 h-4 rounded-full bg-emerald-500/20 flex items-center justify-center text-[10px] text-emerald-400">✓</span>
                <span class="text-emerald-400/80">{msg.tool}</span>
                <span class="text-white/30">completed</span>
              {:else if msg.content?.startsWith('❌')}
                <span class="w-4 h-4 rounded-full bg-red-500/20 flex items-center justify-center text-[10px] text-red-400">✗</span>
                <span class="text-red-400/80">{msg.tool}</span>
                <span class="text-white/30">failed</span>
              {:else}
                <span class="w-4 h-4 rounded-full bg-amber-500/20 flex items-center justify-center">
                  <span class="w-1.5 h-1.5 rounded-full bg-amber-400 animate-pulse"></span>
                </span>
                <span class="text-amber-300/80">{msg.tool}</span>
                <span class="text-white/30">running...</span>
              {/if}
            </span>
          {:else}
            <span class="text-white/50">{msg.content?.split('\n')[0]?.slice(0, 60) || 'Process step'}</span>
          {/if}
        </summary>
        {#if msg.content}
          <div class="mt-1.5 ml-6 px-3 py-2 rounded-lg bg-black/30 border border-white/6 text-[10px] text-white/60 font-mono whitespace-pre-wrap max-h-40 overflow-y-auto scrollbar-thin">
            {msg.content}
          </div>
        {/if}
      </details>
    {:else}
      <!-- User / Assistant messages: regular bubbles -->
      <div class="flex gap-3 {msg.role === 'user' ? 'flex-row-reverse' : ''}">
        <div class="w-7 h-7 shrink-0 rounded-full flex items-center justify-center {msg.role === 'assistant' ? 'bg-indigo-500/20 text-indigo-400 border border-indigo-500/30' : 'bg-white/10 text-white/70 border border-white/20'}">
          {msg.role === 'assistant' ? '✦' : 'U'}
        </div>
        <div class="max-w-[85%]">
          {#if msg.content}
            <div class="px-3 py-2 rounded-xl {msg.role === 'assistant' ? 'bg-white/5 border border-white/10 text-white/90 font-mono text-[11px] whitespace-pre-wrap max-h-48 overflow-y-auto' : 'bg-indigo-500/30 border border-indigo-500/40 text-white whitespace-pre-wrap'}">
              {msg.content}
            </div>
          {/if}
          {#if msg.streaming && !msg.content}
            <div class="px-3 py-2 rounded-xl bg-white/5 border border-white/10 text-white/50 flex gap-1 items-center">
              <div class="w-1.5 h-1.5 rounded-full bg-white/50 animate-pulse"></div>
              <div class="w-1.5 h-1.5 rounded-full bg-white/50 animate-pulse" style="animation-delay: 150ms"></div>
              <div class="w-1.5 h-1.5 rounded-full bg-white/50 animate-pulse" style="animation-delay: 300ms"></div>
            </div>
          {/if}
          {#if msg.ticket}
            <div class="mt-1 flex items-center gap-1.5 text-[10px] text-emerald-400 bg-emerald-500/10 border border-emerald-500/20 rounded px-2 py-1 w-fit">
              <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 24 24"><path d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm-7 14l-5-5 1.41-1.41L12 14.17l7.59-7.59L21 8l-9 9z"/></svg>
              Ticket created on Kanban
            </div>
          {/if}
          {#if msg.spec}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="mt-1 flex items-center gap-1.5 text-[10px] text-emerald-400 bg-emerald-500/10 border border-emerald-500/20 rounded px-2 py-1 w-fit cursor-pointer hover:bg-emerald-500/20 transition-colors" onclick={() => onApplyPatch && onApplyPatch(msg.spec)}>
              <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 24 24"><path d="M9 16h6l1-4H8l1 4zm1-8h4v2h-4V8z"/></svg>
              UI generated · Click to apply
            </div>
          {/if}
        </div>
      </div>
    {/if}
  {/each}
  <div bind:this={chatBottom}></div>
</div>

<div class="p-3 border-t border-white/10 bg-black/40 shrink-0">
  <div class="relative flex gap-2">
    <!-- Auto Complete Popup -->
    {#if showAutocomplete && filteredSuggestions.length > 0}
      <div class="absolute bottom-full left-0 mb-2 w-64 max-h-48 overflow-y-auto bg-neutral-900 border border-white/20 rounded-lg shadow-2xl z-50 p-1">
        {#each filteredSuggestions as suggestion, i}
          <button 
            class="w-full text-left px-3 py-2 text-xs rounded hover:bg-white/10 {i === autocompleteIndex ? 'bg-indigo-500/20 text-white' : 'text-white/80'}"
            onclick={() => selectSuggestion(suggestion)}
          >
            <div class="font-medium">{suggestion.label}</div>
            {#if suggestion.description}
               <div class="text-[10px] text-white/50">{suggestion.description}</div>
            {/if}
          </button>
        {/each}
      </div>
    {/if}

    <textarea 
      bind:this={textareaRef}
      bind:value={prompt}
      onkeydown={handleKeydown}
      oninput={handleInput}
      disabled={isGenerating}
      rows="1"
      placeholder={chatMode === 'figma' ? "Ask AI to styling this... (Enter to send)" : chatMode === 'scrum' ? "Manage board tasks... (Type @ or / to search)" : "Ask about your code... (Enter to send)"}
      class="flex-1 bg-white/5 border border-white/10 rounded-lg py-2 pl-3 pr-10 text-sm text-white placeholder-white/30 focus:outline-none focus:ring-1 focus:ring-indigo-500/50 resize-none overflow-hidden"
    ></textarea>
    <button 
      onclick={send}
      disabled={isGenerating || !prompt.trim()}
      aria-label="Send Message" 
      class="absolute right-2 top-1.5 p-1 rounded hover:bg-white/10 text-white/60 disabled:opacity-30 disabled:hover:bg-transparent transition-colors"
    >
      <svg class="w-5 h-5" aria-hidden="true" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8"></path></svg>
    </button>
  </div>
</div>
