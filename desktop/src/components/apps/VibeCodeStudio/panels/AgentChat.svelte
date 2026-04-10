<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { marked } from "marked";
  import type { FDocument, FNode } from "$lib/figma-json/types";
  import { systemPrompt as renderJsonPrompt } from "$lib/json-render";

  // shadcn-svelte components
  import { Button } from "$lib/components/ui/button";
  import { Badge } from "$lib/components/ui/badge";
  import { ScrollArea } from "$lib/components/ui/scroll-area";
  import { Separator } from "$lib/components/ui/separator";
  import * as Avatar from "$lib/components/ui/avatar";
  import * as Collapsible from "$lib/components/ui/collapsible";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
  import * as Tooltip from "$lib/components/ui/tooltip";

  // Lucide icons
  import {
    Send,
    Square,
    Paperclip,
    Sparkles,
    ChevronDown,
    ChevronRight,
    Check,
    X,
    Bot,
    User,
    Wrench,
    CircleAlert,
    CircleCheck,
    Loader2,
    FileCode2,
    Copy,
    Play,
  } from "@lucide/svelte";

  interface ContextFile {
    path: string;
    name: string;
    content?: string;
  }

  interface Props {
    document: FDocument;
    onApplyPatch?: (patch: any) => void;
    chatMode?: "figma" | "scrum" | "dev";
    activeFilePath?: string | null;
    fileContent?: string;
    contextFiles?: ContextFile[];
    onAddContextFile?: (path: string, name: string) => void;
  }

  let { document, onApplyPatch, chatMode = "figma", activeFilePath = null, fileContent = "", contextFiles = $bindable([]), onAddContextFile }: Props = $props();

  let prompt = $state("");
  let isGenerating = $state(false);

  // ── Per-chat model override (dynamic from active providers) ────────
  interface ModelOption {
    id: string;
    label: string;
    provider: string;
    model: string;
    isActive: boolean;
  }

  let availableModels = $state<ModelOption[]>([]);
  let selectedModelId = $state("global");
  let currentGlobalModel = $state("Agent");

  // Fetch active LLM providers on mount
  $effect(() => {
    Promise.all([
      fetch("http://localhost:8080/api/models").then(r => r.json()).catch(() => null),
      fetch("http://localhost:8080/api/agent/config").then(r => r.json()).catch(() => null),
    ]).then(([modelsResp, configResp]) => {
      if (configResp?.data?.model) {
        currentGlobalModel = configResp.data.model;
      }
      if (modelsResp?.data && Array.isArray(modelsResp.data)) {
        availableModels = (modelsResp.data as any[]).map((p: any) => ({
          id: p.name || p.model,
          label: p.name || p.model,
          provider: p.provider_type || p.provider || "",
          model: p.model || "",
          isActive: p.is_active ?? false,
        }));
      }
    });
  });

  let activeModelLabel = $derived(
    selectedModelId === "global"
      ? currentGlobalModel
      : availableModels.find(m => m.id === selectedModelId)?.label ?? "Agent"
  );

  interface ChatMsg {
    role: "user" | "assistant" | "system";
    content: string;
    streaming?: boolean;
    error?: boolean;
    spec?: any;
    ticket?: { filename: string; title: string };
    tool?: string;
  }

  const WELCOME_MSG: ChatMsg = {
    role: "assistant",
    content: "Hello! I'm your **Vibe Studio** agent. Tell me what to build and I'll generate live HTML/Tailwind code for you.\n\nTry commands like `/add`, `/list`, or just describe what you want."
  };

  let messages = $state<ChatMsg[]>([WELCOME_MSG]);
  let msgCounter = $state(0);

  // ── Persistence ────────────────────────────────────────────────
  let hydrated = false;
  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  async function loadHistory() {
    try {
      const history = await invoke<{
        messages: { id: number; role: string; content: string; timestamp: string }[];
        conversation_id: string | null;
        msg_counter: number;
      }>("load_vibe_chat_history");

      if (history.messages && history.messages.length > 0) {
        messages = history.messages.map(m => ({
          role: m.role as ChatMsg["role"],
          content: m.content,
          streaming: false,
        }));
        msgCounter = history.msg_counter ?? 0;
      }
    } catch {}
    hydrated = true;
  }

  function saveHistory() {
    if (!hydrated) return;
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      const toSave = messages
        .filter(m => !m.streaming && m.content)
        .map((m, i) => ({
          id: i,
          role: m.role,
          content: m.content,
          timestamp: new Date().toISOString(),
        }));
      invoke("save_vibe_chat_history", {
        history: {
          messages: toSave,
          conversation_id: null,
          msg_counter: msgCounter,
        },
      }).catch(() => {});
    }, 500);
  }

  // Load on mount + listen for workspace switches
  $effect(() => {
    loadHistory();
    const unlisten = listen("workspace://changed", () => {
      hydrated = false;
      messages = [WELCOME_MSG];
      msgCounter = 0;
      loadHistory();
    });
    return () => { unlisten.then(fn => fn()); };
  });

  // Auto-save when messages change (debounced)
  $effect(() => {
    const _trigger = messages.length;
    saveHistory();
  });

  let scrollContainer: HTMLDivElement | null = null;
  function scrollToBottom() {
    requestAnimationFrame(() => {
      if (scrollContainer) {
        scrollContainer.scrollTop = scrollContainer.scrollHeight;
      }
    });
  }

  // Markdown renderer setup (sanitized, no raw HTML)
  marked.setOptions({ breaks: true, gfm: true });

  function renderMarkdown(text: string): string {
    return marked.parse(text, { async: false }) as string;
  }

  const figmaPrompt = `You are the NDE Vibe Studio Agent. You output ONLY valid JSON using the provided UI component catalog.

${renderJsonPrompt}

Always wrap your final JSON in a markdown codeblock like \`\`\`json ... \`\`\`. Do not include explanations outside the codeblock, just the JSON.`;

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
    { label: "/add", description: "Create task", icon: "plus" },
    { label: "/list", description: "List all kanban tasks", icon: "list" },
    { label: "/move", description: "Move task status", icon: "move" },
    { label: "/delete", description: "Delete task", icon: "trash" },
    { label: "/tickets-writer", description: "4-phase methodology", icon: "pen" },
    { label: "/research", description: "Web search", icon: "search" },
  ];

  const mcpList = [
    { label: "@kanban", description: "Vibe Studio Kanban Tool", icon: "board" },
    { label: "@agent-native", description: "Native tools", icon: "cpu" },
    { label: "@plan", description: "List tasks in Plan", icon: "circle-red" },
    { label: "@waiting", description: "List tasks in Waiting", icon: "clock" },
    { label: "@yolo", description: "List tasks in YOLO mode", icon: "circle-yellow" },
    { label: "@done", description: "List tasks in Done by AI", icon: "circle-green" },
    { label: "@verified", description: "List tasks in Verified", icon: "check" },
    { label: "@reopen", description: "List tasks in Re-open", icon: "refresh" },
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
    const addMatch = text.match(/^\/add\s+(.+)/i);
    if (addMatch) {
      const title = addMatch[1].trim();
      messages.push({ role: "user", content: text });
      messages.push({ role: "assistant", content: "", streaming: true });
      scrollToBottom();

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
                    messages[messages.length - 1].content = ticketContent;
                    scrollToBottom();
                  }
                } catch {}
              }
            }
          }
        }
      } catch {}

      messages[messages.length - 1].streaming = false;
      try {
        const r: any = await invoke("create_agent_task", {
          title,
          description: "",
          checklist: [],
          content: ticketContent || null,
        });
        messages[messages.length - 1].content = ticketContent
          ? `Created **${title}** → \`${r.filename}\`\n\n${ticketContent}`
          : `Created **${title}** → \`${r.filename}\``;
        messages[messages.length - 1].ticket = { filename: r.filename, title };
      } catch (e: any) {
        messages[messages.length - 1].content = `Error: ${e}`;
        messages[messages.length - 1].error = true;
      }
      scrollToBottom();
      return true;
    }

    if (/^\/list\s*$/i.test(text)) {
      messages.push({ role: "user", content: text });
      messages.push({ role: "system", content: "Listing tasks...", tool: "nde_kanban_get_tasks" });
      scrollToBottom();
      try {
        const tasks: any[] = await invoke("get_agent_tasks");
        if (tasks.length === 0) {
          messages.push({ role: "assistant", content: "No tasks on the board yet." });
        } else {
          const lines = tasks.map(t => `- **${t.title}** — \`${t.status}\` → \`${t.filename}\``);
          messages.push({ role: "assistant", content: `**${tasks.length} task(s):**\n${lines.join("\n")}` });
        }
      } catch (e: any) {
        messages.push({ role: "assistant", content: `Error: ${e}`, error: true });
      }
      scrollToBottom();
      return true;
    }

    const moveMatch = text.match(/^\/move\s+(\S+\.md)\s+(.+)/i);
    if (moveMatch) {
      const filename = moveMatch[1].trim();
      const newStatus = moveMatch[2].trim();
      messages.push({ role: "user", content: text });
      messages.push({ role: "system", content: `Moving \`${filename}\` → **${newStatus}**...`, tool: "nde_kanban_update_task" });
      scrollToBottom();
      try {
        await invoke("update_agent_task_status", { filename, newStatus });
        messages.push({ role: "assistant", content: `Moved \`${filename}\` to **${newStatus}**` });
      } catch (e: any) {
        messages.push({ role: "assistant", content: `Error: ${e}`, error: true });
      }
      scrollToBottom();
      return true;
    }

    const delMatch = text.match(/^\/delete\s+(\S+\.md)/i);
    if (delMatch) {
      const filename = delMatch[1].trim();
      messages.push({ role: "user", content: text });
      messages.push({ role: "system", content: `Deleting \`${filename}\`...`, tool: "nde_kanban_delete_task" });
      scrollToBottom();
      try {
        await invoke("delete_agent_task", { filename });
        messages.push({ role: "assistant", content: `Deleted \`${filename}\`` });
      } catch (e: any) {
        messages.push({ role: "assistant", content: `Error: ${e}`, error: true });
      }
      scrollToBottom();
      return true;
    }

    const statusMap: Record<string, string> = {
      "@plan": "Plan",
      "@waiting": "Waiting Approval",
      "@yolo": "YOLO mode",
      "@done": "Done by AI",
      "@verified": "Verified by Human",
      "@reopen": "Re-open",
    };
    const statusTag = text.trim().toLowerCase();
    if (statusMap[statusTag]) {
      const statusLabel = statusMap[statusTag];
      messages.push({ role: "user", content: text });
      messages.push({ role: "system", content: `Filtering tasks → **${statusLabel}**...`, tool: "nde_kanban_get_tasks" });
      scrollToBottom();
      try {
        const tasks: any[] = await invoke("get_agent_tasks");
        const filtered = tasks.filter((t: any) => t.status === statusLabel);
        if (filtered.length === 0) {
          messages.push({ role: "assistant", content: `No tasks in **${statusLabel}**.` });
        } else {
          const lines = filtered.map((t: any) => `- **${t.title}** (NDE-${t.id}) → \`${t.filename}\``);
          messages.push({ role: "assistant", content: `**${statusLabel}** — ${filtered.length} task(s):\n${lines.join("\n")}` });
        }
      } catch (e: any) {
        messages.push({ role: "assistant", content: `Error: ${e}`, error: true });
      }
      scrollToBottom();
      return true;
    }

    return false;
  }

  // ── Drag-and-drop onto chat panel ────────────────────────────────
  let isDragOver = $state(false);

  function handleDragOver(e: DragEvent) {
    if (e.dataTransfer?.types.includes("application/nde-file-path")) {
      e.preventDefault();
      isDragOver = true;
    }
  }

  function handleDragLeave() {
    isDragOver = false;
  }

  async function handleDrop(e: DragEvent) {
    e.preventDefault();
    isDragOver = false;
    const path = e.dataTransfer?.getData("application/nde-file-path");
    const name = e.dataTransfer?.getData("application/nde-file-name");
    if (path && name) {
      addContextFile(path, name);
    }
  }

  async function addContextFile(path: string, name: string) {
    if (contextFiles.some(f => f.path === path)) return;
    let content = "";
    try {
      content = await invoke<string>("read_file_content", { path });
    } catch {}
    contextFiles = [...contextFiles, { path, name, content }];
  }

  function removeContextFile(path: string) {
    contextFiles = contextFiles.filter(f => f.path !== path);
  }

  $effect(() => {
    if (onAddContextFile) {
      // noop — parent uses addContextFile via contextFiles prop binding
    }
  });

  // Clipboard copy helper
  let copiedId = $state<number | null>(null);
  function copyToClipboard(text: string, idx: number) {
    navigator.clipboard.writeText(text);
    copiedId = idx;
    setTimeout(() => { copiedId = null; }, 2000);
  }

  async function send() {
    if (!prompt.trim() || isGenerating) return;
    const text = prompt.trim();
    prompt = "";

    if (text.startsWith("/") || text.startsWith("@")) {
      const handled = await tryDirectCommand(text);
      if (handled) return;
    }

    isGenerating = true;

    let contextBlock = "";
    if (contextFiles.length > 0) {
      const fileSnippets = contextFiles.map(f =>
        `### File: ${f.name} (${f.path})\n\`\`\`\n${(f.content || "").slice(0, 8000)}\n\`\`\``
      ).join("\n\n");
      contextBlock = `\n\n--- Attached File Context ---\n${fileSnippets}\n--- End Context ---`;
    }

    messages.push({ role: "user", content: text + (contextFiles.length > 0 ? `\n\n${contextFiles.map(f => f.name).join(", ")}` : "") });
    messages.push({ role: "assistant", content: "", streaming: true });
    scrollToBottom();

    let historyContext = activeSystemPrompt + contextBlock + "\n\nUser request: " + text;
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
                  messages[messages.length - 1].streaming = false;
                  messages.push({ role: "system", content: `Calling **${data.tool_name}**...`, tool: data.tool_name });
                  scrollToBottom();
                } else if (data.type === "tool_call_result") {
                  const preview = data.output?.length > 500 ? data.output.slice(0, 500) + "..." : data.output;
                  messages.push({
                    role: "system",
                    content: data.is_error
                      ? `**${data.tool_name}** error:\n\`\`\`\n${preview}\n\`\`\``
                      : `**${data.tool_name}** (${data.duration_ms}ms):\n\`\`\`\n${preview}\n\`\`\``,
                    tool: data.tool_name,
                    error: data.is_error,
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

        messages[messages.length - 1].streaming = false;

        const jsonMatch = fullContent.match(/```(?:json)?\s*(\{[\s\S]*?"tool"[\s\S]*?\})\s*```/);

        if (jsonMatch && chatMode === "scrum") {
          let toolCall: any;
          try { toolCall = JSON.parse(jsonMatch[1]); } catch(e) {}
          if (toolCall && toolCall.tool) {
            messages.push({ role: "system", content: `Calling **${toolCall.tool}**...`, tool: toolCall.tool });
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

            messages.push({
              role: "system",
              content: resultString.startsWith("Error") ? `**${toolCall.tool}**:\n\`\`\`\n${resultString}\n\`\`\`` : `**${toolCall.tool}**:\n\`\`\`\n${resultString}\n\`\`\``,
              tool: toolCall.tool,
              error: resultString.startsWith("Error"),
            });

            historyContext += `\n\nAssistant: ${fullContent}\n\nSystem observation:\n\`\`\`\n${resultString}\n\`\`\`\n\nBased on the result above, provide a clear summary to the user.`;
            messages.push({ role: "assistant", content: "", streaming: true });
            scrollToBottom();
            continue;
          }
        }

        // Also parse UI from any block (json or html) if we are in figma mode
        const uiMatch = fullContent.match(/```(?:json|html|svelte|vue|jsx|tsx)?\s*([\s\S]*?)```/);
        if (uiMatch && onApplyPatch) {
          // If in scrum mode, ensure it's not a tool call before treating it as UI
          if (chatMode !== 'scrum' || !uiMatch[1].includes('"tool"')) {
            messages[messages.length - 1].spec = { code: uiMatch[1] };
            onApplyPatch({ code: uiMatch[1] });
          }
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

  // Tool step state tracking for collapsibles
  let openToolSteps = $state<Record<number, boolean>>({});
</script>

<!-- Main chat container -->
<div class="flex h-full flex-col bg-background/50">

  <!-- Messages scroll area -->
  <div bind:this={scrollContainer} class="flex-1 overflow-y-auto">
    <div class="flex flex-col gap-1 px-3 py-4">
      {#each messages as msg, idx}

        <!-- ─── Tool / System step ─── -->
        {#if msg.role === "system"}
          <div class="mx-1 my-0.5">
            <Collapsible.Root bind:open={openToolSteps[idx]}>
              <Collapsible.Trigger
                class="group flex w-full items-center gap-2 rounded-lg border border-border/50 bg-muted/30 px-3 py-2 text-xs text-muted-foreground transition-colors hover:bg-muted/50"
              >
                <ChevronRight class="size-3 shrink-0 transition-transform duration-200 group-data-[state=open]:rotate-90" />
                {#if msg.error}
                  <CircleAlert class="size-3.5 shrink-0 text-destructive" />
                  <span class="flex-1 truncate text-left font-medium text-destructive">{msg.tool ?? "Error"}</span>
                  <Badge variant="destructive" class="h-4 px-1.5 text-[10px]">failed</Badge>
                {:else if msg.content?.startsWith("Calling") || msg.content?.includes("running")}
                  <Loader2 class="size-3.5 shrink-0 animate-spin text-chart-1" />
                  <span class="flex-1 truncate text-left font-medium text-chart-1">{msg.tool ?? "Tool"}</span>
                  <Badge variant="secondary" class="h-4 px-1.5 text-[10px]">running</Badge>
                {:else}
                  <CircleCheck class="size-3.5 shrink-0 text-chart-1" />
                  <span class="flex-1 truncate text-left font-medium">{msg.tool ?? "Tool"}</span>
                  <Badge variant="secondary" class="h-4 px-1.5 text-[10px]">done</Badge>
                {/if}
              </Collapsible.Trigger>
              <Collapsible.Content>
                {#if msg.content}
                  <div class="mt-1 ml-5 rounded-lg border border-border/40 bg-muted/20 p-3 font-mono text-[11px] leading-relaxed text-muted-foreground max-h-40 overflow-y-auto whitespace-pre-wrap">
                    {msg.content}
                  </div>
                {/if}
              </Collapsible.Content>
            </Collapsible.Root>
          </div>

        <!-- ─── User message ─── -->
        {:else if msg.role === "user"}
          <div class="flex items-end gap-2.5 justify-end px-1 py-1">
            <div class="max-w-[85%] flex flex-col items-end gap-1">
              <div class="rounded-2xl rounded-br-md bg-primary px-3.5 py-2.5 text-sm text-primary-foreground leading-relaxed">
                {msg.content}
              </div>
            </div>
            <Avatar.Root class="size-7 shrink-0 border border-border">
              <Avatar.Fallback class="bg-muted text-muted-foreground text-xs">
                <User class="size-3.5" />
              </Avatar.Fallback>
            </Avatar.Root>
          </div>

        <!-- ─── Assistant message ─── -->
        {:else}
          <div class="flex items-start gap-2.5 px-1 py-1">
            <Avatar.Root class="size-7 shrink-0 border border-chart-2/30 bg-chart-2/10">
              <Avatar.Fallback class="text-chart-2 text-xs">
                <Sparkles class="size-3.5" />
              </Avatar.Fallback>
            </Avatar.Root>
            <div class="max-w-[90%] flex flex-col gap-1.5 min-w-0">
              <!-- Streaming dots -->
              {#if msg.streaming && !msg.content}
                <div class="flex items-center gap-1.5 rounded-2xl rounded-bl-md border border-border/50 bg-muted/30 px-4 py-3">
                  <span class="size-1.5 rounded-full bg-muted-foreground/60 animate-pulse"></span>
                  <span class="size-1.5 rounded-full bg-muted-foreground/60 animate-pulse" style="animation-delay: 150ms"></span>
                  <span class="size-1.5 rounded-full bg-muted-foreground/60 animate-pulse" style="animation-delay: 300ms"></span>
                </div>
              {/if}

              <!-- Message content -->
              {#if msg.content}
                <div class="group relative overflow-hidden rounded-2xl rounded-bl-md border border-border/50 bg-muted/30 px-3.5 py-2.5 text-sm leading-relaxed {msg.error ? 'border-destructive/30 bg-destructive/5 text-destructive' : 'text-foreground'}">
                  <!-- Copy button -->
                  <button
                    class="absolute top-2 right-2 z-10 rounded-md p-1 opacity-0 transition-opacity group-hover:opacity-100 hover:bg-muted"
                    onclick={() => copyToClipboard(msg.content, idx)}
                  >
                    {#if copiedId === idx}
                      <Check class="size-3 text-chart-1" />
                    {:else}
                      <Copy class="size-3 text-muted-foreground" />
                    {/if}
                  </button>
                  <!-- Rendered markdown -->
                  <div class="prose prose-sm prose-invert max-w-none wrap-break-word overflow-wrap-anywhere [&_pre]:overflow-x-auto [&_pre]:rounded-lg [&_pre]:bg-background/80 [&_pre]:p-3 [&_pre]:text-xs [&_pre]:border [&_pre]:border-border/50 [&_code]:text-chart-1 [&_code]:text-xs [&_code]:font-mono [&_code]:break-all [&_p]:my-1.5 [&_ul]:my-1.5 [&_ol]:my-1.5 [&_li]:my-0.5 [&_h1]:text-base [&_h2]:text-sm [&_h3]:text-sm [&_a]:text-chart-2 [&_strong]:text-foreground [&_hr]:border-border/30 [&_blockquote]:border-l-chart-2/50 [&_blockquote]:text-muted-foreground">
                    {@html renderMarkdown(msg.content)}
                  </div>
                </div>
              {/if}

              <!-- Ticket created badge -->
              {#if msg.ticket}
                <button class="flex w-fit items-center gap-1.5 rounded-lg border border-chart-1/20 bg-chart-1/5 px-2.5 py-1.5 text-xs text-chart-1 transition-colors hover:bg-chart-1/10">
                  <CircleCheck class="size-3" />
                  Ticket created on Kanban
                </button>
              {/if}

              <!-- Apply generated UI -->
              {#if msg.spec}
                <button
                  class="flex w-fit items-center gap-1.5 rounded-lg border border-chart-2/20 bg-chart-2/5 px-2.5 py-1.5 text-xs text-chart-2 transition-colors hover:bg-chart-2/10"
                  onclick={() => onApplyPatch && onApplyPatch(msg.spec)}
                >
                  <Play class="size-3" />
                  Preview generated UI
                </button>
              {/if}
            </div>
          </div>
        {/if}
      {/each}
    </div>
  </div>

  <Separator />

  <!-- ─── Input area ─── -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="shrink-0 bg-background/80 backdrop-blur-sm transition-colors {isDragOver ? 'ring-2 ring-inset ring-chart-2/40' : ''}"
    ondragover={handleDragOver}
    ondragleave={handleDragLeave}
    ondrop={handleDrop}
  >
    <!-- Context file chips -->
    {#if contextFiles.length > 0}
      <div class="flex flex-wrap gap-1.5 px-3 pt-2.5">
        {#each contextFiles as file}
          <Badge variant="secondary" class="gap-1 pl-1.5 pr-1 py-1 text-[11px] max-w-[150px]">
            <FileCode2 class="size-3 shrink-0 text-chart-2" />
            <span class="truncate" title={file.path}>{file.name}</span>
            <button
              class="ml-0.5 rounded-full p-0.5 transition-colors hover:bg-destructive/20 hover:text-destructive"
              onclick={() => removeContextFile(file.path)}
            >
              <X class="size-2.5" />
            </button>
          </Badge>
        {/each}
      </div>
    {/if}

    <!-- Drop hint -->
    {#if isDragOver}
      <div class="mx-3 mt-2 flex items-center justify-center gap-2 rounded-lg border border-dashed border-chart-2/40 bg-chart-2/5 py-2 text-xs text-chart-2">
        <Paperclip class="size-3.5" />
        Drop file to add as context
      </div>
    {/if}

    <!-- Autocomplete popup -->
    <div class="relative px-3 pt-2">
      {#if showAutocomplete && filteredSuggestions.length > 0}
        <div class="absolute bottom-full left-3 right-3 mb-1.5 max-h-64 overflow-y-auto rounded-xl border border-border bg-popover p-1 shadow-lg z-50">
          <div class="px-2.5 py-1.5 text-[10px] font-semibold uppercase tracking-widest text-muted-foreground/50">
            {mentionType === "@" ? "Mentions" : "Commands"}
          </div>
          {#each filteredSuggestions as suggestion, i}
            <button
              class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-sm transition-colors
                     {i === autocompleteIndex
                       ? 'bg-accent text-accent-foreground'
                       : 'text-foreground hover:bg-muted/50'}"
              onclick={() => selectSuggestion(suggestion)}
            >
              <span class="flex size-7 shrink-0 items-center justify-center rounded-md border border-border/50 bg-muted/50 text-xs text-muted-foreground">
                {suggestion.label.startsWith("/") ? "/" : "@"}
              </span>
              <div class="flex-1 min-w-0">
                <div class="text-xs font-medium">{suggestion.label}</div>
                <div class="text-[11px] text-muted-foreground/70 truncate">{suggestion.description}</div>
              </div>
              {#if i === autocompleteIndex}
                <kbd class="rounded border border-border/50 bg-muted/50 px-1.5 py-0.5 text-[9px] text-muted-foreground/50 font-mono">Enter</kbd>
              {/if}
            </button>
          {/each}
          <div class="flex items-center gap-3 border-t border-border/50 px-2.5 py-1.5 text-[9px] text-muted-foreground/40">
            <span><kbd class="rounded border border-border/30 bg-muted/30 px-1 py-0.5 font-mono">↑↓</kbd> navigate</span>
            <span><kbd class="rounded border border-border/30 bg-muted/30 px-1 py-0.5 font-mono">Enter</kbd> select</span>
            <span><kbd class="rounded border border-border/30 bg-muted/30 px-1 py-0.5 font-mono">Esc</kbd> close</span>
          </div>
        </div>
      {/if}

      <!-- Textarea -->
      <textarea
        bind:this={textareaRef}
        bind:value={prompt}
        onkeydown={handleKeydown}
        oninput={handleInput}
        disabled={isGenerating}
        rows="2"
        placeholder={isDragOver ? "Drop file here..." : (contextFiles.length > 0 ? `Ask about ${contextFiles.map(f => f.name).join(", ")}...` : (chatMode === 'figma' ? "Describe the UI you want..." : chatMode === 'scrum' ? "Type a command or ask a question..." : "Ask about your code..."))}
        class="w-full resize-none rounded-lg border-none bg-transparent px-0.5 py-2 text-sm text-foreground placeholder:text-muted-foreground/50 focus:outline-none min-h-[48px] max-h-32"
      ></textarea>
    </div>

    <!-- Toolbar -->
    <div class="flex items-center justify-between px-3 pb-2.5 pt-0.5">
      <div class="flex items-center gap-1">

        <!-- Attach file -->
        <Tooltip.Root>
          <Tooltip.Trigger>
            <Button variant="ghost" size="icon-xs" class="text-muted-foreground">
              <Paperclip class="size-3.5" />
            </Button>
          </Tooltip.Trigger>
          <Tooltip.Content>Attach file</Tooltip.Content>
        </Tooltip.Root>

        <!-- Model selector -->
        <DropdownMenu.Root>
          <DropdownMenu.Trigger>
            <Button variant="ghost" size="xs" class="gap-1 text-muted-foreground">
              <Sparkles class="size-3" />
              <span class="max-w-[80px] truncate">{activeModelLabel}</span>
              <ChevronDown class="size-3 opacity-50" />
            </Button>
          </DropdownMenu.Trigger>
          <DropdownMenu.Content align="start" side="bottom" class="w-56">
            <DropdownMenu.Label>Model</DropdownMenu.Label>
            <DropdownMenu.Separator />
            <DropdownMenu.Item onclick={() => selectedModelId = "global"} class="gap-2">
              <span class="flex size-5 items-center justify-center rounded bg-muted text-[10px] text-muted-foreground">G</span>
              <span class="flex-1">Global Default</span>
              {#if selectedModelId === "global"}
                <Check class="size-3.5 text-chart-1" />
              {/if}
            </DropdownMenu.Item>
            {#each availableModels as mdl}
              <DropdownMenu.Item onclick={() => selectedModelId = mdl.id} class="gap-2">
                <span class="flex size-5 items-center justify-center rounded text-[10px] font-medium
                  {mdl.provider === 'openai' || mdl.provider === 'omx' ? 'bg-chart-1/15 text-chart-1'
                   : mdl.provider === 'anthropic' ? 'bg-chart-5/15 text-chart-5'
                   : mdl.provider === 'google' || mdl.provider === 'gemini' ? 'bg-chart-2/15 text-chart-2'
                   : 'bg-muted text-muted-foreground'}">
                  {mdl.provider.charAt(0).toUpperCase()}
                </span>
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-1.5 text-xs">
                    <span class="truncate">{mdl.label}</span>
                    {#if mdl.isActive}
                      <span class="size-1.5 rounded-full bg-chart-1 shrink-0"></span>
                    {/if}
                  </div>
                  <div class="text-[10px] text-muted-foreground/60 truncate">{mdl.model}</div>
                </div>
                {#if selectedModelId === mdl.id}
                  <Check class="size-3.5 text-chart-1 shrink-0" />
                {/if}
              </DropdownMenu.Item>
            {/each}
            {#if availableModels.length === 0}
              <div class="px-2 py-1.5 text-[11px] text-muted-foreground/50 italic">No providers configured</div>
            {/if}
          </DropdownMenu.Content>
        </DropdownMenu.Root>

        <!-- Mode badge -->
        <Badge variant="outline" class="text-[10px] h-5 {
          chatMode === 'figma' ? 'border-chart-3/30 text-chart-3'
          : chatMode === 'scrum' ? 'border-chart-5/30 text-chart-5'
          : 'border-chart-1/30 text-chart-1'
        }">
          {chatMode === 'figma' ? 'Design' : chatMode === 'scrum' ? 'Scrum' : 'Dev'}
        </Badge>
      </div>

      <!-- Send / Stop -->
      <div class="flex items-center gap-1">
        {#if isGenerating}
          <Button variant="outline" size="icon-xs" onclick={() => isGenerating = false}>
            <Square class="size-3" />
          </Button>
        {:else}
          <Button
            size="icon-xs"
            onclick={send}
            disabled={!prompt.trim()}
            class="transition-all {prompt.trim() ? '' : 'opacity-40'}"
          >
            <Send class="size-3" />
          </Button>
        {/if}
      </div>
    </div>
  </div>
</div>
