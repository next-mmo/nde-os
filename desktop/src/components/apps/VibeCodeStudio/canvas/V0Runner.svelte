<script lang="ts">
  interface Props {
    code: string;
  }

  let { code }: Props = $props();

  let srcdoc = $derived(`
    <!DOCTYPE html>
    <html lang="en">
    <head>
      <meta charset="UTF-8">
      <meta name="viewport" content="width=device-width, initial-scale=1.0">
      <script src="https://cdn.tailwindcss.com"><\/script>
      <style>
        body {
          margin: 0;
          font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif, "Apple Color Emoji", "Segoe UI Emoji", "Segoe UI Symbol";
          background-color: transparent;
        }
        /* Custom scrollbar for webkit */
        ::-webkit-scrollbar {
          width: 8px;
          height: 8px;
        }
        ::-webkit-scrollbar-track {
          background: transparent;
        }
        ::-webkit-scrollbar-thumb {
          background: rgba(156, 163, 175, 0.5);
          border-radius: 4px;
        }
        ::-webkit-scrollbar-thumb:hover {
          background: rgba(156, 163, 175, 0.8);
        }
      </style>
      <script>
        tailwind.config = {
          theme: {
            extend: {
              colors: {
                border: "hsl(var(--border))",
                input: "hsl(var(--input))",
                ring: "hsl(var(--ring))",
                background: "hsl(var(--background))",
                foreground: "hsl(var(--foreground))",
                primary: {
                  DEFAULT: "hsl(var(--primary))",
                  foreground: "hsl(var(--primary-foreground))",
                },
                secondary: {
                  DEFAULT: "hsl(var(--secondary))",
                  foreground: "hsl(var(--secondary-foreground))",
                },
                destructive: {
                  DEFAULT: "hsl(var(--destructive))",
                  foreground: "hsl(var(--destructive-foreground))",
                },
                muted: {
                  DEFAULT: "hsl(var(--muted))",
                  foreground: "hsl(var(--muted-foreground))",
                },
                accent: {
                  DEFAULT: "hsl(var(--accent))",
                  foreground: "hsl(var(--accent-foreground))",
                },
                popover: {
                  DEFAULT: "hsl(var(--popover))",
                  foreground: "hsl(var(--popover-foreground))",
                },
                card: {
                  DEFAULT: "hsl(var(--card))",
                  foreground: "hsl(var(--card-foreground))",
                },
              },
            }
          }
        }
      <\/script>
    </head>
    <body class="antialiased text-slate-800 bg-white min-h-screen flex items-center justify-center p-4">
      ${code}
    </body>
    </html>
  `);
</script>

<div class="relative w-full h-full flex flex-col bg-slate-900 overflow-hidden">
  <!-- Interactive Browser Frame Header -->
  <div class="h-10 bg-slate-800 border-b border-white/10 flex items-center px-4 shrink-0 gap-3">
    <div class="flex gap-1.5">
      <div class="w-3 h-3 rounded-full bg-red-500/80"></div>
      <div class="w-3 h-3 rounded-full bg-yellow-500/80"></div>
      <div class="w-3 h-3 rounded-full bg-green-500/80"></div>
    </div>
    <div class="flex-1 max-w-xl bg-black/30 rounded-md text-[11px] text-white/50 px-3 py-1 text-center font-mono truncate border border-white/5">
      localhost:3000/preview
    </div>
  </div>

  <!-- The Actual Iframe -->
  <div class="flex-1 w-full bg-white relative">
    {#if !code}
      <div class="absolute inset-0 flex flex-col items-center justify-center text-slate-400 bg-slate-50">
        <svg class="w-12 h-12 mb-4 text-slate-300" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19.428 15.428a2 2 0 00-1.022-.547l-2.387-.477a6 6 0 00-3.86.517l-.318.158a6 6 0 01-3.86.517L6.05 15.21a2 2 0 00-1.806.547M8 4h8l-1 1v5.172a2 2 0 00.586 1.414l5 5c1.26 1.26.367 3.414-1.415 3.414H4.828c-1.782 0-2.674-2.154-1.414-3.414l5-5A2 2 0 009 10.172V5L8 4z"></path></svg>
        <p class="text-sm font-medium text-slate-500">Waiting for generated code...</p>
        <p class="text-xs mt-1 text-slate-400 max-w-sm text-center">Ask the Vibe Studio agent to build something, or paste code in the IDE tab.</p>
      </div>
    {:else}
      <iframe
        title="Live Preview"
        {srcdoc}
        sandbox="allow-scripts"
        class="w-full h-full border-none"
      ></iframe>
    {/if}
  </div>
</div>
