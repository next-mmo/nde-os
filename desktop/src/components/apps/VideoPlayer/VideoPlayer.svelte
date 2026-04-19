<svelte:options runes={true} />

<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { convertFileSrc, invoke } from "@tauri-apps/api/core";
  import { Button } from "$lib/components/ui/button";
  import { ScrollArea } from "$lib/components/ui/scroll-area";
  import { Badge } from "$lib/components/ui/badge";
  import {
    Play, Pause, SkipBack, SkipForward, Volume2, VolumeX,
    Maximize, Minimize, FolderOpen, Film, ChevronLeft, ChevronRight,
    PictureInPicture2, MonitorSmartphone, LayoutGrid, RotateCcw
  } from "@lucide/svelte";
  import type { DesktopWindow } from "🍎/state/desktop.svelte";

  interface Props {
    window: DesktopWindow;
  }

  let { window: desktopWindow }: Props = $props();

  // ── State ──────────────────────────────────────────────────────────────
  let videoEl = $state<HTMLVideoElement | null>(null);
  let isPlaying = $state(false);
  let currentTime = $state(0);
  let duration = $state(0);
  let volume = $state(1);
  let isMuted = $state(false);
  let isFullscreen = $state(false);
  let showControls = $state(true);
  let controlsTimeout: ReturnType<typeof setTimeout>;
  let isSeeking = $state(false);
  let playbackRate = $state(1);
  let showPlaylist = $state(true);

  // Data from window.data (passed via openStaticApp)
  // Supports: { filePath: string } or { playlist: { title, items: [{ path, title, index }] }, startIndex?: number }
  let windowData = $derived(desktopWindow.data as any);
  let videoError = $state<string | null>(null);

  // Build playlist from window data
  let playlist = $derived.by(() => {
    const data = windowData;
    if (!data) return [];
    if (data.playlist?.items) {
      return data.playlist.items.map((item: any, i: number) => ({
        path: item.path || item.output_path,
        title: item.title || `Episode ${item.index || i + 1}`,
        index: item.index || i + 1,
      }));
    }
    if (data.filePath) {
      const name = data.filePath.split(/[/\\]/).pop() || "Video";
      return [{ path: data.filePath, title: name, index: 1 }];
    }
    return [];
  });

  let currentIndex = $state(0);
  let currentItem = $derived(playlist[currentIndex] || null);
  let videoSrc = $derived(currentItem ? convertFileSrc(currentItem.path) : "");

  // ── Gallery state ────────────────────────────────────────────────────────
  let galleryVideos = $state<{ name: string; path: string; size: number }[]>([]);
  let isLoadingGallery = $state(false);

  $effect(() => {
    if (playlist.length === 0 && galleryVideos.length === 0 && !isLoadingGallery) {
      loadGallery();
    }
  });

  async function loadGallery() {
    isLoadingGallery = true;
    try {
      galleryVideos = await invoke("scan_videos");
    } catch (e) {
      console.error("Failed to scan videos", e);
    } finally {
      isLoadingGallery = false;
    }
  }

  function playGalleryItem(item: { name: string; path: string }) {
    desktopWindow.data = { filePath: item.path };
  }

  // ── Mini Player ─────────────────────────────────────────────────────────

  function toggleMiniPlayer() {
    if (desktopWindow.width <= 400) {
      // Restore large
      desktopWindow.width = 1120;
      desktopWindow.height = 720;
    } else {
      // Small vertical size for dramas
      desktopWindow.width = 360;
      desktopWindow.height = 640;
    }
  }

  function togglePiP() {
    if (!videoEl) return;
    if (document.pictureInPictureElement) {
      document.exitPictureInPicture().catch(console.error);
    } else if (document.pictureInPictureEnabled) {
      videoEl.requestPictureInPicture().catch(console.error);
    }
  }

  // When window data changes (e.g. re-opened with new playlist), reset index
  let lastDataId = $state("");
  $effect(() => {
    const data = windowData;
    if (!data) return;
    const newId = JSON.stringify(data.playlist?.title ?? data.filePath ?? "");
    if (newId !== lastDataId) {
      lastDataId = newId;
      currentIndex = data?.startIndex ?? 0;
      videoError = null;
    }
  });

  // Update window title
  $effect(() => {
    if (currentItem) {
      desktopWindow.title = currentItem.title;
    }
  });

  // Auto-load video when src changes
  $effect(() => {
    const src = videoSrc;
    const el = videoEl;
    if (el && src) {
      videoError = null;
      // Force reload when src changes reactively
      el.load();
    }
  });

  // ── Video event handlers ───────────────────────────────────────────────
  function onTimeUpdate() {
    if (!videoEl || isSeeking) return;
    currentTime = videoEl.currentTime;
  }

  function onLoadedMetadata() {
    if (!videoEl) return;
    duration = videoEl.duration;
  }

  function onVideoError() {
    if (!videoEl) return;
    const err = videoEl.error;
    videoError = err ? `Error ${err.code}: ${err.message || "Cannot play this video"}` : "Unknown video error";
    console.error("[VideoPlayer] Video error:", videoError, "src:", videoSrc);
  }

  function handleLeavePiP() {
    // Chromium WebView2 bug: exiting PiP sometimes leaves the video surface detached/invisible.
    // Toggling the display CSS forces the compositor to redraw the video surface.
    if (videoEl) {
      const orig = videoEl.style.display;
      videoEl.style.display = 'none';
      setTimeout(() => { 
        if (videoEl) videoEl.style.display = orig; 
      }, 50);
    }
  }

  onDestroy(() => {
    if (videoEl) {
      videoEl.pause();
      videoEl.removeAttribute('src');
      videoEl.load();
    }
  });

  function onPlay() { isPlaying = true; }
  function onPause() { isPlaying = false; }
  function onEnded() {
    isPlaying = false;
    // Auto-advance to next in playlist
    if (currentIndex < playlist.length - 1) {
      currentIndex += 1;
      tick().then(() => videoEl?.play().catch(() => {}));
    }
  }

  // ── Controls ───────────────────────────────────────────────────────────
  function togglePlay() {
    if (!videoEl) return;
    if (videoEl.paused) {
      videoEl.play().catch(() => {});
    } else {
      videoEl.pause();
    }
  }

  function seek(e: Event) {
    const target = e.target as HTMLInputElement;
    const time = parseFloat(target.value);
    if (videoEl) {
      videoEl.currentTime = time;
      currentTime = time;
    }
  }

  function seekStart() { isSeeking = true; }
  function seekEnd() { isSeeking = false; }

  function setVolume(e: Event) {
    const target = e.target as HTMLInputElement;
    volume = parseFloat(target.value);
    if (videoEl) {
      videoEl.volume = volume;
      isMuted = volume === 0;
    }
  }

  function toggleMute() {
    if (!videoEl) return;
    isMuted = !isMuted;
    videoEl.muted = isMuted;
  }

  function skip(seconds: number) {
    if (!videoEl) return;
    videoEl.currentTime = Math.max(0, Math.min(videoEl.currentTime + seconds, duration));
  }

  function cycleSpeed() {
    const rates = [0.5, 0.75, 1, 1.25, 1.5, 2];
    const idx = rates.indexOf(playbackRate);
    playbackRate = rates[(idx + 1) % rates.length];
    if (videoEl) videoEl.playbackRate = playbackRate;
  }

  function goToItem(index: number) {
    if (index < 0 || index >= playlist.length) return;
    currentIndex = index;
    tick().then(() => videoEl?.play().catch(() => {}));
  }

  function prevItem() { goToItem(currentIndex - 1); }
  function nextItem() { goToItem(currentIndex + 1); }

  // Controls auto-hide
  function onMouseMove() {
    showControls = true;
    clearTimeout(controlsTimeout);
    if (isPlaying) {
      controlsTimeout = setTimeout(() => { showControls = false; }, 3000);
    }
  }

  function onMouseLeave() {
    if (isPlaying) {
      controlsTimeout = setTimeout(() => { showControls = false; }, 1500);
    }
  }

  // Keyboard shortcuts
  function onKeyDown(e: KeyboardEvent) {
    if (e.target instanceof HTMLInputElement) return;
    switch (e.key) {
      case " ":
      case "k":
        e.preventDefault();
        togglePlay();
        break;
      case "ArrowLeft":
        e.preventDefault();
        skip(e.shiftKey ? -30 : -10);
        break;
      case "ArrowRight":
        e.preventDefault();
        skip(e.shiftKey ? 30 : 10);
        break;
      case "ArrowUp":
        e.preventDefault();
        volume = Math.min(1, volume + 0.1);
        if (videoEl) videoEl.volume = volume;
        break;
      case "ArrowDown":
        e.preventDefault();
        volume = Math.max(0, volume - 0.1);
        if (videoEl) videoEl.volume = volume;
        break;
      case "m":
        toggleMute();
        break;
      case "f":
        isFullscreen = !isFullscreen;
        break;
      case "n":
        nextItem();
        break;
      case "p":
        prevItem();
        break;
      case ">":
      case ".":
        cycleSpeed();
        break;
    }
  }

  // ── Formatting ─────────────────────────────────────────────────────────
  function formatTime(seconds: number): string {
    if (!isFinite(seconds) || isNaN(seconds)) return "0:00";
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = Math.floor(seconds % 60);
    if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
    return `${m}:${String(s).padStart(2, "0")}`;
  }

  let progressPercent = $derived(duration > 0 ? (currentTime / duration) * 100 : 0);

  onMount(() => {
    document.addEventListener("keydown", onKeyDown);
  });

  onDestroy(() => {
    document.removeEventListener("keydown", onKeyDown);
    clearTimeout(controlsTimeout);
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="h-full w-full flex bg-black text-white overflow-hidden select-none"
  onmousemove={onMouseMove}
  onmouseleave={onMouseLeave}
>
  <!-- Sidebar playlist (collapsible) -->
  {#if playlist.length > 1 && showPlaylist}
    <div class="w-64 flex-shrink-0 bg-zinc-950 border-r border-white/5 flex flex-col overflow-hidden">
      <div class="px-3 py-2.5 border-b border-white/5 flex items-center justify-between">
        <span class="text-xs font-semibold text-white/70 uppercase tracking-wider">Playlist</span>
        <button
          class="text-white/40 hover:text-white/80 transition-colors p-1"
          onclick={() => showPlaylist = false}
          title="Hide playlist"
        >
          <ChevronLeft class="w-4 h-4" />
        </button>
      </div>
      <ScrollArea class="flex-1">
        <div class="flex flex-col py-1">
          {#each playlist as item, i}
            <button
              class="flex items-center gap-2.5 px-3 py-2 text-left transition-colors cursor-pointer
                     {i === currentIndex
                       ? 'bg-white/10 text-white'
                       : 'text-white/50 hover:text-white/80 hover:bg-white/5'}"
              onclick={() => goToItem(i)}
            >
              <span class="text-xs font-mono w-6 text-right flex-shrink-0
                           {i === currentIndex ? 'text-primary' : 'text-white/30'}">
                {#if i === currentIndex && isPlaying}
                  <span class="inline-flex gap-[2px] items-end h-3">
                    <span class="w-[2px] bg-primary animate-pulse" style="height: 8px"></span>
                    <span class="w-[2px] bg-primary animate-pulse" style="height: 12px; animation-delay: 0.15s"></span>
                    <span class="w-[2px] bg-primary animate-pulse" style="height: 6px; animation-delay: 0.3s"></span>
                  </span>
                {:else}
                  {item.index}
                {/if}
              </span>
              <span class="text-sm truncate flex-1">{item.title}</span>
            </button>
          {/each}
        </div>
      </ScrollArea>
    </div>
  {/if}

  <!-- Main video area -->
  <div class="flex-1 flex flex-col relative overflow-hidden">
    {#if !currentItem}
      <!-- Gallery view -->
      <div class="flex-1 flex flex-col p-6 overflow-hidden bg-neutral-950">
        <div class="flex items-center justify-between mb-6 shrink-0">
          <div class="flex items-center gap-3">
            <Film class="w-6 h-6 text-indigo-400" />
            <h2 class="text-2xl font-bold font-display tracking-tight text-neutral-100">Video Gallery</h2>
          </div>
          <Button variant="outline" size="sm" class="bg-white/5 border-white/10 hover:bg-white/10 text-white/80 cursor-pointer" onclick={loadGallery}>
            <RotateCcw class="w-4 h-4 mr-2 {isLoadingGallery ? 'animate-spin' : ''}" />
            Refresh
          </Button>
        </div>
        
        <ScrollArea class="flex-1 -mx-2 px-2" viewportClasses="pe-4">
          {#if isLoadingGallery && galleryVideos.length === 0}
            <div class="flex justify-center items-center h-48 text-neutral-500">Scanning for videos...</div>
          {:else if galleryVideos.length === 0}
            <div class="flex flex-col items-center justify-center text-white/30 h-64 gap-4">
              <FolderOpen class="w-12 h-12" />
              <p>No videos found in your workspace.</p>
            </div>
          {:else}
            <div class="grid grid-cols-2 md:grid-cols-3 xl:grid-cols-4 gap-4 pb-8">
              {#each galleryVideos as vid}
                <button
                  class="flex flex-col text-left group overflow-hidden rounded-xl bg-white/5 border border-white/10 hover:bg-white/10 transition-all cursor-pointer focus:outline-none focus:ring-2 focus:ring-indigo-500/50"
                  onclick={() => playGalleryItem(vid)}
                >
                  <div class="w-full aspect-video bg-black flex items-center justify-center relative overflow-hidden group-hover:bg-neutral-900 transition-colors">
                    <!-- svelte-ignore a11y_media_has_caption -->
                    <video 
                      src="{convertFileSrc(vid.path)}#t=1.5" 
                      class="absolute inset-0 w-full h-full object-cover opacity-70 group-hover:opacity-100 group-hover:scale-105 transition-all duration-500 pointer-events-none"
                      preload="metadata"
                      muted
                      playsinline
                    ></video>
                    
                    <div class="absolute inset-0 bg-black/30 group-hover:bg-transparent transition-colors z-0"></div>
                    <Play class="w-10 h-10 text-white opacity-0 group-hover:opacity-100 transition-opacity drop-shadow-xl z-10 drop-shadow-[0_0_8px_rgba(0,0,0,0.8)]" />
                  </div>
                  <div class="p-3">
                    <p class="text-sm font-medium text-neutral-200 line-clamp-2 leading-snug" title={vid.name}>
                      {vid.name}
                    </p>
                    <p class="text-xs text-neutral-500 mt-1 uppercase">{(vid.size / (1024 * 1024)).toFixed(1)} MB</p>
                  </div>
                </button>
              {/each}
            </div>
          {/if}
        </ScrollArea>
      </div>
    {:else}
      <!-- Video element -->
      <!-- svelte-ignore a11y_media_has_caption -->
      <video
        bind:this={videoEl}
        src={videoSrc}
        class="absolute inset-0 w-full h-full object-contain bg-black cursor-pointer"
        onclick={togglePlay}
        ondblclick={() => isFullscreen = !isFullscreen}
        ontimeupdate={onTimeUpdate}
        onloadedmetadata={onLoadedMetadata}
        onplay={onPlay}
        onpause={onPause}
        onended={onEnded}
        onerror={onVideoError}
        onleavepictureinpicture={handleLeavePiP}
        preload="metadata"
        playsinline
      ></video>

      <!-- Video error overlay -->
      {#if videoError}
        <div class="absolute inset-0 flex flex-col items-center justify-center bg-black/80 text-white/70 gap-3">
          <Film class="w-12 h-12 text-red-400" />
          <p class="text-sm font-medium text-red-400">Failed to load video</p>
          <p class="text-xs text-white/40 max-w-xs text-center">{videoError}</p>
          <p class="text-[10px] text-white/20 max-w-md text-center font-mono break-all mt-2">{currentItem?.path}</p>
        </div>
      {/if}

      <!-- Show playlist button (when hidden) -->
      <!-- Big play button overlay when paused -->
      {#if !isPlaying && duration > 0}
        <button
          class="absolute inset-0 flex items-center justify-center bg-black/20 transition-opacity cursor-pointer"
          onclick={togglePlay}
        >
          <div class="w-20 h-20 rounded-full bg-white/15 backdrop-blur-md flex items-center justify-center
                      hover:bg-white/25 transition-colors shadow-2xl">
            <Play class="w-10 h-10 text-white ml-1" />
          </div>
        </button>
      {/if}

      <!-- Controls overlay (bottom) -->
      <div
        class="absolute bottom-0 left-0 right-0 transition-all duration-300
               {showControls || !isPlaying ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-2 pointer-events-none'}"
      >
        <!-- Gradient backdrop -->
        <div class="absolute inset-0 bg-gradient-to-t from-black/90 via-black/50 to-transparent pointer-events-none"></div>

        <div class="relative px-4 pb-3 pt-8 flex flex-col gap-2">
          <!-- Progress bar -->
          <div class="group flex items-center gap-3">
            <span class="text-[11px] font-mono text-white/60 w-12 text-right">{formatTime(currentTime)}</span>
            <div class="flex-1 relative h-1 group-hover:h-1.5 transition-all rounded-full bg-white/20 cursor-pointer">
              <div
                class="absolute left-0 top-0 h-full bg-primary rounded-full transition-[width]"
                style="width: {progressPercent}%"
              ></div>
              <input
                type="range"
                min="0"
                max={duration || 0}
                step="0.1"
                value={currentTime}
                oninput={seek}
                onmousedown={seekStart}
                onmouseup={seekEnd}
                class="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
              />
              <div
                class="absolute top-1/2 -translate-y-1/2 w-3 h-3 bg-primary rounded-full shadow-lg
                       opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none"
                style="left: calc({progressPercent}% - 6px)"
              ></div>
            </div>
            <span class="text-[11px] font-mono text-white/60 w-12">{formatTime(duration)}</span>
          </div>

          <!-- Control buttons -->
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-1">
              <!-- Previous -->
              {#if playlist.length > 1}
                <button
                  class="p-2 text-white/70 hover:text-white transition-colors disabled:opacity-30"
                  onclick={prevItem}
                  disabled={currentIndex === 0}
                  title="Previous (P)"
                >
                  <SkipBack class="w-4 h-4" />
                </button>
              {/if}

              <!-- Play/Pause -->
              <button
                class="p-2 text-white hover:text-primary transition-colors"
                onclick={togglePlay}
                title={isPlaying ? "Pause (Space)" : "Play (Space)"}
              >
                {#if isPlaying}
                  <Pause class="w-5 h-5" />
                {:else}
                  <Play class="w-5 h-5" />
                {/if}
              </button>

              <!-- Next -->
              {#if playlist.length > 1}
                <button
                  class="p-2 text-white/70 hover:text-white transition-colors disabled:opacity-30"
                  onclick={nextItem}
                  disabled={currentIndex >= playlist.length - 1}
                  title="Next (N)"
                >
                  <SkipForward class="w-4 h-4" />
                </button>
              {/if}

              <!-- Skip buttons -->
              <button
                class="p-1.5 text-white/50 hover:text-white/80 transition-colors text-xs font-semibold"
                onclick={() => skip(-10)}
                title="Back 10s (←)"
              >
                -10s
              </button>
              <button
                class="p-1.5 text-white/50 hover:text-white/80 transition-colors text-xs font-semibold"
                onclick={() => skip(10)}
                title="Forward 10s (→)"
              >
                +10s
              </button>

              <!-- Volume -->
              <div class="flex items-center gap-1 ml-2">
                <button
                  class="p-1.5 text-white/60 hover:text-white transition-colors"
                  onclick={toggleMute}
                  title="Mute (M)"
                >
                  {#if isMuted || volume === 0}
                    <VolumeX class="w-4 h-4" />
                  {:else}
                    <Volume2 class="w-4 h-4" />
                  {/if}
                </button>
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.05"
                  value={isMuted ? 0 : volume}
                  oninput={setVolume}
                  class="w-20 h-1 rounded-full appearance-none bg-white/20
                         [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-2.5
                         [&::-webkit-slider-thumb]:h-2.5 [&::-webkit-slider-thumb]:rounded-full
                         [&::-webkit-slider-thumb]:bg-white [&::-webkit-slider-thumb]:cursor-pointer
                         cursor-pointer"
                />
              </div>
            </div>

            <div class="flex items-center gap-2">
              <!-- Playback speed -->
              <button
                class="px-2 py-1 text-[11px] font-semibold rounded text-white/60 hover:text-white hover:bg-white/10 transition-colors"
                onclick={cycleSpeed}
                title="Playback speed (.)"
              >
                {playbackRate}x
              </button>

              <!-- Mini Player / PiP -->
              <button
                class="p-1.5 text-white/60 hover:text-white hover:bg-white/10 rounded transition-colors"
                onclick={toggleMiniPlayer}
                title="Mini Window"
              >
                <MonitorSmartphone class="w-4 h-4" />
              </button>

              <button
                class="p-1.5 text-white/60 hover:text-white hover:bg-white/10 rounded transition-colors"
                onclick={togglePiP}
                title="Picture in Picture"
              >
                <PictureInPicture2 class="w-4 h-4" />
              </button>

              <!-- Episode indicator -->
              {#if playlist.length > 1}
                <span class="text-[11px] text-white/40 font-mono">
                  {currentIndex + 1}/{playlist.length}
                </span>
              {/if}
            </div>
          </div>
        </div>
      </div>

      <!-- Top controls overlay -->
      <div 
        class="absolute top-0 left-0 right-0 p-3 pt-4 flex items-start gap-3 transition-opacity duration-300
               bg-gradient-to-b from-black/80 to-transparent pointer-events-none {showControls ? 'opacity-100' : 'opacity-0'}"
      >
        <button
          class="shrink-0 bg-white/10 hover:bg-white/20 backdrop-blur-md text-white/80 hover:text-white 
                 p-1.5 rounded-lg transition-all pointer-events-auto cursor-pointer"
          onclick={() => { 
            if (videoEl) {
                videoEl.pause();
            }
            if (document.pictureInPictureElement) {
                document.exitPictureInPicture().catch(() => {});
            }
            desktopWindow.data = null; 
            if (desktopWindow.width <= 400) {
              desktopWindow.width = 1120;
              desktopWindow.height = 720;
            }
          }}
          title="Back to Gallery"
        >
          <LayoutGrid class="w-4 h-4" />
        </button>

        {#if playlist.length > 1 && !showPlaylist}
          <button
            class="shrink-0 bg-white/10 hover:bg-white/20 backdrop-blur-md text-white/80 hover:text-white 
                   p-1.5 rounded-lg transition-all pointer-events-auto cursor-pointer"
            onclick={() => showPlaylist = true}
            title="Show playlist"
          >
            <ChevronRight class="w-4 h-4" />
          </button>
        {/if}

        <div class="flex-1 overflow-hidden min-w-0">
          <p class="text-sm font-medium text-white/90 truncate drop-shadow-md">{currentItem.title}</p>
          {#if playlist.length > 1}
            <p class="text-[11px] text-white/50 mt-0.5 drop-shadow-md font-mono">
              Episode {currentIndex + 1}/{playlist.length}
            </p>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>
