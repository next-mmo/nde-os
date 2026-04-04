<script lang="ts">
  import { marked } from 'marked';

  let { content = "" }: { content: string } = $props();

  // Configure marked renderer with GFM + line breaks
  marked.setOptions({ breaks: true, gfm: true } as any);

  let html = $derived(marked.parse(content) as string);
</script>

<!-- GitHub-dark markdown preview -->
<div class="absolute inset-0 overflow-y-auto px-8 py-6 bg-[#1e1e1e]">
  <div class="max-w-3xl mx-auto markdown-body">
    {@html html}
  </div>
</div>

<style>
  :global(.markdown-body) {
    color: #cdd9e5;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
    font-size: 15px;
    line-height: 1.7;
  }
  :global(.markdown-body h1),
  :global(.markdown-body h2),
  :global(.markdown-body h3),
  :global(.markdown-body h4),
  :global(.markdown-body h5),
  :global(.markdown-body h6) {
    color: #e6edf3;
    font-weight: 600;
    margin-top: 1.5em;
    margin-bottom: 0.5em;
    line-height: 1.3;
  }
  :global(.markdown-body h1) { font-size: 2em; border-bottom: 1px solid rgba(255,255,255,0.1); padding-bottom: 0.3em; }
  :global(.markdown-body h2) { font-size: 1.5em; border-bottom: 1px solid rgba(255,255,255,0.08); padding-bottom: 0.25em; }
  :global(.markdown-body h3) { font-size: 1.25em; }
  :global(.markdown-body h4) { font-size: 1em; }
  :global(.markdown-body p)  { margin: 0.75em 0; }
  :global(.markdown-body a)  { color: #58a6ff; text-decoration: none; }
  :global(.markdown-body a:hover) { text-decoration: underline; }
  :global(.markdown-body strong) { color: #e6edf3; font-weight: 600; }
  :global(.markdown-body em)     { font-style: italic; }
  :global(.markdown-body ul),
  :global(.markdown-body ol) { padding-left: 1.5em; margin: 0.5em 0; }
  :global(.markdown-body li) { margin: 0.25em 0; }
  :global(.markdown-body li input[type="checkbox"]) { margin-right: 0.4em; accent-color: #58a6ff; }
  :global(.markdown-body blockquote) {
    border-left: 3px solid rgba(99,102,241,0.5);
    padding: 0.25em 1em;
    margin: 0.75em 0;
    color: rgba(255,255,255,0.5);
    background: rgba(255,255,255,0.03);
    border-radius: 0 6px 6px 0;
  }
  :global(.markdown-body code) {
    font-family: 'Cascadia Code', 'JetBrains Mono', Consolas, monospace;
    font-size: 0.87em;
    background: rgba(255,255,255,0.08);
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 4px;
    padding: 0.15em 0.4em;
    color: #e06c75;
  }
  :global(.markdown-body pre) {
    background: #161b22;
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 8px;
    padding: 1em 1.25em;
    overflow-x: auto;
    margin: 1em 0;
  }
  :global(.markdown-body pre code) {
    background: none;
    border: none;
    padding: 0;
    color: #cdd9e5;
    font-size: 0.9em;
  }
  :global(.markdown-body table) {
    width: 100%;
    border-collapse: collapse;
    margin: 1em 0;
    font-size: 0.9em;
  }
  :global(.markdown-body th) {
    background: rgba(255,255,255,0.07);
    color: #e6edf3;
    font-weight: 600;
    padding: 0.5em 0.75em;
    border: 1px solid rgba(255,255,255,0.1);
    text-align: left;
  }
  :global(.markdown-body td) {
    padding: 0.5em 0.75em;
    border: 1px solid rgba(255,255,255,0.08);
  }
  :global(.markdown-body tr:nth-child(even)) { background: rgba(255,255,255,0.03); }
  :global(.markdown-body hr) {
    border: none;
    border-top: 1px solid rgba(255,255,255,0.1);
    margin: 1.5em 0;
  }
  :global(.markdown-body img) { max-width: 100%; border-radius: 6px; }
</style>
