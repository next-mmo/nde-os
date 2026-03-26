<script lang="ts">
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
    role: "user" | "assistant";
    content: string;
    streaming?: boolean;
    error?: boolean;
    spec?: any;
  }

  let messages = $state<ChatMsg[]>([
    {
      role: "assistant",
      content: "I'm your Vibe Studio agent. Tell me what UI to build, and I'll generate the Figma-like JSON nodes directly onto your canvas."
    }
  ]);
  
  let chatBottom: HTMLDivElement | null = null;
  function scrollToBottom() {
    requestAnimationFrame(() => chatBottom?.scrollIntoView({ behavior: "smooth" }));
  }

  const figmaPrompt = `You are the NDE Vibe Studio Agent. You output ONLY valid JSON describing changes to a Figma-like document. 
The canvas document uses the following schema:
- type: FRAME | TEXT | RECTANGLE
- x, y, width, height: numbers
- fills: [{type: "SOLID", color: {r:1,g:0,b:0,a:1}}]
Always wrap your JSON in a markdown codeblock. If creating new nodes, provide them as a JSON object, e.g.
{
  "op": "append",
  "nodes": [
    { "id": "uuid1", "type": "RECTANGLE", "x": 100, "y": 100, "width": 200, "height": 50, "fills": [{"type":"SOLID","color":{"r":0.2,"g":0.5,"b":0.9,"a":1}}] }
  ]
}`;

  const scrumPrompt = `You are the Scrum Master Agent. You strictly manage project tasks, organize tickets, and answer questions about the development process. You do not generate UI JSON. Never output JSON payloads.`;

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

  async function send() {
    if (!prompt.trim() || isGenerating) return;
    const text = prompt.trim();
    prompt = "";
    isGenerating = true;

    messages.push({ role: "user", content: text });
    const idx = messages.length;
    messages.push({ role: "assistant", content: "", streaming: true });
    scrollToBottom();

    try {
      const resp = await fetch("http://localhost:8080/api/agent/chat/stream", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          message: activeSystemPrompt + "\n\nUser request: " + text,
        }),
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
            if (!trimmed || trimmed.startsWith(":")) continue;
            if (trimmed === "data: [DONE]") continue;
            if (!trimmed.startsWith("data: ")) continue;

            try {
              const data = JSON.parse(trimmed.slice(6));
              if (data.type === "text_delta" && data.content) {
                fullContent += data.content;
                messages[idx].content = fullContent;
                scrollToBottom();
              } else if (data.type === "done") {
                fullContent = data.content || fullContent;
              } else if (data.type === "error") {
                throw new Error(data.message);
              }
            } catch {}
          }
        }
      }
      
      // Attempt to parse final codeblock
      messages[idx].streaming = false;
      const jsonMatch = fullContent.match(/```(?:json)?\s*([\s\S]*?)```/);
      if (jsonMatch && onApplyPatch) {
        try {
          const parsed = JSON.parse(jsonMatch[1]);
          messages[idx].spec = parsed;
          onApplyPatch(parsed);
        } catch {}
      }
      
    } catch (e: any) {
      messages[idx].content = e.message;
      messages[idx].error = true;
      messages[idx].streaming = false;
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
