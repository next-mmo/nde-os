<svelte:options runes={true} />

<script lang="ts">
  import { Settings } from "@lucide/svelte";
  import type { TimelineItem } from "../stores";

  interface Props {
    width: number;
    selectedItem: TimelineItem | null;
    fps: number;
    isRemovingBackground: boolean;
    bgRemovalError: string | null;
    formatTimecode: (frame: number, fps: number) => string;
    updateSelectedItem: (updates: Partial<TimelineItem>) => void;
    updateSelectedTransform: (updates: any) => void;
    hasKeyframeAtCurrentFrame: (property: string) => boolean;
    toggleKeyframe: (property: string, currentValue: number) => void;
    removeBackground: () => void;
  }

  let {
    width,
    selectedItem,
    fps,
    isRemovingBackground,
    bgRemovalError,
    formatTimecode,
    updateSelectedItem,
    updateSelectedTransform,
    hasKeyframeAtCurrentFrame,
    toggleKeyframe,
    removeBackground,
  }: Props = $props();
</script>

<aside class="flex flex-col border-l border-white/5 bg-zinc-900/40 shrink-0" style:width="{width}px">
  <div class="flex items-center px-3 py-2 border-b border-white/5">
    <span class="text-xs font-medium text-white/50 uppercase tracking-wider">Properties</span>
  </div>
  <div class="flex-1 overflow-y-auto p-3">
    {#if selectedItem}
      <div class="space-y-4">
        <div>
          <span class="block text-[10px] text-white/30 uppercase tracking-wider">Name</span>
          <input type="text" class="w-full mt-1 bg-white/5 border border-white/10 rounded px-2 py-1.5 text-xs text-white" value={selectedItem.label} onchange={(e) => updateSelectedItem({ label: e.currentTarget.value })} />
        </div>
        
        <div class="grid grid-cols-2 gap-2">
          <div>
            <span class="block text-[10px] text-white/30 uppercase tracking-wider">Start</span>
            <p class="text-[11px] text-white/70 mt-1 font-mono">{formatTimecode(selectedItem.from, fps)}</p>
          </div>
          <div>
            <span class="block text-[10px] text-white/30 uppercase tracking-wider">Duration</span>
            <p class="text-[11px] text-white/70 mt-1 font-mono">{formatTimecode(selectedItem.durationInFrames, fps)}</p>
          </div>
        </div>

        {#if selectedItem.type === "text"}
          <div class="border-t border-white/5 pt-3">
            <span class="block text-[10px] text-white/30 uppercase tracking-wider mb-2">Text Content</span>
            <textarea class="w-full bg-white/5 border border-white/10 rounded px-2 py-1.5 text-xs text-white" rows="2" value={selectedItem.text} oninput={(e) => updateSelectedItem({ text: e.currentTarget.value })}></textarea>
            
            <div class="grid grid-cols-2 gap-2 mt-3">
              <div>
                <span class="text-[10px] text-white/40 block mb-1">Color</span>
                <div class="flex items-center gap-1.5">
                  <input type="color" class="w-6 h-6 rounded bg-transparent border-0 p-0 cursor-pointer" value={selectedItem.color ?? "#ffffff"} oninput={(e) => updateSelectedItem({ color: e.currentTarget.value })} />
                  <input type="text" class="w-full bg-white/5 border border-white/10 rounded px-1.5 py-1 text-xs text-white font-mono" value={selectedItem.color ?? "#ffffff"} onchange={(e) => updateSelectedItem({ color: e.currentTarget.value })} />
                </div>
              </div>
              <div>
                <span class="text-[10px] text-white/40 block mb-1">Font Size</span>
                <input type="number" class="w-full bg-white/5 border border-white/10 rounded px-2 py-1 text-xs text-white font-mono" value={selectedItem.fontSize ?? 72} onchange={(e) => updateSelectedItem({ fontSize: parseInt(e.currentTarget.value) })} />
              </div>
            </div>
          </div>
        {/if}

        <div class="border-t border-white/5 pt-3">
          <div class="flex items-center justify-between mb-2">
            <span class="block text-[10px] text-white/30 uppercase tracking-wider">Transform</span>
            <button class="text-[9px] text-violet-400 hover:text-violet-300 pointer-events-auto" onclick={() => updateSelectedItem({keyframes: []})} disabled={!selectedItem.keyframes?.length}>Clear All Keyframes</button>
          </div>
          <div class="grid grid-cols-2 gap-3">
            <div>
              <div class="flex items-center justify-between mb-1.5">
                <span class="text-[9px] text-white/40 block font-medium">Position X</span>
                <button class="flex items-center justify-center p-0.5 rounded cursor-pointer transition-colors {hasKeyframeAtCurrentFrame('x') ? 'text-amber-400 bg-amber-400/20 hover:bg-amber-400/30' : 'text-white/20 hover:text-white/50 hover:bg-white/10'}" onclick={() => toggleKeyframe('x', selectedItem?.transform?.x ?? 0)}>
                  <div class="w-1.5 h-1.5 rotate-45 {hasKeyframeAtCurrentFrame('x') ? 'bg-amber-400' : 'border border-current bg-transparent'}" style="border-radius: 1px;"></div>
                </button>
              </div>
              <input type="number" class="w-full bg-white/5 border border-white/10 rounded px-2 py-1 text-xs text-white font-mono focus:border-violet-500/50 focus:outline-none transition-colors" value={selectedItem.transform?.x ?? 0} onchange={(e) => { updateSelectedTransform({ x: parseFloat(e.currentTarget.value) }); if (hasKeyframeAtCurrentFrame('x')) toggleKeyframe('x', parseFloat(e.currentTarget.value)); else { /* manual adjust */ } }} />
            </div>
            <div>
              <div class="flex items-center justify-between mb-1.5">
                <span class="text-[9px] text-white/40 block font-medium">Position Y</span>
                <button class="flex items-center justify-center p-0.5 rounded cursor-pointer transition-colors {hasKeyframeAtCurrentFrame('y') ? 'text-amber-400 bg-amber-400/20 hover:bg-amber-400/30' : 'text-white/20 hover:text-white/50 hover:bg-white/10'}" onclick={() => toggleKeyframe('y', selectedItem?.transform?.y ?? 0)}>
                  <div class="w-1.5 h-1.5 rotate-45 {hasKeyframeAtCurrentFrame('y') ? 'bg-amber-400' : 'border border-current bg-transparent'}" style="border-radius: 1px;"></div>
                </button>
              </div>
              <input type="number" class="w-full bg-white/5 border border-white/10 rounded px-2 py-1 text-xs text-white font-mono focus:border-violet-500/50 focus:outline-none transition-colors" value={selectedItem.transform?.y ?? 0} onchange={(e) => { updateSelectedTransform({ y: parseFloat(e.currentTarget.value) }); if (hasKeyframeAtCurrentFrame('y')) toggleKeyframe('y', parseFloat(e.currentTarget.value)); }} />
            </div>
            <div>
              <div class="flex items-center justify-between mb-1.5">
                <span class="text-[9px] text-white/40 block font-medium">Scale</span>
                <button class="flex items-center justify-center p-0.5 rounded cursor-pointer transition-colors {hasKeyframeAtCurrentFrame('scale') ? 'text-amber-400 bg-amber-400/20 hover:bg-amber-400/30' : 'text-white/20 hover:text-white/50 hover:bg-white/10'}" onclick={() => toggleKeyframe('scale', selectedItem?.transform?.scale ?? 1)}>
                  <div class="w-1.5 h-1.5 rotate-45 {hasKeyframeAtCurrentFrame('scale') ? 'bg-amber-400' : 'border border-current bg-transparent'}" style="border-radius: 1px;"></div>
                </button>
              </div>
              <input type="number" step="0.01" class="w-full bg-white/5 border border-white/10 rounded px-2 py-1 text-xs text-white font-mono focus:border-violet-500/50 focus:outline-none transition-colors" value={selectedItem.transform?.scale ?? 1} onchange={(e) => { updateSelectedTransform({ scale: parseFloat(e.currentTarget.value) }); if (hasKeyframeAtCurrentFrame('scale')) toggleKeyframe('scale', parseFloat(e.currentTarget.value)); }} />
            </div>
            <div>
              <div class="flex items-center justify-between mb-1.5">
                <span class="text-[9px] text-white/40 block font-medium">Rotation (deg)</span>
                <button class="flex items-center justify-center p-0.5 rounded cursor-pointer transition-colors {hasKeyframeAtCurrentFrame('rotation') ? 'text-amber-400 bg-amber-400/20 hover:bg-amber-400/30' : 'text-white/20 hover:text-white/50 hover:bg-white/10'}" onclick={() => toggleKeyframe('rotation', selectedItem?.transform?.rotation ?? 0)}>
                  <div class="w-1.5 h-1.5 rotate-45 {hasKeyframeAtCurrentFrame('rotation') ? 'bg-amber-400' : 'border border-current bg-transparent'}" style="border-radius: 1px;"></div>
                </button>
              </div>
              <input type="number" class="w-full bg-white/5 border border-white/10 rounded px-2 py-1 text-xs text-white font-mono focus:border-violet-500/50 focus:outline-none transition-colors" value={selectedItem.transform?.rotation ?? 0} onchange={(e) => { updateSelectedTransform({ rotation: parseFloat(e.currentTarget.value) }); if (hasKeyframeAtCurrentFrame('rotation')) toggleKeyframe('rotation', parseFloat(e.currentTarget.value)); }} />
            </div>
            <div class="col-span-2">
              <div class="flex items-center justify-between mb-1.5">
                <span class="text-[9px] text-white/40 block font-medium">Opacity (0-1)</span>
                <button class="flex items-center justify-center p-0.5 rounded cursor-pointer transition-colors {hasKeyframeAtCurrentFrame('opacity') ? 'text-amber-400 bg-amber-400/20 hover:bg-amber-400/30' : 'text-white/20 hover:text-white/50 hover:bg-white/10'}" onclick={() => toggleKeyframe('opacity', selectedItem?.transform?.opacity ?? 1)}>
                  <div class="w-1.5 h-1.5 rotate-45 {hasKeyframeAtCurrentFrame('opacity') ? 'bg-amber-400' : 'border border-current bg-transparent'}" style="border-radius: 1px;"></div>
                </button>
              </div>
              <input type="number" step="0.05" min="0" max="1" class="w-full bg-white/5 border border-white/10 rounded px-2 py-1 text-xs text-white font-mono focus:border-violet-500/50 focus:outline-none transition-colors" value={selectedItem.transform?.opacity ?? 1} onchange={(e) => { updateSelectedTransform({ opacity: parseFloat(e.currentTarget.value) }); if (hasKeyframeAtCurrentFrame('opacity')) toggleKeyframe('opacity', parseFloat(e.currentTarget.value)); }} />
            </div>
          </div>
        </div>

        {#if selectedItem.type === "image"}
          <div class="border-t border-white/5 pt-3">
            <span class="block text-[10px] text-white/30 uppercase tracking-wider mb-2">AI Effects</span>
            <button 
              class="w-full flex items-center justify-center gap-2 rounded-lg {isRemovingBackground ? 'bg-amber-600' : 'bg-transparent border border-white/20'} px-3 py-2 text-[11px] text-white transition-colors hover:bg-white/10 shrink-0"
              onclick={removeBackground}
              disabled={isRemovingBackground}
            >
              {#if isRemovingBackground}
                <div class="w-3 h-3 border-2 border-white/30 border-t-white rounded-full animate-spin"></div> Processing...
              {:else}
                ✨ Auto Remove Background
              {/if}
            </button>
            {#if bgRemovalError}
              <p class="text-[10px] text-red-400 mt-2">{bgRemovalError}</p>
            {/if}
          </div>
        {/if}

      </div>
    {:else}
      <div class="flex flex-col items-center justify-center h-full text-white/15 text-xs gap-2">
        <Settings class="w-8 h-8" />
        <p>Select a clip to inspect</p>
      </div>
    {/if}
  </div>
</aside>
