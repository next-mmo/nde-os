<svelte:options runes={true} />

<script lang="ts">
  interface HwEncoder {
    name: string;
    codec: string;
    device: string;
  }

  interface Props {
    exportCodec: string;
    exportQuality: "low" | "medium" | "high" | "ultra";
    hwEncoders: HwEncoder[];
    exportHwAccel: string | null;
    isExporting: boolean;
    exportProgress: number;
    exportError: string | null;
    onCodecChange: (codec: string) => void;
    onQualityChange: (quality: "low" | "medium" | "high" | "ultra") => void;
    onHwAccelChange: (accel: string | null) => void;
    onExport: () => void;
    onClose: () => void;
  }

  let {
    exportCodec, exportQuality, hwEncoders, exportHwAccel,
    isExporting, exportProgress, exportError,
    onCodecChange, onQualityChange, onHwAccelChange,
    onExport, onClose,
  }: Props = $props();
</script>

<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
  <div class="w-[480px] rounded-2xl border border-white/10 bg-zinc-900 shadow-2xl overflow-hidden p-6" style="color-scheme: dark;">
    <h2 class="text-white text-lg font-medium mb-5">Export Video</h2>
    
    <div class="space-y-4 mb-6">
      <div>
        <label for="export-codec" class="block text-xs uppercase text-white/50 mb-1.5">Codec</label>
        <select id="export-codec" class="w-full bg-[#1c1c1c] border border-white/10 rounded px-3 py-2 text-sm text-white [&>option]:bg-zinc-900 [&>option]:text-white focus:outline-none focus:border-violet-500/50" value={exportCodec} onchange={(e) => onCodecChange(e.currentTarget.value)}>
          <option value="h264">H.264 (MP4)</option>
          <option value="hevc">H.265 / HEVC (MP4)</option>
          <option value="vp9">VP9 (WebM)</option>
        </select>
      </div>
      
      <div>
        <label for="export-quality" class="block text-xs uppercase text-white/50 mb-1.5">Quality Profile</label>
        <select id="export-quality" class="w-full bg-[#1c1c1c] border border-white/10 rounded px-3 py-2 text-sm text-white [&>option]:bg-zinc-900 [&>option]:text-white focus:outline-none focus:border-violet-500/50" value={exportQuality} onchange={(e) => onQualityChange(e.currentTarget.value as any)}>
          <option value="low">Fast / Low Size</option>
          <option value="medium">Balanced</option>
          <option value="high">High Quality</option>
          <option value="ultra">Ultra (Lossless)</option>
        </select>
      </div>

      <div>
        <label for="export-hwaccel" class="flex justify-between text-xs uppercase text-white/50 mb-1.5">
          Hardware Acceleration (GPU)
          <span class="text-white/30 lowercase">{hwEncoders.length} detected</span>
        </label>
        <select id="export-hwaccel" class="w-full bg-[#1c1c1c] border border-white/10 rounded px-3 py-2 text-sm text-white [&>option]:bg-zinc-900 [&>option]:text-white focus:outline-none focus:border-violet-500/50" value={exportHwAccel} onchange={(e) => onHwAccelChange(e.currentTarget.value || null)}>
          <option value={null}>None (CPU Rendering - Slowest)</option>
          {#each hwEncoders as enc}
            {#if enc.codec === (exportCodec === "h264" ? "H.264" : exportCodec === "hevc" ? "H.265" : "unknown")}
              <option value={enc.name}>{enc.device} ({enc.name})</option>
            {/if}
          {/each}
        </select>
      </div>
    </div>

    <div class="flex items-center justify-end gap-3 mt-6 pt-6 border-t border-white/10">
      {#if isExporting}
        <div class="flex-1 flex items-center gap-3 pr-4">
          <div class="flex-1 h-1.5 bg-white/10 rounded-full overflow-hidden">
            <div class="h-full bg-violet-500 rounded-full transition-all duration-300" style:width="{exportProgress}%"></div>
          </div>
          <span class="text-xs text-white/50 font-mono w-12 text-right">{exportProgress.toFixed(1)}%</span>
        </div>
      {:else}
        <button class="px-5 py-2 text-sm hover:bg-white/5 rounded-md text-white/50 transition-colors" onclick={onClose}>Cancel</button>
        <button class="px-6 py-2 text-sm bg-violet-600 hover:bg-violet-500 rounded-md text-white font-medium transition-colors shadow-[0_0_15px_rgba(124,58,237,0.4)]" onclick={onExport}>Export Now</button>
      {/if}
    </div>
    
    {#if exportError}
      <div class="mt-4 p-3 rounded-md bg-red-500/10 border border-red-500/20 text-red-400 text-xs">
        <strong>Export Failed:</strong> {exportError}
      </div>
    {/if}
  </div>
</div>
