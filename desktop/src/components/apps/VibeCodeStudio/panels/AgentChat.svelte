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
    requestAnimationFrame(() => chatBottom?.scrollIntoView({ behavior: "smooth" }));
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
- nde_kanban_update_task: args: { "filename": string, "status": "Plan" | "Waiting Approval" | "YOLO mode" | "Done by AI" | "Verified by Human" | "Re-open" }
- nde_kanban_delete_task: args: { "filename": string }

Important Rules:
1. ONLY output the JSON block when using a tool. Do NOT wrap tool JSON payloads outside of the \`\`\`json block.
2. Only use ONE tool per response.
3. Once the tool returns a result, you will receive it as a System observation, and you can then provide your final answer to the user.`;

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
    { label: "/tickets-writer", description: "Enforces 4-phase methodology" },
    { label: "/figma-design-sync", description: "Design UI sync" },
    { label: "/research", description: "Web search" },
    { label: "/setup", description: "Configure codebase" }
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
    setTimeout(() => textareaRef?.focus(), 0);
  }

    // Removed the fast path regex parsing. We fully rely on the LLM tool-calling loop.

  async function send() {
    if (!prompt.trim() || isGenerating) return;
    const text = prompt.trim();
    prompt = "";
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
                } else if (data.type === "error") {
                  throw new Error(data.message);
                }
              } catch {}
            }
          }
        }
        
        // Check for tool calls
        messages[messages.length - 1].streaming = false;
        const jsonMatch = fullContent.match(/```(?:json)?\s*(\{[\s\S]*?"tool"[\s\S]*?\})\s*```/);
        
        if (jsonMatch && chatMode === "scrum") {
          let toolCall: any;
          try { toolCall = JSON.parse(jsonMatch[1]); } catch(e) {}
          if (toolCall && toolCall.tool) {
            messages.push({ role: "system", content: `🔧 Executing **${toolCall.tool}**...`, tool: toolCall.tool });
            scrollToBottom();
            
            let resultString = "";
            try {
              if (toolCall.tool === "nde_kanban_create_task") {
                 let r: any = await invoke("create_agent_task", { title: toolCall.args.title||"Task", description: toolCall.args.description||"", checklist: toolCall.args.checklist||[] });
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
                 resultString = `Error: Unknown tool ${toolCall.tool}. Available tools: nde_kanban_create_task, nde_kanban_update_task, nde_kanban_delete_task, nde_kanban_get_tasks`;
              }
            } catch (e: any) { resultString = `Error executing tool: ${e}`; }
            
            messages.push({ role: "system", content: `\`\`\`\n${resultString}\n\`\`\`` });
            
            historyContext += `\n\nAssistant generated tool call: ${fullContent}\n\nSystem observation:\n\`\`\`\n${resultString}\n\`\`\`\n\nProceed based on the observation. If done, provide a clear, concise summary.`;
            messages.push({ role: "assistant", content: "", streaming: true });
            scrollToBottom();
            continue;
          }
        }

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

<div class="flex-1 overflow-y-auto p-4 flex flex-col gap-4 text-sm scrollbar-thin">
  {#each messages as msg}
    <div class="flex gap-3 {msg.role === 'user' ? 'flex-row-reverse' : ''}">
      <div class="w-7 h-7 shrink-0 rounded-full flex items-center justify-center {msg.role === 'assistant' ? 'bg-indigo-500/20 text-indigo-400 border border-indigo-500/30' : msg.role === 'user' ? 'bg-white/10 text-white/70 border border-white/20' : 'bg-emerald-500/20 text-emerald-400 border border-emerald-500/30'}">
        {msg.role === 'assistant' ? '✦' : msg.role === 'user' ? 'U' : '🔧'}
      </div>
      <div class="max-w-[85%]">
        {#if msg.content}
          <div class="px-3 py-2 rounded-xl {msg.role === 'assistant' ? 'bg-white/5 border border-white/10 text-white/90 font-mono text-[11px] whitespace-pre-wrap max-h-48 overflow-y-auto' : msg.role === 'user' ? 'bg-indigo-500/30 border border-indigo-500/40 text-white whitespace-pre-wrap' : 'bg-emerald-500/10 border border-emerald-500/20 text-emerald-100 font-mono text-[10px] whitespace-pre-wrap'}">
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
