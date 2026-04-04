<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  
  let { content = "", language = "typescript", onChange, readOnly = false } = $props<{
    content?: string;
    language?: string;
    onChange?: (value: string) => void;
    readOnly?: boolean;
  }>();

  let container: HTMLDivElement | undefined = $state();
  let editor: any = $state();
  let monaco: any = $state();
  let ignoreEvent = false;

  onMount(async () => {
    // Lazy load monaco from CDN to avoid bundler issues
    if (!(window as any).monaco) {
      await new Promise<void>((resolve, reject) => {
        const script = document.createElement('script');
        script.src = 'https://cdnjs.cloudflare.com/ajax/libs/monaco-editor/0.45.0/min/vs/loader.min.js';
        script.onload = () => {
          (window as any).require.config({ paths: { 'vs': 'https://cdnjs.cloudflare.com/ajax/libs/monaco-editor/0.45.0/min/vs' }});
          (window as any).require(['vs/editor/editor.main'], () => {
            monaco = (window as any).monaco;
            resolve();
          });
        };
        script.onerror = reject;
        document.body.appendChild(script);
      });
    } else {
      monaco = (window as any).monaco;
    }

    if (!container) return;

    editor = monaco.editor.create(container, {
      value: content,
      language: language,
      theme: 'vs-dark',
      automaticLayout: true,
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
      readOnly: readOnly,
      fontSize: 14,
      fontFamily: '"Inter", "Cascadia Code", "JetBrains Mono", monospace',
      inlineSuggest: { enabled: true }
    });

    if (!(window as any).__monacoAgentRegistered) {
      (window as any).__monacoAgentRegistered = true;
      monaco.languages.registerInlineCompletionsProvider({ pattern: '**' }, {
        provideInlineCompletions: async (model: any, position: any, context: any, token: any) => {
          // Debounce / only trigger at end of typing could be added here
          const prefix = model.getValueInRange({ startLineNumber: 1, startColumn: 1, endLineNumber: position.lineNumber, endColumn: position.column });
          const suffix = model.getValueInRange({ startLineNumber: position.lineNumber, startColumn: position.column, endLineNumber: model.getLineCount(), endColumn: model.getLineMaxColumn(model.getLineCount()) });

          // Only trigger if prefix length is reasonable
          if (prefix.trim().length === 0) return { items: [] };

          try {
            const req = await fetch('http://localhost:8080/api/agent/autocomplete', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({ prefix, suffix, filename: "code" })
            });
            const data = await req.json();
            if (data && data.success && data.data && data.data.completion) {
              return {
                items: [{
                  insertText: data.data.completion,
                  range: new monaco.Range(position.lineNumber, position.column, position.lineNumber, position.column)
                }]
              };
            }
          } catch (e) {
            console.error('Monaco Agent Autocomplete error:', e);
          }
          return { items: [] };
        },
        freeInlineCompletions(completions: any) {}
      });
    }

    editor.onDidChangeModelContent(() => {
      ignoreEvent = true;
      onChange?.(editor.getValue());
      ignoreEvent = false;
    });
  });

  $effect(() => {
    if (editor && !ignoreEvent && editor.getValue() !== content) {
      ignoreEvent = true;
      const pos = editor.getPosition();
      editor.setValue(content);
      if (pos) editor.setPosition(pos);
      ignoreEvent = false;
    }
  });

  $effect(() => {
    if (monaco && editor) {
      monaco.editor.setModelLanguage(editor.getModel(), language);
    }
  });

  onDestroy(() => {
    if (editor) {
      editor.dispose();
    }
  });
</script>

<div bind:this={container} class="w-full h-full text-left"></div>
