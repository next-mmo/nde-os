<script lang="ts">
  import { desktop, closeWindow, type DesktopWindow } from "🍎/state/desktop.svelte";
  
  interface ExtendedWindow extends DesktopWindow {
    data?: { image?: string };
  }

  interface Props {
    window: ExtendedWindow;
  }

  let { window }: Props = $props();

  let imageData = $derived(window.data?.image);

  async function copyToClipboard() {
    if (!imageData) return;
    try {
      // Create a blob from the base64 string and copy to clipboard
      const res = await fetch(imageData);
      const blob = await res.blob();
      await navigator.clipboard.write([
        new ClipboardItem({
          [blob.type]: blob,
        }),
      ]);
    } catch (err) {
      console.error("Failed to copy image: ", err);
    }
  }

  function closeApp() {
    closeWindow(window.id);
  }
</script>

<div class="flex flex-col h-full bg-background/80 backdrop-blur-xl rounded-xl overflow-hidden border border-white/10 shadow-2xl">
  <!-- Titlebar handles dragging if needed, but App window handles it. -->
  <div class="flex-1 p-4 flex flex-col items-center justify-center overflow-auto min-h-0 relative">
    {#if imageData}
      <img src={imageData} alt="Screenshot" class="max-w-full max-h-full object-contain rounded shadow-lg border border-white/5" />
    {:else}
      <p class="text-muted-foreground">No screenshot data available.</p>
    {/if}
  </div>

  <div class="p-4 border-t border-white/10 flex justify-end gap-2 bg-black/20">
    <button class="px-4 py-2 hover:bg-white/10 rounded border border-white/10" onclick={closeApp}>Close</button>
    <button class="px-4 py-2 bg-primary text-primary-foreground hover:brightness-110 rounded" onclick={copyToClipboard}>Copy to Clipboard</button>
  </div>
</div>
