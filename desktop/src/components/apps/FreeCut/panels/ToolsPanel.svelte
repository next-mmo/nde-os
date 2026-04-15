<svelte:options runes={true} />

<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import type { MediaItem } from "../types";
  import { logStore } from "$lib/stores/logs";

  type FfmpegToolId = 'convert' | 'extract-audio' | 'trim' | 'compress' | 'resize' | 'remove-audio' | 'gif' | 'info' | 'split';

  interface Props {
    mediaLibrary: MediaItem[];
  }

  let { mediaLibrary }: Props = $props();

  let ffToolActive = $state<FfmpegToolId | null>(null);
  let ffToolBusy = $state(false);
  let ffToolError = $state<string | null>(null);
  let ffToolResult = $state<string | null>(null);
  let ffToolInfo = $state<any>(null);

  // Tool form fields
  let ffInputPath = $state('');
  let ffConvertFormat = $state('mp4');
  let ffAudioFormat = $state('mp3');
  let ffTrimStart = $state('00:00:00');
  let ffTrimEnd = $state('00:01:00');
  let ffCompressCrf = $state(28);
  let ffCompressPreset = $state('medium');
  let ffResizeW = $state(1920);
  let ffResizeH = $state(1080);
  let ffGifStart = $state('00:00:00');
  let ffGifDuration = $state('5');
  let ffGifFps = $state(10);
  let ffGifWidth = $state(480);

  // Split tool state
  let ffSplitUnit = $state<'seconds' | 'minutes'>('minutes');
  let ffSplitValue = $state(1);
  let ffSplitInputDuration = $state<number | null>(null);
  let ffSplitBusy = $state(false);
  let ffSplitDoneParts = $state(0);

  // Derived: total parts based on input file duration
  let ffSplitSegSeconds = $derived(
    ffSplitUnit === 'minutes' ? ffSplitValue * 60 : ffSplitValue
  );
  let ffSplitTotalParts = $derived(
    ffSplitInputDuration && ffSplitSegSeconds > 0
      ? Math.ceil(ffSplitInputDuration / ffSplitSegSeconds)
      : null
  );

  async function ffProbeInputDuration() {
    if (!ffInputPath) return;
    try {
      const resp = await fetch('http://localhost:8080/api/freecut/ffmpeg/info', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ input_path: ffInputPath }),
      });
      const data = await resp.json();
      const dur = parseFloat(data?.data?.format?.duration ?? '0');
      ffSplitInputDuration = dur > 0 ? dur : null;
    } catch { ffSplitInputDuration = null; }
  }

  function ffUseMediaInput() {
    const vid = mediaLibrary.find(m => m.mediaType === 'video') ?? mediaLibrary.find(m => m.mediaType === 'audio');
    if (vid) ffInputPath = vid.filePath;
  }

  async function ffBrowseFile() {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          { name: 'Media', extensions: ['mp4', 'mkv', 'avi', 'mov', 'webm', 'flv', 'ts', 'mp3', 'wav', 'aac', 'flac', 'ogg', 'png', 'jpg', 'jpeg', 'gif', 'bmp', 'webp'] },
          { name: 'All Files', extensions: ['*'] },
        ],
      });
      if (selected && typeof selected === 'string') {
        ffInputPath = selected;
      }
    } catch (e) {
      console.error('File browse cancelled or failed:', e);
    }
  }

  async function ffRunTool(endpoint: string, body: Record<string, any>) {
    ffToolBusy = true;
    ffToolError = null;
    ffToolResult = null;
    ffToolInfo = null;
    logStore.info(`FFmpeg ${endpoint}: starting…`, 'freecut');
    try {
      const resp = await fetch(`http://localhost:8080/api/freecut/ffmpeg/${endpoint}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      const data = await resp.json();
      if (data.success) {
        if (endpoint === 'info') {
          ffToolInfo = data.data;
          ffToolResult = 'Media info loaded';
          logStore.info(`FFmpeg info: ${data.data?.format?.format_long_name ?? 'ok'}`, 'freecut');
        } else {
          ffToolResult = `✅ Done! Output: ${data.data?.output_path ?? 'complete'}`;
          logStore.success(`FFmpeg ${endpoint}: done → ${data.data?.output_path ?? 'complete'}`, 'freecut');
        }
      } else {
        ffToolError = data.message ?? 'Operation failed';
        logStore.error(`FFmpeg ${endpoint}: ${ffToolError}`, 'freecut');
      }
    } catch (e: any) {
      const msg = e?.message ?? String(e);
      ffToolError = msg.includes('Load failed') || msg.includes('fetch')
        ? 'Cannot reach NDE-OS server (localhost:8080)'
        : msg;
      logStore.error(`FFmpeg ${endpoint}: ${ffToolError}`, 'freecut');
    } finally {
      ffToolBusy = false;
    }
  }
</script>

<div class="p-3.5 space-y-3">
  <div class="rounded-2xl border border-white/8 bg-linear-to-br from-white/[0.05] to-white/[0.015] p-3 space-y-3">
    <div class="flex items-start justify-between gap-3">
      <div class="min-w-0">
        <p class="text-xs font-semibold text-white/85">🛠️ FFmpeg Tools</p>
        <p class="text-[10px] text-white/40 mt-1">Common video/audio operations powered by FFmpeg.</p>
      </div>
    </div>

    <!-- Input file -->
    <div class="space-y-1.5">
      <label class="text-[9px] uppercase tracking-wider text-white/35 block">Input File</label>
      <div class="flex gap-2">
        <input type="text" bind:value={ffInputPath} placeholder="/path/to/file.mp4"
          class="flex-1 rounded-lg border border-white/10 bg-black/30 px-3 py-2 text-[11px] text-white/80 outline-none focus:border-violet-500/40 font-mono" />
        <button class="rounded-lg bg-violet-600/80 px-2.5 py-2 text-[10px] text-white/90 hover:bg-violet-500 transition-colors font-medium shrink-0"
          onclick={ffBrowseFile}>Browse</button>
      </div>
      {#if mediaLibrary.length > 0}
        <select
          class="w-full rounded-lg border border-white/10 bg-white/5 px-2.5 py-2 text-[11px] text-white/70 outline-none focus:border-violet-500/40"
          onchange={(e) => { const v = (e.target as HTMLSelectElement).value; if (v) ffInputPath = v; }}
        >
          <option value="" class="bg-zinc-900">— Pick from imported media —</option>
          {#each mediaLibrary as media (media.id)}
            <option value={media.filePath} class="bg-zinc-900">
              {media.mediaType === 'video' ? '🎬' : media.mediaType === 'audio' ? '🎵' : '🖼️'} {media.fileName}
            </option>
          {/each}
        </select>
      {/if}
    </div>

    <!-- Tool cards grid -->
    <div class="grid grid-cols-4 gap-1.5">
      {#each [
        { id: 'convert', icon: '🔄', label: 'Convert' },
        { id: 'extract-audio', icon: '🎧', label: 'Audio' },
        { id: 'trim', icon: '✂️', label: 'Trim' },
        { id: 'compress', icon: '📦', label: 'Compress' },
        { id: 'resize', icon: '📏', label: 'Resize' },
        { id: 'remove-audio', icon: '🔇', label: 'Mute' },
        { id: 'gif', icon: '🎬', label: 'GIF' },
        { id: 'split', icon: '🔀', label: 'Split' },
        { id: 'info', icon: 'ℹ️', label: 'Info' },
      ] as tool (tool.id)}
        <button
          class="flex flex-col items-center gap-1 rounded-xl border px-2 py-2.5 text-center transition-all {ffToolActive === tool.id ? 'border-violet-500/30 bg-violet-500/10 text-violet-300' : 'border-white/6 bg-black/20 text-white/50 hover:bg-white/5 hover:text-white/70'}"
          onclick={() => ffToolActive = ffToolActive === tool.id ? null : tool.id as FfmpegToolId}
        >
          <span class="text-sm">{tool.icon}</span>
          <span class="text-[9px] font-medium">{tool.label}</span>
        </button>
      {/each}
    </div>
  </div>

  <!-- Active tool panel -->
  {#if ffToolActive}
    <div class="rounded-2xl border border-white/8 bg-white/[0.02] p-3 space-y-3">
      {#if ffToolActive === 'convert'}
        <p class="text-[10px] font-semibold text-white/70">🔄 Convert Format</p>
        <div class="flex gap-2">
          <select bind:value={ffConvertFormat} class="flex-1 rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white">
            <option value="mp4">MP4</option>
            <option value="mkv">MKV</option>
            <option value="avi">AVI</option>
            <option value="webm">WebM</option>
            <option value="mov">MOV</option>
            <option value="ts">TS</option>
            <option value="flv">FLV</option>
          </select>
          <button class="rounded-lg bg-violet-600 px-4 py-2 text-[11px] font-medium text-white hover:bg-violet-500 transition-colors disabled:opacity-40"
            onclick={() => ffRunTool('convert', { input_path: ffInputPath, output_format: ffConvertFormat })}
            disabled={ffToolBusy || !ffInputPath}>
            {ffToolBusy ? '⏳...' : 'Convert'}
          </button>
        </div>

      {:else if ffToolActive === 'extract-audio'}
        <p class="text-[10px] font-semibold text-white/70">🎧 Extract Audio</p>
        <div class="flex gap-2">
          <select bind:value={ffAudioFormat} class="flex-1 rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white">
            <option value="mp3">MP3</option>
            <option value="wav">WAV</option>
            <option value="aac">AAC</option>
            <option value="flac">FLAC</option>
            <option value="ogg">OGG</option>
          </select>
          <button class="rounded-lg bg-violet-600 px-4 py-2 text-[11px] font-medium text-white hover:bg-violet-500 transition-colors disabled:opacity-40"
            onclick={() => ffRunTool('extract-audio', { input_path: ffInputPath, output_format: ffAudioFormat })}
            disabled={ffToolBusy || !ffInputPath}>
            {ffToolBusy ? '⏳...' : 'Extract'}
          </button>
        </div>

      {:else if ffToolActive === 'trim'}
        <p class="text-[10px] font-semibold text-white/70">✂️ Trim / Cut</p>
        <div class="grid grid-cols-2 gap-2">
          <label class="block">
            <span class="text-[9px] uppercase tracking-wider text-white/35">Start</span>
            <input type="text" bind:value={ffTrimStart} placeholder="00:00:00"
              class="mt-1 w-full rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white font-mono" />
          </label>
          <label class="block">
            <span class="text-[9px] uppercase tracking-wider text-white/35">End</span>
            <input type="text" bind:value={ffTrimEnd} placeholder="00:01:00"
              class="mt-1 w-full rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white font-mono" />
          </label>
        </div>
        <button class="w-full rounded-lg bg-violet-600 px-4 py-2 text-[11px] font-medium text-white hover:bg-violet-500 transition-colors disabled:opacity-40"
          onclick={() => ffRunTool('trim', { input_path: ffInputPath, start_time: ffTrimStart, end_time: ffTrimEnd })}
          disabled={ffToolBusy || !ffInputPath}>
          {ffToolBusy ? '⏳ Trimming...' : 'Trim'}
        </button>

      {:else if ffToolActive === 'compress'}
        <p class="text-[10px] font-semibold text-white/70">📦 Compress Video</p>
        <div class="grid grid-cols-2 gap-2">
          <label class="block">
            <span class="text-[9px] uppercase tracking-wider text-white/35">CRF (0–51, lower=better)</span>
            <input type="number" min="0" max="51" bind:value={ffCompressCrf}
              class="mt-1 w-full rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white" />
          </label>
          <label class="block">
            <span class="text-[9px] uppercase tracking-wider text-white/35">Preset</span>
            <select bind:value={ffCompressPreset} class="mt-1 w-full rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white">
              <option value="ultrafast">Ultrafast</option>
              <option value="fast">Fast</option>
              <option value="medium">Medium</option>
              <option value="slow">Slow</option>
              <option value="veryslow">Very Slow</option>
            </select>
          </label>
        </div>
        <button class="w-full rounded-lg bg-violet-600 px-4 py-2 text-[11px] font-medium text-white hover:bg-violet-500 transition-colors disabled:opacity-40"
          onclick={() => ffRunTool('compress', { input_path: ffInputPath, crf: ffCompressCrf, preset: ffCompressPreset })}
          disabled={ffToolBusy || !ffInputPath}>
          {ffToolBusy ? '⏳ Compressing...' : 'Compress'}
        </button>

      {:else if ffToolActive === 'resize'}
        <p class="text-[10px] font-semibold text-white/70">📏 Resize / Scale</p>
        <div class="grid grid-cols-2 gap-2">
          <label class="block">
            <span class="text-[9px] uppercase tracking-wider text-white/35">Width</span>
            <input type="number" min="1" bind:value={ffResizeW}
              class="mt-1 w-full rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white" />
          </label>
          <label class="block">
            <span class="text-[9px] uppercase tracking-wider text-white/35">Height</span>
            <input type="number" min="1" bind:value={ffResizeH}
              class="mt-1 w-full rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white" />
          </label>
        </div>
        <div class="flex gap-1.5 flex-wrap">
          {#each [{w:1920,h:1080,l:'1080p'},{w:1280,h:720,l:'720p'},{w:854,h:480,l:'480p'},{w:640,h:360,l:'360p'}] as p (p.l)}
            <button class="rounded-md bg-white/5 px-2 py-1 text-[9px] text-white/50 hover:bg-white/10 hover:text-white/70 transition-colors"
              onclick={() => { ffResizeW = p.w; ffResizeH = p.h; }}>{p.l}</button>
          {/each}
        </div>
        <button class="w-full rounded-lg bg-violet-600 px-4 py-2 text-[11px] font-medium text-white hover:bg-violet-500 transition-colors disabled:opacity-40"
          onclick={() => ffRunTool('resize', { input_path: ffInputPath, width: ffResizeW, height: ffResizeH })}
          disabled={ffToolBusy || !ffInputPath}>
          {ffToolBusy ? '⏳ Resizing...' : `Resize to ${ffResizeW}×${ffResizeH}`}
        </button>

      {:else if ffToolActive === 'remove-audio'}
        <p class="text-[10px] font-semibold text-white/70">🔇 Remove Audio (Mute)</p>
        <p class="text-[9px] text-white/40">Strip the audio track, keeping video untouched.</p>
        <button class="w-full rounded-lg bg-violet-600 px-4 py-2 text-[11px] font-medium text-white hover:bg-violet-500 transition-colors disabled:opacity-40"
          onclick={() => ffRunTool('remove-audio', { input_path: ffInputPath })}
          disabled={ffToolBusy || !ffInputPath}>
          {ffToolBusy ? '⏳...' : 'Remove Audio'}
        </button>

      {:else if ffToolActive === 'gif'}
        <p class="text-[10px] font-semibold text-white/70">🎬 Create GIF</p>
        <div class="grid grid-cols-2 gap-2">
          <label class="block">
            <span class="text-[9px] uppercase tracking-wider text-white/35">Start Time</span>
            <input type="text" bind:value={ffGifStart} placeholder="00:00:00"
              class="mt-1 w-full rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white font-mono" />
          </label>
          <label class="block">
            <span class="text-[9px] uppercase tracking-wider text-white/35">Duration (sec)</span>
            <input type="text" bind:value={ffGifDuration}
              class="mt-1 w-full rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white" />
          </label>
          <label class="block">
            <span class="text-[9px] uppercase tracking-wider text-white/35">FPS</span>
            <input type="number" min="1" max="30" bind:value={ffGifFps}
              class="mt-1 w-full rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white" />
          </label>
          <label class="block">
            <span class="text-[9px] uppercase tracking-wider text-white/35">Width (px)</span>
            <input type="number" min="100" bind:value={ffGifWidth}
              class="mt-1 w-full rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white" />
          </label>
        </div>
        <button class="w-full rounded-lg bg-violet-600 px-4 py-2 text-[11px] font-medium text-white hover:bg-violet-500 transition-colors disabled:opacity-40"
          onclick={() => ffRunTool('gif', { input_path: ffInputPath, start_time: ffGifStart, duration: ffGifDuration, fps: ffGifFps, width: ffGifWidth })}
          disabled={ffToolBusy || !ffInputPath}>
          {ffToolBusy ? '⏳ Creating GIF...' : 'Create GIF'}
        </button>

      {:else if ffToolActive === 'split'}
        <p class="text-[10px] font-semibold text-white/70">🔀 Split Video into Parts</p>
        <p class="text-[9px] text-white/40">Divide the video into equal-length segments.</p>

        <!-- Segment duration input -->
        <div class="flex gap-2 items-end">
          <div class="flex-1 space-y-1">
            <span class="text-[9px] uppercase tracking-wider text-white/35">Segment Length</span>
            <input type="number" min="0.1" step="0.1" bind:value={ffSplitValue}
              oninput={() => {
                if (ffSplitUnit === 'minutes' && ffSplitValue < 1) {
                  // auto-switch to seconds and convert
                  ffSplitValue = Math.round(ffSplitValue * 60);
                  ffSplitUnit = 'seconds';
                }
              }}
              class="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-[11px] text-white outline-none focus:border-violet-500/40" />
          </div>
          <div class="space-y-1">
            <span class="text-[9px] uppercase tracking-wider text-white/35">Unit</span>
            <div class="flex gap-1">
              <button
                class="rounded-lg border px-3 py-2 text-[10px] font-medium transition-colors {ffSplitUnit === 'seconds' ? 'border-violet-500/40 bg-violet-500/15 text-violet-300' : 'border-white/10 bg-white/5 text-white/50 hover:bg-white/10'}"
                onclick={() => ffSplitUnit = 'seconds'}>sec</button>
              <button
                class="rounded-lg border px-3 py-2 text-[10px] font-medium transition-colors {ffSplitUnit === 'minutes' ? 'border-violet-500/40 bg-violet-500/15 text-violet-300' : 'border-white/10 bg-white/5 text-white/50 hover:bg-white/10'}"
                onclick={() => ffSplitUnit = 'minutes'}>min</button>
            </div>
          </div>
        </div>

        <!-- Probe button + total parts display -->
        <div class="flex items-center gap-2">
          <button
            class="flex-1 rounded-lg border border-white/10 bg-white/5 px-3 py-1.5 text-[10px] text-white/60 hover:bg-white/10 transition-colors"
            onclick={ffProbeInputDuration}
            disabled={!ffInputPath}
          >🔍 Detect Duration</button>
          {#if ffSplitInputDuration}
            <span class="text-[10px] text-white/50 shrink-0">
              {Math.floor(ffSplitInputDuration / 60)}m {Math.round(ffSplitInputDuration % 60)}s
            </span>
          {/if}
        </div>

        <!-- Total parts preview -->
        {#if ffSplitTotalParts !== null}
          <div class="rounded-xl border border-violet-500/20 bg-violet-500/8 px-3 py-2.5 flex items-center justify-between">
            <span class="text-[10px] text-violet-300/80">Total parts</span>
            <span class="text-lg font-bold text-violet-300">{ffSplitTotalParts}</span>
          </div>
        {:else if ffInputPath && !ffSplitInputDuration}
          <p class="text-[9px] text-white/30 text-center">Click "Detect Duration" to preview total parts</p>
        {/if}

        <button
          class="w-full rounded-lg bg-violet-600 px-4 py-2 text-[11px] font-medium text-white hover:bg-violet-500 transition-colors disabled:opacity-40"
          onclick={async () => {
            ffToolBusy = true;
            ffToolError = null;
            ffToolResult = null;
            ffSplitDoneParts = 0;
            logStore.info(`Split: starting ~${ffSplitTotalParts ?? '?'} parts (${ffSplitSegSeconds}s each)…`, 'freecut');
            try {
              const resp = await fetch('http://localhost:8080/api/freecut/dub/split-stream', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ input_path: ffInputPath, segment_duration_secs: ffSplitSegSeconds }),
              });

              // Read SSE events line by line from the buffered response body
              const text = await resp.text();
              const lines = text.split('\n');
              let eventType = '';
              let dataLine = '';

              for (const line of lines) {
                if (line.startsWith('event: ')) {
                  eventType = line.slice(7).trim();
                } else if (line.startsWith('data: ')) {
                  dataLine = line.slice(6).trim();
                } else if (line === '') {
                  // End of event block — process
                  if (dataLine) {
                    try {
                      const payload = JSON.parse(dataLine);
                      if (eventType === 'progress') {
                        ffSplitDoneParts = payload.part;
                        logStore.info(
                          `Split part ${payload.part}/${payload.total} ✅ — ${payload.msg ?? ''}`,
                          'freecut'
                        );
                      } else if (eventType === 'done') {
                        const total = payload.total_parts ?? payload.parts?.length ?? ffSplitDoneParts;
                        ffToolResult = `✅ ${total} parts ready in: ${payload.workspace}`;
                        logStore.success(`Split done — ${total} parts → ${payload.workspace}`, 'freecut');
                      } else if (eventType === 'error') {
                        ffToolError = payload.error ?? 'Split failed';
                        logStore.error(`Split: ${ffToolError}`, 'freecut');
                      }
                    } catch { /* ignore malformed event */ }
                  }
                  eventType = '';
                  dataLine = '';
                }
              }
            } catch (e: any) {
              ffToolError = e?.message ?? String(e);
              logStore.error(`Split: ${ffToolError}`, 'freecut');
            } finally {
              ffToolBusy = false;
            }
          }}
          disabled={ffToolBusy || !ffInputPath || ffSplitValue < 1}
        >
          {ffToolBusy
            ? `⏳ Splitting… ${ffSplitDoneParts}/${ffSplitTotalParts ?? '?'} parts`
            : `Split into ~${ffSplitTotalParts ?? '?'} parts`}
        </button>

      {:else if ffToolActive === 'info'}
        <p class="text-[10px] font-semibold text-white/70">ℹ️ Media Info</p>
        <button class="w-full rounded-lg bg-violet-600 px-4 py-2 text-[11px] font-medium text-white hover:bg-violet-500 transition-colors disabled:opacity-40"
          onclick={() => ffRunTool('info', { input_path: ffInputPath })}
          disabled={ffToolBusy || !ffInputPath}>
          {ffToolBusy ? '⏳ Probing...' : 'Get Info'}
        </button>
        {#if ffToolInfo}
          <div class="rounded-xl border border-white/6 bg-black/30 p-2.5 max-h-[300px] overflow-y-auto">
            {#if ffToolInfo.format}
              <div class="space-y-1 mb-2">
                <p class="text-[9px] uppercase tracking-wider text-white/30">Format</p>
                <p class="text-[10px] text-white/70">{ffToolInfo.format.format_long_name ?? ffToolInfo.format.format_name}</p>
                <p class="text-[10px] text-white/50">Duration: {parseFloat(ffToolInfo.format.duration ?? 0).toFixed(1)}s · Size: {(parseInt(ffToolInfo.format.size ?? 0) / 1048576).toFixed(1)} MB</p>
                <p class="text-[10px] text-white/50">Bitrate: {(parseInt(ffToolInfo.format.bit_rate ?? 0) / 1000).toFixed(0)} kbps</p>
              </div>
            {/if}
            {#if ffToolInfo.streams}
              {#each ffToolInfo.streams as stream, i (i)}
                <div class="rounded-lg border border-white/5 bg-white/[0.02] p-2 mb-1.5">
                  <p class="text-[9px] uppercase tracking-wider text-white/30">Stream {i} — {stream.codec_type}</p>
                  <p class="text-[10px] text-white/60">{stream.codec_long_name ?? stream.codec_name}</p>
                  {#if stream.codec_type === 'video'}
                    <p class="text-[10px] text-white/50">{stream.width}×{stream.height} · {stream.r_frame_rate} fps</p>
                  {:else if stream.codec_type === 'audio'}
                    <p class="text-[10px] text-white/50">{stream.sample_rate}Hz · {stream.channels}ch</p>
                  {/if}
                </div>
              {/each}
            {/if}
          </div>
        {/if}
      {/if}

      <!-- Result / Error -->
      {#if ffToolError}
        <div class="rounded-lg border border-red-500/20 bg-red-500/8 px-3 py-2">
          <p class="text-[10px] text-red-300">{ffToolError}</p>
        </div>
      {/if}
      {#if ffToolResult}
        <p class="text-[10px] text-emerald-300/80">{ffToolResult}</p>
      {/if}
    </div>
  {/if}
</div>
