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

    // Always define One Dark Pro (defineTheme is idempotent — safe to call every mount)
    const ONE_DARK_PRO_RULES = [
      { token: '',                        foreground: 'abb2bf' },
      // Comments
      { token: 'comment',                 foreground: '5c6370', fontStyle: 'italic' },
      { token: 'comment.line.double-slash', foreground: '5c6370', fontStyle: 'italic' },
      { token: 'comment.block',           foreground: '5c6370', fontStyle: 'italic' },
      // Keywords
      { token: 'keyword',                 foreground: 'c678dd' },
      { token: 'keyword.control',         foreground: 'c678dd' },
      { token: 'keyword.operator',        foreground: '56b6c2' },
      { token: 'keyword.other',           foreground: 'c678dd' },
      { token: 'storage',                 foreground: 'c678dd' },
      { token: 'storage.type',            foreground: 'c678dd' },
      // Strings
      { token: 'string',                  foreground: '98c379' },
      { token: 'string.quoted',           foreground: '98c379' },
      { token: 'string.template',         foreground: '98c379' },
      { token: 'string.escape',           foreground: '56b6c2' },
      // Numbers & constants
      { token: 'number',                  foreground: 'd19a66' },
      { token: 'constant.numeric',        foreground: 'd19a66' },
      { token: 'constant.language',       foreground: 'd19a66' },
      { token: 'constant.language.boolean', foreground: 'd19a66' },
      { token: 'constant',                foreground: 'd19a66' },
      // Types & classes
      { token: 'type',                    foreground: 'e5c07b' },
      { token: 'type.identifier',         foreground: 'e5c07b' },
      { token: 'entity.name.type',        foreground: 'e5c07b' },
      { token: 'entity.name.class',       foreground: 'e5c07b' },
      { token: 'support.class',           foreground: 'e5c07b' },
      // Functions & methods
      { token: 'entity.name.function',    foreground: '61afef' },
      { token: 'support.function',        foreground: '61afef' },
      { token: 'meta.function-call',      foreground: '61afef' },
      // Variables & parameters
      { token: 'variable',                foreground: 'e06c75' },
      { token: 'variable.other',          foreground: 'e06c75' },
      { token: 'variable.parameter',      foreground: 'abb2bf' },
      { token: 'variable.language',       foreground: 'e06c75' },
      // Operators / delimiters
      { token: 'operator',                foreground: '56b6c2' },
      { token: 'keyword.operator.assignment', foreground: '56b6c2' },
      { token: 'delimiter',               foreground: 'abb2bf' },
      { token: 'delimiter.bracket',       foreground: 'abb2bf' },
      { token: 'delimiter.parenthesis',   foreground: 'abb2bf' },
      // Tags (HTML / JSX / Svelte)
      { token: 'tag',                     foreground: 'e06c75' },
      { token: 'tag.html',                foreground: 'e06c75' },
      { token: 'attribute.name',          foreground: 'd19a66' },
      { token: 'attribute.value',         foreground: '98c379' },
      { token: 'metatag',                 foreground: 'c678dd' },
      // Markdown tokens
      { token: 'keyword.md',              foreground: 'e06c75' },   // headings #, ##
      { token: 'string.link.md',          foreground: '61afef' },
      { token: 'string.md',               foreground: '98c379' },
      { token: 'emphasis',                foreground: 'abb2bf', fontStyle: 'italic' },
      { token: 'strong',                  foreground: 'e6edf3', fontStyle: 'bold' },
      // Punctuation
      { token: 'punctuation',             foreground: 'abb2bf' },
    ];

    const ONE_DARK_PRO_COLORS: Record<string, string> = {
      'editor.background':                        '#282c34',
      'editor.foreground':                        '#abb2bf',
      'editor.lineHighlightBackground':           '#2c313a',
      'editor.selectionBackground':               '#3e4451',
      'editor.inactiveSelectionBackground':       '#3a3f4b',
      'editorCursor.foreground':                  '#528bff',
      'editorWhitespace.foreground':              '#3b4048',
      'editorLineNumber.foreground':              '#495162',
      'editorLineNumber.activeForeground':        '#abb2bf',
      'editorGutter.background':                  '#282c34',
      'editorIndentGuide.background1':            '#3b4048',
      'editorIndentGuide.activeBackground1':      '#c678dd55',
      'editor.findMatchBackground':               '#42557b',
      'editor.findMatchHighlightBackground':      '#314365',
      'scrollbarSlider.background':               '#4e566680',
      'scrollbarSlider.hoverBackground':          '#5a637580',
      'scrollbarSlider.activeBackground':         '#747d9180',
      'minimap.background':                       '#282c34',
      'editorWidget.background':                  '#21252b',
      'editorWidget.border':                      '#3e4451',
      'editorSuggestWidget.background':           '#21252b',
      'editorSuggestWidget.border':               '#3e4451',
      'editorSuggestWidget.selectedBackground':   '#2c313a',
      'editorBracketMatch.background':            '#515a6b55',
      'editorBracketMatch.border':                '#515a6b',
      'diffEditor.insertedTextBackground':        '#98c37922',
      'diffEditor.removedTextBackground':         '#e06c7522',
    };

    monaco.editor.defineTheme('one-dark-pro', {
      base: 'vs-dark',
      inherit: true,
      rules: ONE_DARK_PRO_RULES,
      colors: ONE_DARK_PRO_COLORS,
    });
    // Apply globally so any already-open editors also switch
    monaco.editor.setTheme('one-dark-pro');

    editor = monaco.editor.create(container, {
      value: content,
      language,
      theme: 'one-dark-pro',
      automaticLayout: true,
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
      readOnly,
      fontSize: 14,
      fontFamily: '"Cascadia Code", "JetBrains Mono", "Fira Code", Consolas, monospace',
      fontLigatures: true,
      lineHeight: 22,
      letterSpacing: 0.3,
      inlineSuggest: { enabled: true },
      renderLineHighlight: 'all',
      smoothScrolling: true,
      cursorBlinking: 'smooth',
      cursorSmoothCaretAnimation: 'on',
      bracketPairColorization: { enabled: true },
      guides: { bracketPairs: true, indentation: true },
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
