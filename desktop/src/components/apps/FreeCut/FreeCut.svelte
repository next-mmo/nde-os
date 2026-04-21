<svelte:options runes={true} />

<script lang="ts">
  import { invoke, convertFileSrc } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { onMount, onDestroy } from "svelte";
  import { openStaticApp } from "🍎/state/desktop.svelte";
  import {
    Film, Play, Pause, Square, SkipBack, SkipForward, Plus, Save, Upload,
    Scissors, Trash2, Volume2, VolumeX, Eye, EyeOff, Lock, Unlock,
    Settings, ZoomIn, ZoomOut, Layers, Type, Image, Music, Video,
    ChevronRight, Repeat, PanelLeftClose, PanelRightClose, PanelLeft, PanelRight,
    MousePointer2, Slice, GripHorizontal, FolderOpen, Download, Sparkles,
    Circle, Triangle, RectangleHorizontal, Undo2, Redo2, Grid,
    Magnet, Maximize2, Minus, GripVertical, Check, AlertCircle, File,
    Captions
  } from "@lucide/svelte";

  // ─── Stores (Zustand vanilla → Svelte 5 reactive) ─────────────────────
  import { useStore } from "./lib/use-store.svelte";
  import { playbackStore } from "./stores/playback";
  import { selectionStore } from "./stores/selection";
  import { editorStore } from "./stores/editor";
  import { itemsStore } from "./stores/items";
  import { zoomStore } from "./stores/zoom";
  import { historyStore } from "./stores/history";
  import type { MediaItem, Project, DubbingSession, DubbingSpeaker, DubbingSegment, DubbingRvcConfig, DubbingLlmConfig, DubbingToolReport, DubbingImportResult, DubbingProgress, DubbingRuntimeInstallResult, WhisperSettings, DubStudioJob, DubStudioPart, ProjectSummary, Marker } from "./types";

  // ─── Extracted panel components ──────────────────────────────────────
  import ToolsPanel from "./panels/ToolsPanel.svelte";
  import ProjectsView from "./panels/ProjectsView.svelte";
  import ExportModal from "./panels/ExportModal.svelte";
  import PropertiesPanel from "./panels/PropertiesPanel.svelte";
  import DubbingPanel from "./panels/DubbingPanel.svelte";
  import FileExplorer from "../FileExplorer/FileExplorer.svelte";

  import type { TimelineItem, TimelineTrack, EditorTab, ActiveTool } from "./stores";

  // ─── Reactive state from stores ────────────────────────────────────────
  const pb = useStore(playbackStore);
  const sel = useStore(selectionStore);
  const ed = useStore(editorStore);
  const ti = useStore(itemsStore);
  const zm = useStore(zoomStore);
  const hist = useStore(historyStore);

  // Derive primitive value for safer template bindings and cross-branch reactivity in Svelte 5
  let activeTab = $derived(ed.activeTab);

  // ─── Local state ─────────────────────────────────────────────────────

  let currentView = $state<"projects" | "editor">("projects");
  let projects = $state<ProjectSummary[]>([]);
  let currentProject = $state<Project | null>(null);
  let mediaLibrary = $state<MediaItem[]>([]);
  let isLoading = $state(false);
  let renderedFramePath = $state<string | null>(null);


  // Export state
  let showExportModal = $state(false);
  let isExporting = $state(false);
  let exportProgress = $state(0);
  let exportError = $state<string | null>(null);
  let exportQuality = $state<"low" | "medium" | "high" | "ultra">("high");
  let exportCodec = $state("h264");
  let hwEncoders = $state<{ name: string; codec: string; device: string }[]>([]);
  let exportHwAccel = $state<string | null>(null);

  // New Project Modal
  let showNewProjectModal = $state(false);
  let newProjectName = $state("");
  const ratioPresets = [
    { label: "16:9", icon: "▬", w: 1920, h: 1080, desc: "YouTube / Landscape" },
    { label: "9:16", icon: "▮", w: 1080, h: 1920, desc: "TikTok / Reels" },
    { label: "1:1", icon: "■", w: 1080, h: 1080, desc: "Instagram Post" },
    { label: "4:5", icon: "▯", w: 1080, h: 1350, desc: "Instagram Portrait" },
    { label: "4:3", icon: "▭", w: 1440, h: 1080, desc: "Classic" },
    { label: "21:9", icon: "━", w: 2560, h: 1080, desc: "Ultrawide / Cinema" },
  ] as const;
  let selectedRatioIdx = $state(0);
  let selectedRatio = $derived(ratioPresets[selectedRatioIdx]!);

  // AI Background Removal properties
  let isRemovingBackground = $state(false);
  let bgRemovalError = $state<string | null>(null);

  // Pointer-based media-to-timeline drag (HTML5 DragEvent is unreliable in WebView2)
  let pendingDragPayload: { source: string; media?: MediaItem } | null = null;
  let isPointerDragging = $state(false);
  let pointerDragGhostX = $state(0);
  let pointerDragGhostY = $state(0);

  // Timeline interaction
  let isDraggingClip = $state(false);
  let dragClipId = $state<string | null>(null);
  let dragStartX = $state(0);
  let dragStartY = $state(0);
  let dragStartFrom = $state(0);
  let dragStartTrackId = $state("");

  // Trim interaction
  let isTrimming = $state(false);

  // Playhead drag
  let isDraggingPlayhead = $state(false);

  // Timeline resize drag
  let isResizingTimeline = $state(false);
  let resizeStartY = $state(0);
  let resizeStartHeight = $state(0);
  let isResizingLeftSidebar = $state(false);
  let leftSidebarResizeStartX = $state(0);
  let leftSidebarResizeStartWidth = $state(0);

  // Context menu for media items
  let contextMenuMedia = $state<{
    x: number;
    y: number;
    media: MediaItem;
  } | null>(null);

  function closeContextMenu() {
    contextMenuMedia = null;
  }

  // NDE Media Import Modal state
  let showNdeExplorerModal = $state(false);

  // Preview Tabs
  let activePreviewTab = $state<"timeline" | "media">("timeline");
  let previewMediaItem = $state<MediaItem | null>(null);

  $effect(() => {
    // Auto-switch to timeline when playback starts
    if (pb.isPlaying && activePreviewTab === "media") {
      activePreviewTab = "timeline";
    }
  });

  // Snap visual feedback
  let activeSnapFrame = $state<number | null>(null);

  // Clipboard for copy/paste
  let clipboardItems: TimelineItem[] = [];

  // In/Out points
  let inPoint = $state<number | null>(null);
  let outPoint = $state<number | null>(null);

  // Toast notification for saving
  let toastMessage = $state<string | null>(null);
  let toastTimer: ReturnType<typeof setTimeout> | null = null;

  // ─── Dirty state ───────────────────────────────────────────────────
  let isDirty = $state(false);
  let showUnsavedModal = $state(false);
  let pendingGoBack = $state(false);


  // Markers
  // Markers (type imported from ./types)
  let markers = $state<Marker[]>([]);

  // Preview scrubber (ghost playhead on hover)
  let previewFrame = $state<number | null>(null);

  // Playback preview — DOM overlays for smooth real-time playback
  let activePreviewItems = $state<TimelineItem[]>([]);
  let audioSyncCounter = 0;
  let lastPlaybackFrame = 0;

  // Derived
  let totalFrames = $derived(currentProject?.duration || ti.maxItemEndFrame || 1);
  let fps = $derived(currentProject?.metadata.fps ?? ti.fps);
  let currentTime = $derived(formatTimecode(pb.currentFrame, fps));
  let totalTime = $derived(formatTimecode(totalFrames, fps));

  // Selected item
  let selectedItem = $derived.by(() => {
    if (sel.selectedItemIds.length === 1) {
      return ti.itemById[sel.selectedItemIds[0]!] ?? null;
    }
    return null;
  });
  let dubbingSession = $derived(currentProject?.dubbing ?? createDefaultDubbingSession());

  function createDefaultDubbingSession(partial: Partial<DubbingSession> = {}): DubbingSession {
    return {
      sourceMediaId: null,
      sourceMediaPath: null,
      sourceLanguage: "auto",
      targetLanguage: "en",
      ingestMode: "srt",
      importedSrtPath: null,
      outputDir: null,
      notes: null,
      segments: [],
      speakers: [{
        id: "speaker-narrator",
        label: "Narrator",
        voice: "en-US-AriaNeural",
        rate: "+0%",
        pitch: null,
        volume: null,
        rvc: {
          enabled: false,
          pythonPath: null,
          cliPath: null,
          modelPath: null,
          indexPath: null,
          pitchShift: 0,
        },
      }],
      llm: {
        enabled: false,
        model: null,
        mode: "translate",
      },
      updatedAt: null,
      lastGeneratedAt: null,
      ...partial,
    };
  }

  function updateCurrentProjectDubbing(updater: (session: DubbingSession) => DubbingSession) {
    if (!currentProject) return;
    const next = updater(currentProject.dubbing ?? createDefaultDubbingSession());
    currentProject = { ...currentProject, dubbing: next };
  }

  function patchCurrentProjectDubbing(patch: Partial<DubbingSession>) {
    updateCurrentProjectDubbing((session) => ({ ...session, ...patch }));
  }

  // ─── Lifecycle ────────────────────────────────────────────────────────
  let unlisten: Array<() => void> = [];

  const LAST_PROJECT_KEY = "last_project_id";

  // Subscribe to itemsStore to detect changes → mark dirty
  const unsubItems = itemsStore.subscribe(() => {
    if (!currentProject) return;
    isDirty = true;
  });

  onMount(async () => {
    await loadProjects();

    // Restore the last open project after a hard reload so that the
    // imported media library (fetched via freecut_list_media on openProject)
    // is immediately available without requiring the user to re-click.
    const lastId: string | null = await invoke("freecut_get_setting", { key: LAST_PROJECT_KEY }).catch(() => null);
    if (lastId && projects.some((p) => p.id === lastId)) {
      await openProject(lastId);
    }


    unlisten.push(
      await listen("freecut://media-imported", (event: any) => {
        const { media } = event.payload;
        mediaLibrary = [...mediaLibrary, media];
      }),
      await listen("freecut://thumbnails-ready", (event: any) => {
        const { mediaId, thumbnailPaths } = event.payload;
        mediaLibrary = mediaLibrary.map((m) =>
          m.id === mediaId ? { ...m, thumbnailPath: thumbnailPaths[0] } : m
        );
      }),
      await listen("freecut://waveform-ready", (event: any) => {
        const { mediaId, peaks } = event.payload;
        const items = itemsStore.getState().items;
        for (const item of items) {
          if (item.mediaId === mediaId) {
            itemsStore.getState().updateItem(item.id, { waveformData: peaks });
          }
        }
      }),
      await listen("freecut://frame-rendered", (event: any) => {
        const { bitmapPath } = event.payload;
        // Always accept the latest rendered frame — during playback the playhead
        // moves faster than FFmpeg can render, so demanding exact frame match
        // would cause the preview to never update.
        if (bitmapPath) {
          renderedFramePath = bitmapPath;
          renderEpoch++;
        }
      }),
      await listen("freecut://export-progress", (event: any) => {
        exportProgress = event.payload.percent ?? 0;
      }),
      await listen("freecut://export-complete", (event: any) => {
        isExporting = false;
        if (event.payload.success) {
          exportProgress = 100;
          showExportModal = false;
        } else {
          exportError = event.payload.error ?? "Export failed";
        }
      }),

    );
  });

  onDestroy(() => {
    unlisten.forEach((fn) => fn());
    unsubItems();
  });

  // ─── Playback engine (pure vanilla — zero Svelte reactivity) ─────────
  // Uses a vanilla zustand subscription instead of $effect to ensure
  // the rAF loop can NEVER be killed by unrelated reactive changes.
  //
  // TIMING MODEL: the rAF tick is a RENDERER, not a clock. It reads the
  // currently-playing media element's `currentTime` to derive the exact
  // timeline frame. This eliminates the dual-clock drift that caused the
  // playhead to run ahead of the decoded video (and the periodic "seeking"
  // stutter caused by the old drift-correction seeks).
  //
  // Only when no media element is active (e.g. the playhead is over an
  // image clip or empty region) do we fall back to wall-clock timing.
  let animFrame: number | null = null;
  let isPlayingRaw = $state(false);

  type PlaybackClock = {
    startTime: number;     // wall-clock anchor
    startFrame: number;    // timeline frame at startTime
    rate: number;          // timelineRate snapshot
    fps: number;
    pps: number;
    maxFrames: number;
    rulerPH: HTMLElement | null;
    trackPHs: NodeListOf<Element>;
    tcPreview: HTMLElement | null;
    tcRuler: HTMLElement | null;
    lastIntFrame: number;
    lastAudioSyncFrame: number;
    lastVideoCheckFrame: number;
  };

  let _clock: PlaybackClock | null = null;

  /** Rebase the wall-clock anchor so that `frame` corresponds to `now`. */
  function rebaseClock(frame: number, now: number) {
    if (!_clock) return;
    _clock.startTime = now;
    _clock.startFrame = frame;
  }

  /**
   * Compute the current exact timeline frame by reading the active
   * media element's clock (video preferred, then active audio element).
   *
   * Priority:
   *   1. Active <video>.currentTime (if ready & playing)
   *   2. First active <audio>.currentTime (if ready & playing)
   *   3. If a media item exists at this frame but isn't ready yet:
   *      HOLD the playhead at startFrame (avoids the "playhead jumps
   *      back" glitch when video finishes loading 200ms into playback)
   *   4. Otherwise (image clip / empty region): wall-clock timing
   */
  function computeExactFrame(now: number): number {
    if (!_clock) return 0;
    const { startTime, startFrame, rate, fps } = _clock;

    // 1. Video-driven: use the topmost video's currentTime as master clock.
    const topItem = [...activePreviewItems].reverse().find(i => i.type === 'video');
    if (topItem) {
      const vid = document.querySelector(`video[data-preview-video="${topItem.id}"]`) as HTMLVideoElement | null;
      if (vid && !vid.paused && !vid.seeking && vid.readyState >= 2) {
        const sourceFps = topItem.sourceFps ?? fps;
        const speed = Math.abs(topItem.speed ?? 1) || 1;
        const sourceStartFrame = topItem.sourceStart ?? 0;
        const sourceFrames = vid.currentTime * sourceFps;
        const timelineFramesInClip = (sourceFrames - sourceStartFrame) / speed;
        const frame = topItem.from + timelineFramesInClip;
        if (Number.isFinite(frame) && frame >= topItem.from - 2 && frame < topItem.from + topItem.durationInFrames + 2) {
          return frame;
        }
      }
    }

    // 2. Audio-driven: when no video is active, sync to first active audio.
    const items = itemsStore.getState().items;
    for (const item of items) {
      if (item.type !== 'audio' && item.type !== 'video') continue;
      if (!item.src) continue;
      const el = audioElements.get(item.id);
      if (!el || el.paused || el.seeking || el.readyState < 2) continue;
      const sourceFps = item.sourceFps ?? fps;
      const speed = Math.abs(item.speed ?? 1) || 1;
      const sourceStartFrame = item.sourceStart ?? 0;
      const sourceFrames = el.currentTime * sourceFps;
      const timelineFramesInClip = (sourceFrames - sourceStartFrame) / speed;
      const frame = item.from + timelineFramesInClip;
      if (Number.isFinite(frame) && frame >= item.from - 2 && frame < item.from + item.durationInFrames + 2) {
        return frame;
      }
    }

    // 3. Media item exists at this frame but isn't ready → hold at startFrame.
    //    This avoids a cosmetic backward-jump of the playhead when the video
    //    finishes loading (wall-clock advances during load → media clock
    //    takes over at video.currentTime=0 → playhead would jump back).
    //    Only advance via wall-clock when there is NO media at this frame.
    const holdFrame = Math.max(startFrame, _clock.lastIntFrame);
    for (const item of items) {
      if (item.type !== 'audio' && item.type !== 'video') continue;
      if (!item.src) continue;
      const itemEnd = item.from + item.durationInFrames;
      if (holdFrame >= item.from && holdFrame < itemEnd) {
        // Media item covers this frame — hold until it becomes ready.
        return holdFrame;
      }
    }

    // 4. Fallback: wall-clock (image clips / empty regions, no media at all).
    const wallMs = now - startTime;
    const fd = 1000 / fps;
    return startFrame + (wallMs / fd) * rate;
  }

  function startPlaybackLoop() {
    if (animFrame) { cancelAnimationFrame(animFrame); animFrame = null; }
    isPlayingRaw = true;

    const now = performance.now();
    const startFrame = playbackStore.getState().currentFrame;
    const fps = itemsStore.getState().fps;
    const pps = zoomStore.getState().pixelsPerSecond;
    const rate = playbackStore.getState().playbackRate;
    const maxFrames = itemsStore.getState().maxItemEndFrame || 1;

    _clock = {
      startTime: now,
      startFrame,
      rate,
      fps,
      pps,
      maxFrames,
      rulerPH: document.querySelector('[data-playhead="ruler"]') as HTMLElement | null,
      trackPHs: document.querySelectorAll('[data-playhead="track"]'),
      tcPreview: document.querySelector('[data-tc="preview"]') as HTMLElement | null,
      tcRuler: document.querySelector('[data-tc="ruler"]') as HTMLElement | null,
      lastIntFrame: startFrame - 1,  // force first-frame branch on first tick
      lastAudioSyncFrame: startFrame,
      lastVideoCheckFrame: startFrame,
    };

    // Kick off media IMMEDIATELY (before the rAF tick runs) so decoders have
    // a head-start. Since computeExactFrame falls back to wall-clock until
    // video.readyState ≥ HAVE_CURRENT_DATA, the playhead starts on wall-clock
    // and transparently hands off to the media clock once ready.
    updateActivePreviewItems(startFrame);
    syncAudioToFrame(startFrame, true, fps);
    audioSyncCounter = 0;

    let _handedOffToMediaClock = false;

    const tick = (tnow: number) => {
      if (!_clock) return;
      const c = _clock;

      const exactFrame = computeExactFrame(tnow);
      const intFrame = Math.floor(exactFrame);

      // Detect the wall-clock → media-clock handoff and rebase the fallback
      // anchor so any fallback read afterwards stays in phase with media.
      const topItem = [...activePreviewItems].reverse().find(i => i.type === 'video');
      let mediaReady = false;
      if (topItem) {
        const vid = document.querySelector(`video[data-preview-video="${topItem.id}"]`) as HTMLVideoElement | null;
        if (vid && !vid.paused && vid.readyState >= 2) mediaReady = true;
      }
      if (!_handedOffToMediaClock && mediaReady) {
        _handedOffToMediaClock = true;
        rebaseClock(exactFrame, tnow);
      }

      // ── Smooth playhead every tick ──
      const pxPerFrame = c.pps / c.fps;
      const smoothPx = exactFrame * pxPerFrame;
      const sl = itemsStore.getState().scrollLeft;
      if (c.rulerPH) c.rulerPH.style.transform = `translate3d(${smoothPx - sl}px,0,0)`;
      for (let i = 0; i < c.trackPHs.length; i++) {
        (c.trackPHs[i] as HTMLElement).style.transform = `translate3d(${smoothPx}px,0,0)`;
      }

      // ── Frame boundary work ──
      if (intFrame !== c.lastIntFrame) {
        c.lastIntFrame = intFrame;
        lastPlaybackFrame = intFrame;

        if (intFrame >= c.maxFrames) {
          playbackStore.getState().pause();
          return;
        }

        audioSyncCounter++;

        // Timecodes every 5 frames
        if (audioSyncCounter % 5 === 0) {
          const tc = formatTimecode(intFrame, c.fps);
          if (c.tcPreview) c.tcPreview.textContent = tc;
          if (c.tcRuler) c.tcRuler.textContent = tc;
        }

        // Clip-transition detection: update active items if the set changed.
        const changed = updateActivePreviewItems(intFrame);
        if (changed) {
          // Hand back to wall-clock briefly while new clip's decoder warms up.
          _handedOffToMediaClock = false;
          rebaseClock(intFrame, tnow);
        }

        // Audio re-sync throttled to every ~1s to activate/deactivate clips
        // without blasting el.currentTime writes on every frame.
        if (intFrame - c.lastAudioSyncFrame >= c.fps) {
          c.lastAudioSyncFrame = intFrame;
          syncAudioToFrame(intFrame, true, c.fps);
        }

        // Refresh snapshotted constants every ~2s; honour rate changes.
        if (intFrame - c.lastVideoCheckFrame >= c.fps * 2) {
          c.lastVideoCheckFrame = intFrame;
          const prevRate = c.rate;
          c.rate = playbackStore.getState().playbackRate;
          c.pps = zoomStore.getState().pixelsPerSecond;
          c.fps = itemsStore.getState().fps;
          c.maxFrames = itemsStore.getState().maxItemEndFrame || 1;

          if (c.rate !== prevRate) {
            // Video playback rates will be updated in the main tick loop below
            c.lastAudioSyncFrame = intFrame - c.fps;
          }
        }
      }

      // Sync all active video elements' currentTime and playbackRate
      for (const item of activePreviewItems) {
        if (item.type === 'video') {
          const video = document.querySelector(`video[data-preview-video="${item.id}"]`) as HTMLVideoElement | null;
          if (video && video.readyState >= 1) {
            const frameInClip = exactFrame - item.from;
            const speed = item.speed ?? 1;
            const sourceStartFrame = item.sourceStart ?? 0;
            const sourceFps = item.sourceFps ?? c.fps;
            const targetTime = (sourceStartFrame + frameInClip * speed) / sourceFps;
            const effectiveRate = Math.abs(speed) * c.rate * (c.fps / sourceFps);
            
            if (!video.seeking && Math.abs(video.currentTime - targetTime) > 0.1) {
              video.currentTime = targetTime;
            }
            if (video.playbackRate !== effectiveRate) {
              video.playbackRate = effectiveRate;
            }
            if (video.paused) {
              video.play().catch(() => {});
            }
          }
        }
      }

      animFrame = requestAnimationFrame(tick);
    };
    animFrame = requestAnimationFrame(tick);
  }

  function stopPlaybackLoop() {
    if (animFrame) { cancelAnimationFrame(animFrame); animFrame = null; }
    isPlayingRaw = false;
    _clock = null;
    activePreviewItems.forEach(item => {
      if (item.type === 'video') {
        const vid = document.querySelector(`video[data-preview-video="${item.id}"]`) as HTMLVideoElement | null;
        if (vid && !vid.paused) vid.pause();
      }
    });
    pauseAllAudio();
    // Use the full setCurrentFrame (with epoch bump) on stop
    // so the UI updates the playhead position, render preview, etc.
    playbackStore.getState().setCurrentFrame(lastPlaybackFrame);
  }

  /** Pause all active audio elements without altering their currentTime. */
  function pauseAllAudio() {
    audioElements.forEach((el) => { if (!el.paused) el.pause(); });
  }

  // Subscribe to playbackStore — only react to isPlaying changes
  let _prevIsPlaying = false;
  const unsubPlayback = playbackStore.subscribe((state) => {
    const nowPlaying = state.isPlaying;
    if (nowPlaying === _prevIsPlaying) return; // no change
    _prevIsPlaying = nowPlaying;
    if (nowPlaying) {
      startPlaybackLoop();
    } else {
      stopPlaybackLoop();
    }
  });

  onDestroy(() => {
    unsubPlayback();
    stopPlaybackLoop();
  });

  // ─── Audio Playback Engine ─────────────────────────────────────────────
  // Manages hidden HTML5 media elements to play audio in sync with timeline.
  const audioElements = new Map<string, HTMLMediaElement>();

  function getOrCreateAudioEl(item: TimelineItem): HTMLMediaElement | null {
    if (!item.src) return null;
    let el = audioElements.get(item.id);
    if (el) return el;
    const src = assetUrl(item.src);
    // Always use <audio> — even for video files. Chromium extracts just the
    // audio track, avoiding redundant video decoding that competes with the
    // preview <video> element for GPU decoder slots.
    if (item.type === 'video' || item.type === 'audio') {
      const a = document.createElement('audio');
      a.src = src;
      a.preload = 'auto';
      document.body.appendChild(a);
      el = a;
    } else {
      return null;
    }
    audioElements.set(item.id, el);
    return el;
  }

  function cleanupAudioElements() {
    audioElements.forEach((el) => {
      el.pause();
      el.remove();
    });
    audioElements.clear();
  }

  // ─── Audio / Video Sync ──────────────────────────────────────────────
  // Extracted so it can be called from the rAF tick (during playback) or
  // reactively (when paused/scrubbing). During playback the rAF tick calls
  // this every few frames; during scrubbing the $effect below calls it on
  // every frame change.

  // Cached muted/solo sets — rebuilt only when tracks change
  let _cachedMutedTrackIds: Set<string> = new Set();
  let _cachedSoloTrackIds: Set<string> = new Set();
  let _cachedHasSolo = false;
  let _cachedTracksEpoch = -1;

  function rebuildTrackAudioCache(tracks: typeof itemsStore extends { getState: () => infer S } ? S extends { tracks: infer T } ? T : never : never) {
    _cachedMutedTrackIds = new Set<string>();
    _cachedSoloTrackIds = new Set<string>();
    for (const t of tracks) {
      if (t.muted) _cachedMutedTrackIds.add(t.id);
      if (t.solo) _cachedSoloTrackIds.add(t.id);
    }
    _cachedHasSolo = _cachedSoloTrackIds.size > 0;
  }

  // Reusable set to avoid allocation every sync call
  const _activeItemIds = new Set<string>();

  function syncAudioToFrame(frame: number, playing: boolean, currentFps: number) {
    const pbState = playbackStore.getState();
    const tiState = itemsStore.getState();
    const globalMuted = pbState.muted;
    const globalVolume = pbState.volume;
    const items = tiState.items;
    const tracks = tiState.tracks;

    // Rebuild muted/solo cache only when tracks array identity changes
    const tracksLen = tracks.length;
    if (tracksLen !== _cachedTracksEpoch) {
      _cachedTracksEpoch = tracksLen;
      rebuildTrackAudioCache(tracks);
    }

    _activeItemIds.clear();

    for (const item of items) {
      if (item.type !== 'video' && item.type !== 'audio') continue;
      if (!item.src) continue;
      const vol = item.volume ?? 1;
      if (vol <= 0) continue;

      const itemEnd = item.from + item.durationInFrames;
      const isActive = frame >= item.from && frame < itemEnd;

      if (!isActive) {
        const el = audioElements.get(item.id);
        if (el && !el.paused) el.pause();
        continue;
      }

      _activeItemIds.add(item.id);

      const trackMuted = _cachedMutedTrackIds.has(item.trackId);
      const trackSoloed = !_cachedHasSolo || _cachedSoloTrackIds.has(item.trackId);

      const el = getOrCreateAudioEl(item);
      if (!el) continue;

      const frameInClip = frame - item.from;
      const speed = item.speed ?? 1;
      const sourceStartFrame = item.sourceStart ?? 0;
      const sourceFrame = sourceStartFrame + frameInClip * speed;
      const sourceFps = item.sourceFps ?? currentFps;
      const targetTime = sourceFrame / sourceFps;

      const effectiveVol = (globalMuted || trackMuted || !trackSoloed) ? 0 : vol * globalVolume;
      el.volume = Math.max(0, Math.min(1, effectiveVol));
      el.playbackRate = Math.abs(speed) * pbState.playbackRate * (currentFps / sourceFps);

      if (playing) {
        // Only seek when the audio element was PAUSED (first activation of
        // this clip) or when drift is absolutely massive (>1s — indicates
        // the audio fell off a cliff, e.g. system sleep or decoder error).
        // During normal playback we let the audio element play naturally;
        // the rAF tick derives the playhead from a media clock, so a small
        // audio drift costs us nothing visually and avoids audible glitches.
        if (el.paused) {
          if (effectiveVol > 0) {
            if (!el.seeking && Math.abs(el.currentTime - targetTime) > 0.1) {
              el.currentTime = targetTime;
            }
            el.play().catch(() => {});
          }
        } else if (!el.seeking && Math.abs(el.currentTime - targetTime) > 1.0) {
          el.currentTime = targetTime;
        }
      } else {
        if (!el.paused) el.pause();
        el.currentTime = targetTime;
      }
    }

    audioElements.forEach((el, id) => {
      if (!_activeItemIds.has(id) && !el.paused) {
        el.pause();
      }
    });
  }

  /** Update the active preview items based on the current frame. Returns true if changed. */
  function updateActivePreviewItems(frame: number): boolean {
    const currentActive = itemsStore.getState().items.filter(i => {
      return frame >= i.from && frame < i.from + i.durationInFrames;
    });
    
    let changed = false;
    if (currentActive.length !== activePreviewItems.length) {
      changed = true;
    } else {
      for (let i = 0; i < currentActive.length; i++) {
        if (currentActive[i].id !== activePreviewItems[i].id) { changed = true; break; }
      }
    }
    if (changed) {
      const tracks = itemsStore.getState().tracks;
      const orderMap = new Map(tracks.map(t => [t.id, t.order]));
      currentActive.sort((a,b) => (orderMap.get(a.trackId) ?? 0) - (orderMap.get(b.trackId) ?? 0));
      activePreviewItems = currentActive;
      return true;
    }
    return false;
  }

  // During playback, audio sync is handled by the rAF tick.
  // When paused/scrubbing, sync with throttle to prevent 30fps reactive iterations.
  let _audioSyncTimer: ReturnType<typeof setTimeout> | null = null;
  $effect(() => {
    if (pb.isPlaying) return;
    const frame = pb.currentFrame;
    const currentFps = fps;
    // Throttle: don't sync more than ~15fps during rapid scrubbing
    if (_audioSyncTimer) clearTimeout(_audioSyncTimer);
    _audioSyncTimer = setTimeout(() => {
      _audioSyncTimer = null;
      syncAudioToFrame(frame, false, currentFps);
    }, 30);
  });

  // Pre-warm audio elements when items change so the first play doesn't
  // stutter while the audio decoder loads from disk. Elements are created
  // with preload='auto' and kept paused at currentTime=0 until needed.
  let _prewarmedItemIds: Set<string> = new Set();
  $effect(() => {
    const items = ti.items;
    const stillPresent = new Set<string>();
    for (const item of items) {
      if (item.type !== 'video' && item.type !== 'audio') continue;
      if (!item.src) continue;
      stillPresent.add(item.id);
      if (_prewarmedItemIds.has(item.id)) continue;
      // getOrCreateAudioEl handles creation + src assignment; calling it
      // here warms the browser decoder before playback starts.
      const el = getOrCreateAudioEl(item);
      if (el) {
        el.volume = 0; // silent until real sync sets volume
        _prewarmedItemIds.add(item.id);
      }
    }
    // Remove dead entries
    for (const id of _prewarmedItemIds) {
      if (!stillPresent.has(id)) _prewarmedItemIds.delete(id);
    }
  });

  onDestroy(() => {
    cleanupAudioElements();
  });

  // ─── Request frame render from Rust backend on frame change ───────────
  let lastRenderedFrame = -1;
  let isRendering = false;
  let renderEpoch = $state(0);
  let pendingRenderFrame: number | null = null;
  let _renderDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  function requestRender(frame: number) {
    if (!currentProject) return;
    if (isRendering) {
      pendingRenderFrame = frame;
      return;
    }
    isRendering = true;
    lastRenderedFrame = frame;

    const projectState = {
      ...currentProject,
      timeline: {
        items: itemsStore.getState().items,
        tracks: itemsStore.getState().tracks,
      }
    };

    invoke("freecut_render_frame", { project: projectState, frame })
      .then((pathStr: any) => {
        if (pathStr) {
          // Preload the image before updating state to prevent flicker
          const assetSrc = assetUrl(pathStr as string);
          const url = assetSrc + "?t=" + Date.now();
          const img = document.createElement('img');
          const apply = () => {
            renderedFramePath = pathStr as string;
            renderEpoch++;
          };
          img.onload = apply;
          img.onerror = apply; // show even if preload fails
          img.src = url;
        }
      })
      .catch(console.error)
      .finally(() => {
        isRendering = false;
        if (pendingRenderFrame !== null && pendingRenderFrame !== lastRenderedFrame) {
          const next = pendingRenderFrame;
          pendingRenderFrame = null;
          requestRender(next);
        }
      });
  }

  // Skip FFmpeg per-frame renders during playback — the native <video>
  // preview handles smooth real-time display. Only render when paused
  // (scrubbing / seeking) for accurate composited preview.
  // PERF: Debounce render requests by 80ms during rapid scrubbing to
  // avoid stacking FFmpeg subprocess calls.
  $effect(() => {
    const playing = pb.isPlaying;
    if (playing) return;
    const frame = pb.currentFrame;
    if (frame !== lastRenderedFrame && currentProject) {
      // 1. Instantly update DOM overlays for real-time preview during scrubbing
      updateActivePreviewItems(frame);
      setTimeout(() => {
        activePreviewItems.forEach(item => {
          if (item.type === 'video') {
            const video = document.querySelector(`video[data-preview-video="${item.id}"]`) as HTMLVideoElement | null;
            if (video && video.readyState >= 1) {
              const frameInClip = frame - item.from;
              const speed = item.speed ?? 1;
              const sourceStartFrame = item.sourceStart ?? 0;
              const sourceFps = item.sourceFps ?? itemsStore.getState().fps;
              const targetTime = (sourceStartFrame + frameInClip * speed) / sourceFps;
              if (Math.abs(video.currentTime - targetTime) > 0.05) {
                video.currentTime = targetTime;
              }
            }
          }
        });
      }, 0);

      // 2. Debounced backend render for precise composition preview
      if (_renderDebounceTimer) clearTimeout(_renderDebounceTimer);
      _renderDebounceTimer = setTimeout(() => {
        _renderDebounceTimer = null;
        requestRender(frame);
      }, 80);
    }
  });

  // ─── API calls ────────────────────────────────────────────────────────

  async function loadProjects() {
    try { 
      projects = await invoke("freecut_list_projects"); 
      hwEncoders = await invoke("freecut_get_hw_encoders");
      
      // Auto-select the first NVENC encoder if available (NVIDIA GPU), otherwise AMD/Intel
      if (hwEncoders.length > 0 && !exportHwAccel) {
        const nvenc = hwEncoders.find(enc => enc.name === "h264_nvenc");
        exportHwAccel = nvenc ? nvenc.name : hwEncoders[0].name;
      }
    } catch (e) { console.error(e); }
  }

  function openNewProjectModal() {
    newProjectName = `Untitled Project ${projects.length + 1}`;
    selectedRatioIdx = 0;
    showNewProjectModal = true;
  }

  async function createProject() {
    try {
      const preset = selectedRatio;
      const name = newProjectName.trim() || `Untitled Project ${projects.length + 1}`;
      const project: Project = await invoke("freecut_create_project", {
        args: { name, width: preset.w, height: preset.h, fps: 30 },
      });
      currentProject = { ...project, dubbing: project.dubbing ?? createDefaultDubbingSession() };
      itemsStore.getState().setFps(project.metadata.fps);
      historyStore.getState().clear();
      isDirty = false;
      showNewProjectModal = false;
      currentView = "editor";
      await loadProjects();
      // Persist so a hard reload returns to this project (same as openProject).
      invoke("freecut_set_setting", { key: LAST_PROJECT_KEY, value: project.id }).catch(console.error);
    } catch (e) { console.error(e); }
  }

  async function openProject(id: string) {
    try {
      isLoading = true;
      const project: Project | null = await invoke("freecut_get_project", { id });
      if (project) {
        currentProject = { ...project, dubbing: project.dubbing ?? createDefaultDubbingSession() };
        itemsStore.getState().setFps(project.metadata.fps);
        if (project.timeline) {
          itemsStore.getState().setItems(project.timeline.items ?? []);
          itemsStore.getState().setTracks(project.timeline.tracks ?? []);
        }
        mediaLibrary = await invoke("freecut_list_media", { projectId: id });
        // ensureSourceMediaSelection is now handled by DubbingPanel
        historyStore.getState().clear();
        isDirty = false;
        currentView = "editor";
        // Persist so a hard reload returns to the same project (DB-backed).
        invoke("freecut_set_setting", { key: LAST_PROJECT_KEY, value: id }).catch(console.error);
      }
    } catch (e) { console.error(e); } finally { isLoading = false; }
  }

  async function deleteProject(id: string) {
    try { await invoke("freecut_delete_project", { id }); await loadProjects(); } catch (e) { console.error(e); }
  }

  /** Build the serializable project snapshot for saving. */
  function buildProjectSnapshot() {
    const state = itemsStore.getState();
    return {
      ...currentProject!,
      duration: state.maxItemEndFrame,
      timeline: { items: state.items, tracks: state.tracks },
      dubbing: currentProject!.dubbing ?? createDefaultDubbingSession(),
    };
  }

  async function saveProject() {
    if (!currentProject) return;
    try {
      await invoke("freecut_save_project", { project: buildProjectSnapshot() });
      isDirty = false;

      // Show success toast
      toastMessage = "Project saved";
      if (toastTimer) clearTimeout(toastTimer);
      toastTimer = setTimeout(() => { toastMessage = null; }, 2000);
    } catch (e) {
      console.error(e);
      toastMessage = "Failed to save project";
      if (toastTimer) clearTimeout(toastTimer);
      toastTimer = setTimeout(() => { toastMessage = null; }, 3000);
    }
  }

  async function importMedia() {
    if (!currentProject) return;
    try {
      const selected = await open({
        multiple: true,
        filters: [{
          name: "Media",
          extensions: ["mp4", "mov", "avi", "mkv", "webm", "mp3", "wav", "flac", "aac", "ogg", "png", "jpg", "jpeg", "gif", "webp", "bmp"],
        }],
      });
      if (selected) {
        const paths = Array.isArray(selected) ? selected : [selected];
        for (const filePath of paths) {
          if (typeof filePath === "string") {
            const media: MediaItem = await invoke("freecut_import_media", { projectId: currentProject.id, filePath });
            mediaLibrary = [...mediaLibrary, media];
            if (media.mediaType !== "image" && !dubbingSession.sourceMediaId) {
              patchCurrentProjectDubbing({ sourceMediaId: media.id, sourceMediaPath: media.filePath });
            }
            if (media.mediaType === "video" || media.mediaType === "image") {
              invoke("freecut_generate_thumbnails", {
                mediaId: media.id, filePath: media.filePath, count: 8,
              }).catch(console.error);
            }
            if (media.mediaType === "audio" || media.mediaType === "video") {
              invoke("freecut_generate_waveform", {
                mediaId: media.id, filePath: media.filePath, sampleCount: 500,
              }).catch(console.error);
            }
          }
        }
      }
    } catch (e) { console.error(e); }
  }

  function openNdeImport() {
    showNdeExplorerModal = true;
  }

  async function handleNdeImport(file: { path: string }) {
    if (!currentProject) return;
    try {
      const media: MediaItem = await invoke("freecut_import_media", { projectId: currentProject.id, filePath: file.path });
      mediaLibrary = [...mediaLibrary, media];
      if (media.mediaType !== "image" && !dubbingSession.sourceMediaId) {
        patchCurrentProjectDubbing({ sourceMediaId: media.id, sourceMediaPath: media.filePath });
      }
      if (media.mediaType === "video" || media.mediaType === "image") {
        invoke("freecut_generate_thumbnails", {
          mediaId: media.id, filePath: media.filePath, count: 8,
        }).catch(console.error);
      }
      if (media.mediaType === "audio" || media.mediaType === "video") {
        invoke("freecut_generate_waveform", {
          mediaId: media.id, filePath: media.filePath, sampleCount: 500,
        }).catch(console.error);
      }
    } catch (e) { console.error(e); }
  }

  async function deleteMedia(media: MediaItem) {
    try {
      await invoke("freecut_delete_media", { mediaId: media.id });
      mediaLibrary = mediaLibrary.filter((m) => m.id !== media.id);
      if (dubbingSession.sourceMediaId === media.id) {
        const next = mediaLibrary.find((m) => m.mediaType !== "image") ?? null;
        patchCurrentProjectDubbing({
          sourceMediaId: next?.id ?? null,
          sourceMediaPath: next?.filePath ?? null,
        });
      }
    } catch (e) { console.error(e); }
  }



  async function exportVideo() {
    if (!currentProject) return;
    try {
      let filePath: string | null = null;
      
      // E2E Mock: bypass native save dialog which hangs tests
      if ((window as any).__E2E_TEST__) {
        filePath = 'e2e-video-output.mp4';
        console.log("E2E Mock: saved to " + filePath);
      } else {
        filePath = await save({
          filters: [{
            name: "Video",
            extensions: exportCodec === "vp9" ? ["webm"] : ["mp4"],
          }],
          defaultPath: `${currentProject.name}.${exportCodec === "vp9" ? "webm" : "mp4"}`,
        });
      }
      
      if (!filePath) return;

      isExporting = true;
      exportProgress = 0;
      exportError = null;

      const state = itemsStore.getState();
      const projectData = {
        ...currentProject,
        duration: state.maxItemEndFrame,
        timeline: { items: state.items, tracks: state.tracks },
      };

      // E2E Mock: bypass actual Rust native FFmpeg export which takes too long / errors
      if ((window as any).__E2E_TEST__) {
        document.body.setAttribute('data-export-called', 'true');
        isExporting = false;
        showExportModal = false;
        return;
      }

      await invoke("freecut_export_video", {
        project: projectData,
        config: {
          outputPath: filePath,
          codec: exportCodec,
          width: currentProject.metadata.width,
          height: currentProject.metadata.height,
          fps: currentProject.metadata.fps,
          quality: exportQuality,
          hwAccel: exportHwAccel,
        },
      });
    } catch (e: any) {
      isExporting = false;
      exportError = e?.toString() ?? "Export failed";
      console.error("export error:", e);
    }
  }

  function handleRulerClick(e: MouseEvent) {
    // Now handled by startPlayheadDrag/scrubPlayhead
    const ruler = document.getElementById('timeline-ruler');
    if (!ruler) return;
    const rulerArea = ruler.querySelector('.flex-1') as HTMLElement;
    if (!rulerArea) return;
    const rect = rulerArea.getBoundingClientRect();
    const x = e.clientX - rect.left + ti.scrollLeft;
    const frame = pixelToFrame(x);
    playbackStore.getState().setCurrentFrame(Math.max(0, Math.min(frame, totalFrames)));
  }

  // ─── Convert file path to Tauri 2 asset URL ─────────────────────────────
  function assetUrl(path: string | null | undefined): string {
    if (!path) return "";
    return convertFileSrc(path);
  }

  // ─── Pointer-based Drag to Timeline (WebView2-safe) ───────────────────
  // HTML5 DragEvent is unreliable in WebView2. We use pointer events instead.
  function startMediaPointerDrag(e: MouseEvent, media: MediaItem) {
    e.preventDefault();
    e.stopPropagation();
    console.log("[FreeCut] startMediaPointerDrag:", media.fileName, "at", e.clientX, e.clientY);
    pendingDragPayload = { source: "media", media };
    const startX = e.clientX;
    const startY = e.clientY;
    let activated = false;

    const onMove = (ev: MouseEvent) => {
      const dx = ev.clientX - startX;
      const dy = ev.clientY - startY;
      // Threshold before entering drag mode
      if (!activated && Math.abs(dx) < 5 && Math.abs(dy) < 5) return;
      if (!activated) {
        activated = true;
        isPointerDragging = true;
        document.body.style.cursor = "copy";
        document.body.style.userSelect = "none";
        console.log("[FreeCut] drag activated for:", media.fileName);
      }
      pointerDragGhostX = ev.clientX;
      pointerDragGhostY = ev.clientY;
    };

    const onUp = (ev: MouseEvent) => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";

      if (!activated) {
        // Was just click, not drag — ignore
        pendingDragPayload = null;
        console.log("[FreeCut] drag not activated (click), ignoring");
        return;
      }
      isPointerDragging = false;
      console.log("[FreeCut] mouseup at", ev.clientX, ev.clientY, "→ handlePointerDrop");
      handlePointerDrop(ev.clientX, ev.clientY);
    };

    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }

  function startTextPointerDrag(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    console.log("[FreeCut] startTextPointerDrag");
    pendingDragPayload = { source: "text" };
    const startX = e.clientX;
    const startY = e.clientY;
    let activated = false;

    const onMove = (ev: MouseEvent) => {
      const dx = ev.clientX - startX;
      const dy = ev.clientY - startY;
      if (!activated && Math.abs(dx) < 5 && Math.abs(dy) < 5) return;
      if (!activated) {
        activated = true;
        isPointerDragging = true;
        document.body.style.cursor = "copy";
        document.body.style.userSelect = "none";
      }
      pointerDragGhostX = ev.clientX;
      pointerDragGhostY = ev.clientY;
    };

    const onUp = (ev: MouseEvent) => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      if (!activated) {
        pendingDragPayload = null;
        return;
      }
      isPointerDragging = false;
      handlePointerDrop(ev.clientX, ev.clientY);
    };

    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }

  function handlePointerDrop(clientX: number, clientY: number) {
    const data = pendingDragPayload;
    pendingDragPayload = null;
    if (!data) return;

    // Accept drops on the entire timeline section (preview + transport + timeline)
    // by looking for the tracks-container. If it doesn't exist, try the broader
    // timeline wrapper.
    const tracksContainer = document.getElementById("tracks-container");
    if (!tracksContainer) {
      console.warn("[FreeCut] tracks-container not found for drop");
      return;
    }
    const rect = tracksContainer.getBoundingClientRect();

    // Be lenient: allow drops that are somewhat near the timeline area.
    // This prevents drops from failing when cursor is slightly outside.
    const padding = 40;
    if (
      clientX < rect.left - padding || clientX > rect.right + padding ||
      clientY < rect.top - padding || clientY > rect.bottom + padding
    ) {
      console.log("[FreeCut] drop outside timeline area", { clientX, clientY, rect: { left: rect.left, right: rect.right, top: rect.top, bottom: rect.bottom } });
      return;
    }

    // The tracks-container has a 120px track header on each row.
    // Clips are rendered starting from `left: 120px` within each track row.
    // So subtract the header width from the X calculation.
    const TRACK_HEADER_WIDTH = 120;
    const dropX = Math.max(0, clientX - rect.left - TRACK_HEADER_WIDTH + ti.scrollLeft);
    const dropY = Math.max(0, clientY - rect.top + tracksContainer.scrollTop);
    let fromFrame = Math.max(0, pixelToFrame(dropX));

    let trackId = "";
    let yAcc = 0;
    for (const t of itemsStore.getState().tracks) {
      if (dropY >= yAcc && dropY < yAcc + t.height) { trackId = t.id; break; }
      yAcc += t.height;
    }

    if (!trackId) {
      const targetKind = (data.source === "media" && data.media?.mediaType === "audio") ? "audio" : "video";
      trackId = createDefaultTrack(targetKind);
    }

    historyStore.getState().push();

    if (data.source === "media" && data.media) {
      const media = data.media;
      const duration = media.durationSecs ? Math.round(media.durationSecs * fps) : fps * 5;
      const item: TimelineItem = {
        id: crypto.randomUUID(),
        trackId,
        from: fromFrame,
        durationInFrames: duration,
        label: media.fileName,
        type: media.mediaType as any,
        mediaId: media.id,
        src: media.filePath,
        sourceWidth: media.width,
        sourceHeight: media.height,
        sourceDuration: duration,
        sourceFps: media.fps ?? fps,
        sourceStart: 0,
        sourceEnd: duration,
        speed: 1,
        volume: 1,
      };
      itemsStore.getState().addItem(item);
      console.log("[FreeCut] Added media to timeline:", item.label, "at frame", fromFrame, "on track", trackId);
    } else if (data.source === "text") {
      const duration = fps * 3;
      const width = currentProject?.metadata.width ?? 1920;
      const height = currentProject?.metadata.height ?? 1080;
      const item: TimelineItem = {
        id: crypto.randomUUID(),
        trackId,
        from: fromFrame,
        durationInFrames: duration,
        label: "Title Text",
        type: "text",
        text: "Add Title Here",
        fontSize: 72,
        fontFamily: "Inter",
        color: "#ffffff",
        textAlign: "center",
        fillColor: "#ffffff",
        transform: { x: width / 2, y: height / 2, rotation: 0, opacity: 1 },
      };
      itemsStore.getState().addItem(item);
      console.log("[FreeCut] Added text to timeline at frame", fromFrame, "on track", trackId);
    }
  }

  // ─── Clip Interactions (Drag / Trim / Split) ─────────────────────────
  function handleClipClick(e: MouseEvent, item: TimelineItem) {
    if (sel.activeTool === "razor") {
      const tracksContainer = document.getElementById("tracks-container");
      if (tracksContainer) {
        const rect = tracksContainer.getBoundingClientRect();
        const clickX = e.clientX - rect.left + ti.scrollLeft;
        const frame = pixelToFrame(clickX);
        historyStore.getState().push();
        itemsStore.getState().splitItem(item.id, frame);
      }
    } else if (sel.activeTool === "select") {
      if (e.shiftKey || e.ctrlKey || e.metaKey) {
        // Multi-select: toggle item in selection
        const current = selectionStore.getState().selectedItemIds;
        if (current.includes(item.id)) {
          selectionStore.getState().selectItems(current.filter(id => id !== item.id));
        } else {
          selectionStore.getState().selectItems([...current, item.id]);
        }
      } else {
        selectionStore.getState().selectItems([item.id]);
      }
    }
  }

  function startClipDrag(e: MouseEvent, itemId: string, itemFrom: number, trackId: string) {
    if (sel.activeTool !== "select") return;

    // Don't drag on locked tracks
    const track = itemsStore.getState().tracks.find(t => t.id === trackId);
    if (track?.locked) return;

    const target = e.target as HTMLElement;
    if (target.dataset.dragMode === "trim") return;

    e.preventDefault();
    const mouseStartX = e.clientX;
    const mouseStartY = e.clientY;
    let hasDragged = false;

    const onMove = (ev: MouseEvent) => {
      const dx = ev.clientX - mouseStartX;
      const dy = ev.clientY - mouseStartY;

      // 3px drag threshold — prevents accidental moves on click
      if (!hasDragged) {
        if (Math.abs(dx) < 3 && Math.abs(dy) < 3) return;
        hasDragged = true;
        historyStore.getState().push();
        isDraggingClip = true;
        dragClipId = itemId;
        dragStartX = mouseStartX;
        dragStartY = mouseStartY;
        dragStartFrom = itemFrom;
        dragStartTrackId = trackId;
        selectionStore.getState().selectItems([itemId]);
        // Alt+drag starts duplicate mode
        const isAltDrag = ev.altKey;
        document.body.style.cursor = isAltDrag ? "copy" : "grabbing";
        document.body.style.userSelect = "none";
      }

      // Update cursor dynamically based on alt key
      if (hasDragged) {
        document.body.style.cursor = ev.altKey ? "copy" : "grabbing";
      }

      if (!isDraggingClip || !dragClipId) return;
      const dFrames = pixelToFrame(dx);
      let newFrom = Math.max(0, dragStartFrom + dFrames);

      // Snap-to-edges: snap clip start/end to other item edges and playhead
      if (ti.snapEnabled) {
        const SNAP_THRESHOLD = 8; // pixels
        const snapThresholdFrames = pixelToFrame(SNAP_THRESHOLD);
        const clipDuration = itemsStore.getState().itemById[dragClipId]?.durationInFrames ?? 0;
        const clipEnd = newFrom + clipDuration;
        
        // Build snap targets from other items' edges + playhead
        const snapTargets: number[] = [pb.currentFrame]; // playhead
        const allItems = itemsStore.getState().items;
        for (const otherItem of allItems) {
          if (otherItem.id === dragClipId) continue;
          snapTargets.push(otherItem.from); // start edge
          snapTargets.push(otherItem.from + otherItem.durationInFrames); // end edge
        }

        let bestSnap: number | null = null;
        let bestDist = snapThresholdFrames;
        
        // Check clip start against all targets
        for (const target of snapTargets) {
          const dist = Math.abs(newFrom - target);
          if (dist < bestDist) {
            bestDist = dist;
            bestSnap = target;
          }
        }
        // Check clip end against all targets
        for (const target of snapTargets) {
          const dist = Math.abs(clipEnd - target);
          if (dist < bestDist) {
            bestDist = dist;
            bestSnap = target - clipDuration; // Adjust so end aligns
          }
        }

        if (bestSnap !== null) {
          newFrom = Math.max(0, bestSnap);
          activeSnapFrame = newFrom; // For visual indicator
        } else {
          activeSnapFrame = null;
        }
      } else {
        activeSnapFrame = null;
      }

      // Cross-track movement
      let newTrackId = dragStartTrackId;
      const tracksContainer = document.getElementById("tracks-container");
      if (tracksContainer) {
        const rect = tracksContainer.getBoundingClientRect();
        const dropY = ev.clientY - rect.top + tracksContainer.scrollTop;
        if (dropY >= 0) {
          let yAcc = 0;
          for (const t of itemsStore.getState().tracks) {
            if (dropY >= yAcc && dropY < yAcc + t.height) { newTrackId = t.id; break; }
            yAcc += t.height;
          }
        }
      }

      itemsStore.getState().moveItem(dragClipId, newFrom, newTrackId);
    };

    let altDragDuplicated = false; // prevent re-duplicating on same drag
    const onUp = (ev: MouseEvent) => {
      if (hasDragged && ev.altKey && dragClipId && !altDragDuplicated) {
        // Alt+drag: revert item to original position, create duplicate at new position
        altDragDuplicated = true;
        const currentItem = itemsStore.getState().itemById[dragClipId];
        if (currentItem) {
          // Move original back
          itemsStore.getState().moveItem(dragClipId, dragStartFrom, dragStartTrackId);
          // Add duplicate at the drop position
          const duplicate: TimelineItem = {
            ...currentItem,
            id: crypto.randomUUID(),
            from: currentItem.from, // was already moved to new position
            originId: crypto.randomUUID(),
          };
          // Re-calculate from the UI position
          const dFrames = pixelToFrame(ev.clientX - dragStartX);
          duplicate.from = Math.max(0, dragStartFrom + dFrames);
          itemsStore.getState().addItem(duplicate);
          selectionStore.getState().selectItems([duplicate.id]);
        }
      }
      if (!hasDragged) {
        // Was just a click, not a drag
      }
      isDraggingClip = false;
      dragClipId = null;
      activeSnapFrame = null;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };

    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }

  function startTrim(e: MouseEvent, itemId: string, side: "left" | "right") {
    e.preventDefault();
    e.stopPropagation();
    historyStore.getState().push();
    selectionStore.getState().selectItems([itemId]);
    isTrimming = true;

    const startX = e.clientX;
    
    // We track total dx accumulated so Zustand doesn't get flooded with jitter
    let lastDx = 0;

    const onMove = (ev: MouseEvent) => {
      if (!isTrimming) return;
      const currentDx = ev.clientX - startX;
      
      const dxDiff = currentDx - lastDx;
      const trimFrames = pixelToFrame(dxDiff);
      
      if (trimFrames !== 0) {
        lastDx += frameToPixel(trimFrames); // Match integer quantizations
        if (side === "left") {
          itemsStore.getState().trimItemStart(itemId, trimFrames);
        } else {
          itemsStore.getState().trimItemEnd(itemId, trimFrames);
        }
      }
    };

    const onUp = () => {
      isTrimming = false;
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };

    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }

  function addMediaToTimeline(media: MediaItem) {
    historyStore.getState().push();
    const state = itemsStore.getState();
    const trackId = state.tracks.length > 0 ? state.tracks[0]!.id : createDefaultTrack(media.mediaType === "audio" ? "audio" : "video");
    const from = state.maxItemEndFrame;
    const duration = media.durationSecs ? Math.round(media.durationSecs * fps) : fps * 5;

    const item: TimelineItem = {
      id: crypto.randomUUID(),
      trackId,
      from,
      durationInFrames: duration,
      label: media.fileName,
      type: media.mediaType as any,
      mediaId: media.id,
      src: media.filePath,
      sourceWidth: media.width,
      sourceHeight: media.height,
      sourceDuration: duration,
      sourceFps: media.fps ?? fps,
      sourceStart: 0,
      sourceEnd: duration,
      speed: 1,
      volume: 1,
    };

    state.addItem(item);
  }

  function createDefaultTrack(kind: "video" | "audio"): string {
    const state = itemsStore.getState();
    const id = crypto.randomUUID();
    const track: TimelineTrack = {
      id,
      name: kind === "video" ? `V${state.tracks.filter(t => t.kind === "video").length + 1}` : `A${state.tracks.filter(t => t.kind === "audio").length + 1}`,
      kind,
      height: 60,
      locked: false,
      visible: true,
      muted: false,
      solo: false,
      order: state.tracks.length,
      isGroup: false,
      isCollapsed: false,
    };
    state.addTrack(track);
    return id;
  }

  // ─── SRT Import → Timeline ────────────────────────────────────────────
  /** Detect if text contains Khmer Unicode characters (U+1780–U+17FF). */
  function containsKhmer(text: string): boolean {
    return /[\u1780-\u17FF]/.test(text);
  }

  let srtImportBusy = $state(false);

  async function importSrtToTimeline() {
    if (!currentProject) return;
    try {
      srtImportBusy = true;
      const selected = await open({
        multiple: false,
        filters: [{ name: "Subtitles", extensions: ["srt"] }],
      });
      if (!selected || Array.isArray(selected) || typeof selected !== "string") {
        srtImportBusy = false;
        return;
      }

      // Parse SRT via the existing backend command
      const imported: DubbingImportResult = await invoke("freecut_import_dubbing_srt", { filePath: selected });
      if (!imported.segments || imported.segments.length === 0) {
        srtImportBusy = false;
        toastMessage = "No subtitle segments found in SRT file";
        if (toastTimer) clearTimeout(toastTimer);
        toastTimer = setTimeout(() => { toastMessage = null; }, 3000);
        return;
      }

      historyStore.getState().push();

      // Detect Khmer content by sampling the first few segments
      const sampleText = imported.segments.slice(0, 10).map((s: DubbingSegment) => s.text).join(" ");
      const isKhmer = containsKhmer(sampleText);

      // Create a dedicated subtitle track
      const state = itemsStore.getState();
      const subtitleTrackId = crypto.randomUUID();
      const existingSubtitleTracks = state.tracks.filter(t => t.name.startsWith("Subtitles")).length;
      const trackName = existingSubtitleTracks > 0 ? `Subtitles ${existingSubtitleTracks + 1}` : "Subtitles";
      const subtitleTrack: TimelineTrack = {
        id: subtitleTrackId,
        name: trackName,
        kind: "video",
        height: 50,
        locked: false,
        visible: true,
        muted: false,
        solo: false,
        order: state.tracks.length,
        isGroup: false,
        isCollapsed: false,
        color: "#F59E0B",
      };
      state.addTrack(subtitleTrack);

      // Place each SRT segment as a text item on the timeline
      const width = currentProject.metadata.width;
      const height = currentProject.metadata.height;
      const fontFamily = isKhmer ? "Noto Sans Khmer" : "Inter";
      const fontSize = isKhmer ? 38 : 42;
      const yPos = height - 120;

      const items: TimelineItem[] = [];
      for (const [index, segment] of imported.segments.entries()) {
        const from = Math.max(0, Math.round(segment.startSecs * fps));
        const duration = Math.max(1, Math.round((segment.endSecs - segment.startSecs) * fps));
        const clipText = segment.text;

        items.push({
          id: crypto.randomUUID(),
          trackId: subtitleTrackId,
          from,
          durationInFrames: duration,
          label: clipText.length > 30 ? clipText.slice(0, 30) + "…" : clipText,
          type: "text",
          text: clipText,
          fontSize,
          fontFamily,
          color: "#ffffff",
          textAlign: "center",
          fillColor: "#ffffff",
          transform: { x: width / 2, y: yPos, rotation: 0, opacity: 1 },
        });
      }

      itemsStore.getState().addItems(items);

      // Show success toast
      const langTag = isKhmer ? " (ខ្មែរ)" : "";
      toastMessage = `✅ Placed ${items.length} subtitles on timeline${langTag}`;
      if (toastTimer) clearTimeout(toastTimer);
      toastTimer = setTimeout(() => { toastMessage = null; }, 3000);

      console.log(`[FreeCut] Imported ${items.length} SRT segments to timeline (Khmer: ${isKhmer})`);
    } catch (e: any) {
      console.error("[FreeCut] SRT import error:", e);
      toastMessage = `SRT import failed: ${e?.toString?.() ?? "Unknown error"}`;
      if (toastTimer) clearTimeout(toastTimer);
      toastTimer = setTimeout(() => { toastMessage = null; }, 4000);
    } finally {
      srtImportBusy = false;
    }
  }

  let autoGenBusy = $state(false);
  let autoGenPhase = $state('');

  async function autoGenerateSrtFromVideo(targetMedia?: MediaItem, language?: string) {
    if (!currentProject) return;

    // Use provided media or find the first video.
    const video = targetMedia ?? mediaLibrary.find(m => m.mediaType === 'video');
    if (!video || video.mediaType !== 'video') {
      toastMessage = "No video found — import a video first";
      if (toastTimer) clearTimeout(toastTimer);
      toastTimer = setTimeout(() => { toastMessage = null; }, 3000);
      return;
    }

    try {
      autoGenBusy = true;
      const langLabel = language === 'km' ? 'ខ្មែរ' : language === 'en' ? 'English' : '';
      autoGenPhase = `Extracting audio${langLabel ? ` (${langLabel})` : ''}…`;

      // Listen for progress events from the backend.
      const { listen } = await import("@tauri-apps/api/event");
      const unlisten = await listen<{ phase: string; message: string }>("freecut://dubbing-progress", (ev) => {
        autoGenPhase = ev.payload.message;
      });

      // Build whisper settings with language if provided.
      // We let Whisper auto-detect the source language (language: null).
      // If requesting Khmer, use Whisper's 'translate' task (to English) first,
      // because Whisper's native Khmer transcription is poor.
      // We pass the requested language to translateTo for the LLM translation pass.
      const whisperSettings: WhisperSettings | null = { 
        language: null, 
        model: null, 
        task: language === 'km' ? 'translate' : 'transcribe', 
        diarize: null 
      };
      const translateTo = language;

      // Call the backend to extract audio → transcribe → translate → parse SRT.
      const imported: DubbingImportResult = await invoke("freecut_auto_generate_srt", {
        videoPath: video.filePath,
        whisper: whisperSettings,
        translateTo,
      });

      unlisten();

      if (!imported.segments || imported.segments.length === 0) {
        autoGenBusy = false;
        autoGenPhase = '';
        toastMessage = "Whisper produced no subtitle segments";
        if (toastTimer) clearTimeout(toastTimer);
        toastTimer = setTimeout(() => { toastMessage = null; }, 3000);
        return;
      }

      historyStore.getState().push();

      // Detect Khmer content from output (may differ from requested language).
      const sampleText = imported.segments.slice(0, 10).map((s: DubbingSegment) => s.text).join(" ");
      const isKhmer = language === 'km' || containsKhmer(sampleText);

      // Create subtitle track.
      const state = itemsStore.getState();
      const subtitleTrackId = crypto.randomUUID();
      const existingSubtitleTracks = state.tracks.filter(t => t.name.startsWith("Subtitles")).length;
      const trackSuffix = langLabel ? ` (${langLabel})` : '';
      const trackName = existingSubtitleTracks > 0
        ? `Subtitles ${existingSubtitleTracks + 1}${trackSuffix}`
        : `Subtitles${trackSuffix}`;
      const subtitleTrack: TimelineTrack = {
        id: subtitleTrackId,
        name: trackName,
        kind: "video",
        height: 50,
        locked: false,
        visible: true,
        muted: false,
        solo: false,
        order: state.tracks.length,
        isGroup: false,
        isCollapsed: false,
        color: isKhmer ? "#F59E0B" : "#3B82F6",
      };
      state.addTrack(subtitleTrack);

      // Place subtitle items.
      const width = currentProject.metadata.width;
      const height = currentProject.metadata.height;
      const fontFamily = isKhmer ? "Noto Sans Khmer" : "Inter";
      const fontSize = isKhmer ? 38 : 42;
      const yPos = height - 120;

      const items: TimelineItem[] = [];
      for (const segment of imported.segments) {
        const from = Math.max(0, Math.round(segment.startSecs * fps));
        const duration = Math.max(1, Math.round((segment.endSecs - segment.startSecs) * fps));
        const clipText = segment.text;

        items.push({
          id: crypto.randomUUID(),
          trackId: subtitleTrackId,
          from,
          durationInFrames: duration,
          label: clipText.length > 30 ? clipText.slice(0, 30) + "…" : clipText,
          type: "text",
          text: clipText,
          fontSize,
          fontFamily,
          color: "#ffffff",
          textAlign: "center",
          fillColor: "#ffffff",
          transform: { x: width / 2, y: yPos, rotation: 0, opacity: 1 },
        });
      }

      itemsStore.getState().addItems(items);

      const langTag = isKhmer ? " (ខ្មែរ)" : language === 'en' ? " (English)" : "";
      toastMessage = `✅ Auto-generated ${items.length} subtitles${langTag}`;
      if (toastTimer) clearTimeout(toastTimer);
      toastTimer = setTimeout(() => { toastMessage = null; }, 4000);

      console.log(`[FreeCut] Auto-generated ${items.length} SRT segments from ${video.fileName} (lang: ${language ?? 'auto'})`);
    } catch (e: any) {
      console.error("[FreeCut] Auto-generate SRT error:", e);
      toastMessage = `Auto-generate failed: ${e?.toString?.() ?? "Unknown error"}`;
      if (toastTimer) clearTimeout(toastTimer);
      toastTimer = setTimeout(() => { toastMessage = null; }, 5000);
    } finally {
      autoGenBusy = false;
      autoGenPhase = '';
    }
  }

  function goBack() {
    // If there are unsaved changes, show confirmation modal instead of leaving.
    if (isDirty) {
      pendingGoBack = true;
      showUnsavedModal = true;
      return;
    }
    doGoBack();
  }

  /** Actually navigate back — called after save/discard decision. */
  function doGoBack() {
    // Clear the persisted project ID so the next hard reload starts at the
    // projects list rather than re-opening the closed project.
    invoke("freecut_delete_setting", { key: LAST_PROJECT_KEY }).catch(console.error);
    isDirty = false;
    pendingGoBack = false;
    showUnsavedModal = false;
    currentProject = null;
    currentView = "projects";
    mediaLibrary = [];
    renderedFramePath = null;

    playbackStore.getState().setCurrentFrame(0);
    playbackStore.getState().pause();
    itemsStore.getState().setItems([]);
    itemsStore.getState().setTracks([]);
    selectionStore.getState().clearSelection();
    historyStore.getState().clear();
    markers = [];
    inPoint = null;
    outPoint = null;
    clipboardItems = [];
  }

  async function handleUnsavedSave() {
    await saveProject();
    if (pendingGoBack) doGoBack();
  }
  function handleUnsavedDiscard() {
    isDirty = false;
    if (pendingGoBack) doGoBack();
  }
  function handleUnsavedCancel() {
    pendingGoBack = false;
    showUnsavedModal = false;
  }

  // ─── In/Out Points ────────────────────────────────────────────────────
  function setInPoint() {
    inPoint = pb.currentFrame;
    if (outPoint !== null && outPoint <= inPoint) outPoint = null;
  }

  function setOutPoint() {
    outPoint = pb.currentFrame;
    if (inPoint !== null && inPoint >= outPoint) inPoint = null;
  }

  // ─── Markers ──────────────────────────────────────────────────────────
  const markerColors = ["#ef4444", "#f59e0b", "#22c55e", "#3b82f6", "#a855f7", "#ec4899"];
  function addMarker(frame: number) {
    markers = [...markers, {
      id: crypto.randomUUID(),
      frame,
      label: `M${markers.length + 1}`,
      color: markerColors[markers.length % markerColors.length]!,
    }];
  }

  function removeMarker(id: string) {
    markers = markers.filter(m => m.id !== id);
  }

  // ─── Clipboard & Duplicate ────────────────────────────────────────────
  function duplicateSelectedItems() {
    historyStore.getState().push();
    const state = itemsStore.getState();
    const selected = sel.selectedItemIds.map(id => state.itemById[id]).filter(Boolean) as TimelineItem[];
    if (selected.length === 0) return;

    const positions = selected.map(item => ({
      from: item.from + item.durationInFrames,
      trackId: item.trackId,
    }));
    const newItems = state.duplicateItems(sel.selectedItemIds, positions);
    selectionStore.getState().selectItems(newItems.map(i => i.id));
  }

  function pasteItemsAtPlayhead() {
    if (clipboardItems.length === 0) return;
    const baseFrame = pb.currentFrame;
    const minFrom = Math.min(...clipboardItems.map(i => i.from));
    
    const newItems: TimelineItem[] = clipboardItems.map(item => ({
      ...item,
      id: crypto.randomUUID(),
      from: baseFrame + (item.from - minFrom),
      originId: crypto.randomUUID(),
    }));

    for (const item of newItems) {
      itemsStore.getState().addItem(item);
    }
    selectionStore.getState().selectItems(newItems.map(i => i.id));
  }

  // ─── Preview Scrubber ─────────────────────────────────────────────────
  function handleTrackMouseMove(e: MouseEvent) {
    const tracksContainer = document.getElementById("tracks-container");
    if (!tracksContainer || isDraggingClip || isTrimming) { previewFrame = null; return; }
    const rect = tracksContainer.getBoundingClientRect();
    const x = e.clientX - rect.left - 120 + ti.scrollLeft; // subtract track header
    if (x < 0) { previewFrame = null; return; }
    previewFrame = Math.max(0, pixelToFrame(x));
  }

  function handleTrackMouseLeave() {
    previewFrame = null;
  }

  // ─── Editable Properties Update ────────────────────────────────────────
  function updateSelectedItem(updates: Partial<TimelineItem>) {
    if (!selectedItem) return;
    historyStore.getState().push();
    itemsStore.getState().updateItem(selectedItem.id, updates);
  }

  function updateSelectedTransform(updates: any) {
    if (!selectedItem) return;
    historyStore.getState().push();
    itemsStore.getState().updateItemTransform(selectedItem.id, updates);
  }

  // ─── Keyframe Operations ────────────────────────────────────────────────
  function toggleKeyframe(property: string, currentValue: number) {
    if (!selectedItem) return;
    const frameOffset = Math.max(0, pb.currentFrame - selectedItem.from);
    
    let keyframes = [...(selectedItem.keyframes || [])];
    const existingIdx = keyframes.findIndex(k => k.property === property && k.frameOffset === frameOffset);
    
    if (existingIdx >= 0) {
      keyframes.splice(existingIdx, 1);
    } else {
      keyframes.push({ frameOffset, property, value: currentValue });
    }
    
    updateSelectedItem({ keyframes });
  }

  function hasKeyframeAtCurrentFrame(property: string) {
    if (!selectedItem) return false;
    const frameOffset = pb.currentFrame - selectedItem.from;
    return (selectedItem.keyframes || []).some(k => k.property === property && k.frameOffset === frameOffset);
  }

  async function removeBackground() {
    if (!selectedItem || !currentProject) return;
    if (selectedItem.type !== "video" && selectedItem.type !== "image") return;
    
    isRemovingBackground = true;
    bgRemovalError = null;
    
    try {
      const srcPath = selectedItem.src;
      if (!srcPath) throw new Error("No source media found on item.");
      
      const maskedPath = srcPath + "_masked.png";
      await invoke("freecut_remove_background", {
        inputPath: srcPath,
        outputPath: maskedPath
      });

      updateSelectedItem({
        src: maskedPath,
        label: `${selectedItem.label} (Masked)`
      });
      await saveProject();
    } catch (e: any) {
      bgRemovalError = e?.toString?.() ?? "AI Background Removal failed";
      
      if (bgRemovalError?.includes("not installed")) {
        import("🍎/state/desktop.svelte").then(({ openServiceHub }) => {
          openServiceHub({ require: ["ai-vision-runtime"], returnTo: "freecut" });
        });
      }
    } finally {
      isRemovingBackground = false;
    }
  }

  // ─── Keyboard & Wheel Handling ─────────────────────────────────────────

  function handleKeydown(e: KeyboardEvent) {
    if (currentView !== "editor") return;
    const target = e.target as HTMLElement;
    if (target.tagName === "INPUT" || target.tagName === "TEXTAREA") return;

    switch (e.key) {
      case " ":
        e.preventDefault();
        playbackStore.getState().togglePlayPause();
        break;
      case "ArrowLeft":
        e.preventDefault();
        playbackStore.getState().setCurrentFrame(Math.max(0, pb.currentFrame - (e.shiftKey ? 10 : 1)));
        break;
      case "ArrowRight":
        e.preventDefault();
        playbackStore.getState().setCurrentFrame(pb.currentFrame + (e.shiftKey ? 10 : 1));
        break;
      case "Home":
        e.preventDefault();
        playbackStore.getState().setCurrentFrame(0);
        itemsStore.getState().setScrollLeft(0);
        break;
      case "End":
        e.preventDefault();
        playbackStore.getState().setCurrentFrame(totalFrames);
        break;
      case "Delete":
      case "Backspace":
        if (sel.selectedItemIds.length > 0) {
          e.preventDefault();
          historyStore.getState().push();
          // Ripple delete: shift subsequent items left to fill gaps
          if (e.shiftKey) {
            const state = itemsStore.getState();
            const deletedItems = sel.selectedItemIds.map(id => state.itemById[id]).filter(Boolean) as TimelineItem[];
            // Find the smallest start frame among deleted items per track
            const gapsByTrack = new Map<string, { start: number; gap: number }[]>();
            for (const item of deletedItems) {
              const gaps = gapsByTrack.get(item.trackId) ?? [];
              gaps.push({ start: item.from, gap: item.durationInFrames });
              gapsByTrack.set(item.trackId, gaps);
            }
            state.removeItems(sel.selectedItemIds);
            // Shift remaining items on each track to close gaps
            for (const [trackId, gaps] of gapsByTrack) {
              gaps.sort((a, b) => a.start - b.start);
              const trackItems = itemsStore.getState().items.filter(i => i.trackId === trackId).sort((a, b) => a.from - b.from);
              let totalShift = 0;
              for (const gap of gaps) {
                for (const ti of trackItems) {
                  if (ti.from >= gap.start - totalShift) {
                    itemsStore.getState().moveItem(ti.id, ti.from - gap.gap);
                  }
                }
                totalShift += gap.gap;
              }
            }
          } else {
            itemsStore.getState().removeItems(sel.selectedItemIds);
          }
          selectionStore.getState().clearItemSelection();
        }
        break;
      case "s":
        if (e.ctrlKey || e.metaKey) { e.preventDefault(); saveProject(); }
        break;
      case "i":
        if (e.ctrlKey || e.metaKey) { e.preventDefault(); importMedia(); }
        else { setInPoint(); }
        break;
      case "o":
        if (!e.ctrlKey && !e.metaKey) { setOutPoint(); }
        break;
      case "z":
        if (e.ctrlKey || e.metaKey) { 
          e.preventDefault();
          if (e.shiftKey) historyStore.getState().redo();
          else historyStore.getState().undo();
        } else {
          zoomToFit();
        }
        break;
      case "y":
        if (e.ctrlKey || e.metaKey) {
          e.preventDefault();
          historyStore.getState().redo();
        }
        break;
      case "=":
      case "+":
        if (e.ctrlKey || e.metaKey) { e.preventDefault(); zoomStore.getState().zoomIn(); }
        break;
      case "-":
        if (e.ctrlKey || e.metaKey) { e.preventDefault(); zoomStore.getState().zoomOut(); }
        break;
      case "b":
        if (!e.ctrlKey && !e.metaKey) {
          // Split all selected items at playhead (or just one if single selected)
          if (sel.selectedItemIds.length > 0) {
            historyStore.getState().push();
            for (const id of sel.selectedItemIds) {
              itemsStore.getState().splitItem(id, pb.currentFrame);
            }
          }
        }
        break;
      case "d":
        if (e.altKey || ((e.ctrlKey || e.metaKey) && e.shiftKey)) {
          // Alt+D or Ctrl+Shift+D: Duplicate selected items
          e.preventDefault();
          if (sel.selectedItemIds.length > 0) {
            duplicateSelectedItems();
          }
        }
        break;
      case "v": if (!e.ctrlKey) selectionStore.getState().setActiveTool("select"); break;
      case "c":
        if (e.ctrlKey || e.metaKey) {
          // Ctrl+C: Copy items to clipboard (store in local var)
          e.preventDefault();
          clipboardItems = sel.selectedItemIds.map(id => itemsStore.getState().itemById[id]).filter(Boolean) as TimelineItem[];
        } else {
          selectionStore.getState().setActiveTool("razor");
        }
        break;
      case "x":
        if (e.ctrlKey || e.metaKey) {
          // Ctrl+X: Cut (copy + delete)
          e.preventDefault();
          clipboardItems = sel.selectedItemIds.map(id => itemsStore.getState().itemById[id]).filter(Boolean) as TimelineItem[];
          if (clipboardItems.length > 0) {
            historyStore.getState().push();
            itemsStore.getState().removeItems(sel.selectedItemIds);
            selectionStore.getState().clearItemSelection();
          }
        }
        break;
      case "v":
        if (e.ctrlKey || e.metaKey) {
          // Ctrl+V: Paste at playhead
          e.preventDefault();
          if (clipboardItems.length > 0) {
            historyStore.getState().push();
            pasteItemsAtPlayhead();
          }
        } else {
          selectionStore.getState().setActiveTool("select");
        }
        break;
      case "m":
        if (!e.ctrlKey && !e.metaKey) {
          // M: Add marker at playhead
          addMarker(pb.currentFrame);
        }
        break;
      case "a":
        if (e.ctrlKey || e.metaKey) {
          e.preventDefault();
          const allIds = itemsStore.getState().items.map(i => i.id);
          selectionStore.getState().selectItems(allIds);
        }
        break;
      case "Escape":
        selectionStore.getState().clearSelection();
        inPoint = null;
        outPoint = null;
        break;
      case "l":
        // L: reserved (was playback rate toggle — removed)
        break;
      case "j":
        // J: reserved (was reverse playback — removed)
        break;
      case "k":
        if (!e.ctrlKey && !e.metaKey) {
          // K: Stop playback
          if (pb.isPlaying) playbackStore.getState().pause();
        }
        break;
    }
  }

  function handleTimelineWheel(e: WheelEvent) {
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      // Cursor-anchored zoom: keep mouse position stable
      const target = e.currentTarget as HTMLElement;
      const rect = target.getBoundingClientRect();
      // Account for 120px track header offset in tracks container
      const headerOffset = target.id === 'tracks-container' ? 120 : 0;
      const cursorScreenX = e.clientX - rect.left - headerOffset;
      const cursorContentX = ti.scrollLeft + cursorScreenX;
      const currentZoom = zm.level;
      const currentPPS = currentZoom * 100;
      const cursorTime = currentPPS > 0 ? cursorContentX / currentPPS : 0;

      // Logarithmic zoom for symmetric in/out feel
      const logZoom = Math.log(currentZoom);
      const zoomDelta = e.deltaY > 0 ? -0.08 : 0.08;
      const newZoom = Math.max(0.01, Math.min(10, Math.exp(logZoom + zoomDelta)));
      const newPPS = newZoom * 100;
      const newCursorContentX = cursorTime * newPPS;
      const newScrollLeft = Math.max(0, newCursorContentX - cursorScreenX);

      zoomStore.getState().setZoomLevelImmediate(newZoom);
      itemsStore.getState().setScrollLeft(newScrollLeft);
    } else if (e.shiftKey) {
      // Shift+scroll = vertical scroll (let native behavior handle it)
    } else {
      // Default scroll = horizontal timeline scroll (NLE convention)
      e.preventDefault();
      const dx = e.deltaY || e.deltaX;
      itemsStore.getState().setScrollLeft(Math.max(0, ti.scrollLeft + dx));
    }
  }

  function handleZoomSlider(e: Event) {
    const val = parseFloat((e.target as HTMLInputElement).value);
    // Logarithmic mapping: slider 0..1 -> zoom 0.01..10
    const newZoom = 0.01 * Math.pow(10 / 0.01, val);
    zoomStore.getState().setZoomLevelImmediate(newZoom);
  }

  function zoomToFit() {
    const tracksContainer = document.getElementById('tracks-container');
    if (!tracksContainer) return;
    const containerWidth = tracksContainer.clientWidth - 120; // subtract track headers
    const contentDuration = Math.max(totalFrames / fps, 10);
    const newZoom = Math.max(0.01, Math.min(10, containerWidth / (contentDuration * 100)));
    zoomStore.getState().setZoomLevelImmediate(newZoom);
    itemsStore.getState().setScrollLeft(0);
  }

  // Inverse: zoom -> slider value (0..1)
  function zoomToSlider(zoom: number): number {
    return Math.log(zoom / 0.01) / Math.log(10 / 0.01);
  }

  // Playhead drag
  function startPlayheadDrag(e: MouseEvent) {
    e.preventDefault();
    isDraggingPlayhead = true;
    if (activePreviewTab === "media") activePreviewTab = "timeline";
    scrubPlayhead(e);

    const onMove = (ev: MouseEvent) => {
      if (!isDraggingPlayhead) return;
      scrubPlayhead(ev);
    };
    const onUp = () => {
      isDraggingPlayhead = false;
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
    };
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  }

  function scrubPlayhead(e: MouseEvent) {
    const ruler = document.getElementById('timeline-ruler');
    if (!ruler) return;
    // The ruler has a 120px spacer on the left for track headers.
    // Target the flex-1 ruler area child for accurate frame calculation.
    const rulerArea = ruler.querySelector('.flex-1') as HTMLElement;
    if (!rulerArea) return;
    const rect = rulerArea.getBoundingClientRect();
    const x = e.clientX - rect.left + ti.scrollLeft;
    const frame = Math.max(0, Math.min(pixelToFrame(x), totalFrames));
    playbackStore.getState().setCurrentFrame(frame);
  }

  // Timeline resize handle
  function startTimelineResize(e: MouseEvent) {
    e.preventDefault();
    isResizingTimeline = true;
    resizeStartY = e.clientY;
    resizeStartHeight = ed.timelineHeight;

    const onMove = (ev: MouseEvent) => {
      if (!isResizingTimeline) return;
      const dy = resizeStartY - ev.clientY;
      const newHeight = Math.max(120, Math.min(600, resizeStartHeight + dy));
      editorStore.getState().setTimelineHeight(newHeight);
    };
    const onUp = () => {
      isResizingTimeline = false;
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
    };
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  }

  function startLeftSidebarResize(e: MouseEvent) {
    e.preventDefault();
    isResizingLeftSidebar = true;
    leftSidebarResizeStartX = e.clientX;
    leftSidebarResizeStartWidth = ed.sidebarWidth;
    document.body.style.cursor = "ew-resize";
    document.body.style.userSelect = "none";

    const onMove = (ev: MouseEvent) => {
      if (!isResizingLeftSidebar) return;
      const dx = ev.clientX - leftSidebarResizeStartX;
      const newWidth = Math.max(280, Math.min(560, leftSidebarResizeStartWidth + dx));
      editorStore.getState().setSidebarWidth(newWidth);
    };
    const onUp = () => {
      isResizingLeftSidebar = false;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
    };

    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  }

  function toggleSnap() {
    itemsStore.getState().setSnapEnabled(!ti.snapEnabled);
  }

  function addNewTrack(kind: 'video' | 'audio') {
    createDefaultTrack(kind);
  }

  function removeTrack(trackId: string) {
    historyStore.getState().push();
    itemsStore.getState().removeTrack(trackId);
  }

  // ─── Format formatting ────────────────────────────────────────────────
  function formatTimecode(frame: number, fps: number): string {
    if (fps <= 0) return "00:00:00";
    const totalSeconds = frame / fps;
    const m = Math.floor(totalSeconds / 60);
    const s = Math.floor(totalSeconds % 60);
    const f = Math.round(frame % fps);
    return `${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}:${f.toString().padStart(2, "0")}`;
  }

  function formatFileSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function formatDuration(secs?: number): string {
    if (secs === undefined || secs === null || Number.isNaN(secs)) return "--:--";
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${m}:${s.toString().padStart(2, '0')}`;
  }

  function getMediaIcon(type: string) {
    switch (type) { case "video": return Video; case "audio": return Music; case "image": return Image; case "text": return Type; default: return Film; }
  }

  const toolIcons: Record<string, any> = {
    select: MousePointer2, razor: Slice, slip: GripHorizontal,
  };

  const tabIcons: Record<string, any> = {
    media: FolderOpen, effects: Sparkles, transitions: ChevronRight, text: Type, audio: Music, shapes: RectangleHorizontal, dubbing: Volume2, tools: Settings,
  };
  const sidebarTabs: Array<{ id: EditorTab; label: string }> = [
    { id: "media", label: "Media" },
    { id: "dubbing", label: "Dubbing" },
    { id: "tools", label: "Tools" },
    { id: "effects", label: "FX" },
    { id: "transitions", label: "Cuts" },
    { id: "text", label: "Text" },
  ];

  function frameToPixel(frame: number): number {
    return (frame / fps) * zm.pixelsPerSecond;
  }

  function pixelToFrame(px: number): number {
    return Math.round((px / zm.pixelsPerSecond) * fps);
  }

  const trackColors = ["#8B5CF6", "#06B6D4", "#F59E0B", "#EF4444", "#10B981", "#EC4899", "#6366F1", "#14B8A6"];
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- ─── Projects View ────────────────────────────────────────────────── -->
{#if currentView === "projects"}
  <div class="flex flex-col h-full bg-linear-to-br from-zinc-950 via-zinc-900 to-zinc-950">
    <header class="flex items-center justify-between px-6 py-4 border-b border-white/5">
      <div class="flex items-center gap-3">
        <div class="w-9 h-9 rounded-xl bg-linear-to-br from-violet-500 to-fuchsia-600 grid place-items-center shadow-lg shadow-violet-500/20">
          <Film class="w-5 h-5 text-white" />
        </div>
        <div>
          <h1 class="text-lg font-semibold text-white tracking-tight">FreeCut</h1>
          <p class="text-xs text-white/40">Video Editor</p>
        </div>
      </div>
      <button
        class="flex items-center gap-2 px-4 py-2 rounded-lg bg-violet-600 hover:bg-violet-500 text-white text-sm font-medium transition-colors shadow-lg shadow-violet-600/20"
        onclick={openNewProjectModal}
      >
        <Plus class="w-4 h-4" />
        New Project
      </button>
    </header>
    <div class="flex-1 overflow-y-auto p-6">
      {#if projects.length === 0}
        <div class="flex flex-col items-center justify-center h-full gap-4 text-white/30">
          <Film class="w-16 h-16" />
          <p class="text-lg font-medium">No projects yet</p>
          <p class="text-sm">Create a new project to get started</p>
        </div>
      {:else}
        <div class="grid grid-cols-[repeat(auto-fill,minmax(240px,1fr))] gap-4">
          {#each projects as project (project.id)}
            <div
              role="button"
              tabindex="0"
              class="group flex flex-col rounded-xl border border-white/5 bg-white/[0.02] hover:bg-white/[0.05] hover:border-white/10 transition-all duration-200 overflow-hidden text-left cursor-pointer"
              onclick={() => openProject(project.id)}
              onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') openProject(project.id); }}
            >
              <div class="aspect-video bg-linear-to-br from-zinc-800 to-zinc-900 flex items-center justify-center relative">
                <Film class="w-10 h-10 text-white/10 group-hover:text-white/20 transition-colors" />
                <button
                  class="absolute top-2 right-2 p-1 rounded bg-red-500/0 hover:bg-red-500/80 text-white/0 group-hover:text-white/60 hover:text-white! transition-all"
                  onclick={(e) => { e.stopPropagation(); deleteProject(project.id); }}
                  aria-label="Delete project"
                >
                  <Trash2 class="w-3.5 h-3.5" />
                </button>
              </div>
              <div class="p-3">
                <p class="text-sm font-medium text-white/80 truncate">{project.name}</p>
                <p class="text-xs text-white/30 mt-1">{project.updatedAt?.split("T")[0] ?? ""}</p>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- New Project Modal -->
    {#if showNewProjectModal}
      <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm">
        <div class="w-[520px] rounded-2xl border border-white/10 bg-zinc-900 shadow-2xl overflow-hidden" style="color-scheme: dark;">
          <!-- Header -->
          <div class="flex items-center justify-between px-6 pt-6 pb-2">
            <h2 class="text-white text-lg font-semibold">New Project</h2>
            <button class="p-1.5 rounded-lg hover:bg-white/10 text-white/40 hover:text-white/80 transition-colors" onclick={() => showNewProjectModal = false}>
              <Plus class="w-4 h-4 rotate-45" />
            </button>
          </div>

          <!-- Project Name -->
          <div class="px-6 pt-3 pb-4">
            <label for="new-project-name" class="block text-[11px] uppercase tracking-wider text-white/40 mb-1.5">Project Name</label>
            <input
              id="new-project-name"
              type="text"
              class="w-full bg-white/5 border border-white/10 rounded-lg px-3 py-2 text-sm text-white placeholder-white/20 focus:outline-none focus:border-violet-500/50 focus:ring-1 focus:ring-violet-500/20 transition-colors"
              placeholder="My Video Project"
              bind:value={newProjectName}
              onkeydown={(e) => { if (e.key === 'Enter') createProject(); }}
            />
          </div>

          <!-- Ratio Presets -->
          <div class="px-6 pb-2">
            <p class="text-[11px] uppercase tracking-wider text-white/40 mb-3">Canvas Ratio</p>
            <div class="grid grid-cols-3 gap-2">
              {#each ratioPresets as preset, i}
                <button
                  class="group flex flex-col items-center gap-2 p-3 rounded-xl border transition-all duration-200 {selectedRatioIdx === i ? 'border-violet-500/60 bg-violet-500/10 ring-1 ring-violet-500/20' : 'border-white/5 bg-white/[0.02] hover:bg-white/[0.04] hover:border-white/10'}"
                  onclick={() => selectedRatioIdx = i}
                >
                  <!-- Dynamic shape preview -->
                  <div class="w-full flex items-center justify-center" style="height: 48px;">
                    <div
                      class="border-2 rounded-sm transition-colors {selectedRatioIdx === i ? 'border-violet-400' : 'border-white/20 group-hover:border-white/30'}"
                      style="aspect-ratio: {preset.w}/{preset.h}; height: {preset.h > preset.w ? '100%' : 'auto'}; width: {preset.w >= preset.h ? '100%' : 'auto'}; max-height: 48px; max-width: 72px;"
                    ></div>
                  </div>
                  <div class="text-center">
                    <p class="text-xs font-semibold {selectedRatioIdx === i ? 'text-violet-300' : 'text-white/70'}">{preset.label}</p>
                    <p class="text-[9px] {selectedRatioIdx === i ? 'text-violet-300/60' : 'text-white/30'}">{preset.desc}</p>
                  </div>
                </button>
              {/each}
            </div>
          </div>

          <!-- Resolution Info -->
          <div class="px-6 py-3">
            <div class="flex items-center justify-center gap-2 px-3 py-2 rounded-lg bg-white/[0.03] border border-white/5">
              <span class="font-mono text-xs text-white/50">{selectedRatio.w} × {selectedRatio.h}</span>
              <span class="text-[10px] text-white/20">·</span>
              <span class="text-[10px] text-white/30">30 fps</span>
            </div>
          </div>

          <!-- Actions -->
          <div class="flex items-center justify-end gap-3 px-6 py-4 border-t border-white/5">
            <button class="px-5 py-2 text-sm hover:bg-white/5 rounded-lg text-white/50 transition-colors" onclick={() => showNewProjectModal = false}>Cancel</button>
            <button
              class="px-6 py-2 text-sm bg-violet-600 hover:bg-violet-500 rounded-lg text-white font-medium transition-colors shadow-lg shadow-violet-600/25"
              onclick={createProject}
            >Create Project</button>
          </div>
        </div>
      </div>
    {/if}
  </div>

<!-- ─── Editor View ──────────────────────────────────────────────────── -->
{:else if currentView === "editor" && currentProject}
  <div class="flex flex-col h-full bg-zinc-950 text-white overflow-hidden select-none">

    <!-- Top Toolbar -->
    <header class="flex items-center gap-1 px-2 py-1 bg-zinc-900/80 border-b border-white/5 shrink-0">
      <button class="p-1.5 rounded-md hover:bg-white/5 text-white/50 hover:text-white/80 transition-colors" onclick={goBack} aria-label="Back">
        <SkipBack class="w-3.5 h-3.5" />
      </button>
      <div class="w-px h-4 bg-white/10 mx-1"></div>
      <span class="text-xs font-medium text-white/50 truncate max-w-[160px] px-1">{currentProject.name}</span>
      {#if isDirty}
        <span class="w-2 h-2 rounded-full bg-amber-400 animate-pulse" title="Unsaved changes"></span>
      {:else}
        <span class="w-2 h-2 rounded-full bg-emerald-500/60" title="Saved"></span>
      {/if}
      <div class="flex-1"></div>

      <!-- Tools -->
      <button class="p-1.5 rounded-md hover:bg-white/5 text-white/40 hover:text-white/70 transition-colors" onclick={() => historyStore.getState().undo()} aria-label="Undo"><Undo2 class="w-3.5 h-3.5" /></button>
      <button class="p-1.5 rounded-md hover:bg-white/5 text-white/40 hover:text-white/70 transition-colors" onclick={() => historyStore.getState().redo()} aria-label="Redo"><Redo2 class="w-3.5 h-3.5" /></button>
      
      <div class="w-px h-4 bg-white/10 mx-2"></div>

      {#each ["select", "razor", "slip"] as tool (tool)}
        {@const ToolIcon = toolIcons[tool]}
        <button
          class="p-1.5 rounded-md transition-colors {sel.activeTool === tool ? 'bg-violet-600/30 text-violet-300' : 'hover:bg-white/5 text-white/40 hover:text-white/70'}"
          onclick={() => selectionStore.getState().setActiveTool(tool as ActiveTool)}
          aria-label={tool}
        >
          <ToolIcon class="w-3.5 h-3.5" />
        </button>
      {/each}

      <div class="w-px h-4 bg-white/10 mx-2"></div>

      <!-- Actions -->
      <button class="p-1.5 rounded-md transition-colors {isDirty ? 'text-amber-400 hover:bg-amber-500/10 hover:text-amber-300' : 'text-white/40 hover:bg-white/5 hover:text-white/70'}" onclick={saveProject} aria-label="Save (Ctrl+S)">
        <Save class="w-3.5 h-3.5" />
      </button>
      <button class="p-1.5 rounded-md hover:bg-white/5 text-white/40 hover:text-white/70 transition-colors" onclick={importMedia} aria-label="Import Media (Host)">
        <Upload class="w-3.5 h-3.5" />
      </button>
      <button class="p-1.5 rounded-md hover:bg-white/5 text-white/40 hover:text-white/70 transition-colors" onclick={openNdeImport} aria-label="Import from NDE">
        <FolderOpen class="w-3.5 h-3.5" />
      </button>

      <div class="w-px h-4 bg-white/10 mx-2"></div>

      <!-- Export -->
      <button
        class="flex items-center gap-1.5 px-3 py-1 rounded-md bg-violet-600 hover:bg-violet-500 text-white text-[11px] font-medium transition-colors shadow-sm"
        onclick={() => { showExportModal = true; }}
        disabled={ti.items.length === 0}
      >
        <Download class="w-3.5 h-3.5" />
        Export
      </button>

      <div class="w-px h-4 bg-white/10 mx-1"></div>

      <!-- Sidebar toggles -->
      <button class="p-1.5 rounded-md hover:bg-white/5 text-white/40 hover:text-white/70 transition-colors" onclick={() => editorStore.getState().toggleLeftSidebar()}>
        {#if ed.leftSidebarOpen}<PanelLeftClose class="w-3.5 h-3.5" />{:else}<PanelLeft class="w-3.5 h-3.5" />{/if}
      </button>
      <button class="p-1.5 rounded-md hover:bg-white/5 text-white/40 hover:text-white/70 transition-colors" onclick={() => editorStore.getState().toggleRightSidebar()}>
        {#if ed.rightSidebarOpen}<PanelRightClose class="w-3.5 h-3.5" />{:else}<PanelRight class="w-3.5 h-3.5" />{/if}
      </button>

      <div class="ml-2 px-2 py-0.5 rounded bg-white/5 text-[10px] text-white/30 font-mono">
        {currentProject.metadata.width}×{currentProject.metadata.height} · {fps}fps
      </div>
    </header>

    <!-- Main Editor Layout -->
    <div class="flex flex-1 min-h-0">

      <!-- Left Panel -->
      {#if ed.leftSidebarOpen}
        <aside class="flex min-w-0 flex-col border-r border-white/5 bg-zinc-900/50 shrink-0" style:width="{ed.sidebarWidth}px">
          <div class="border-b border-white/5 px-2 py-2">
            <div class="grid grid-cols-3 gap-1">
            {#each sidebarTabs as tab (tab.id)}
              {@const TabIcon = tabIcons[tab.id]}
              <button
                class="flex min-w-0 items-center justify-center gap-1 rounded-md px-1.5 py-2 text-[10px] font-medium tracking-wide transition-colors {activeTab === tab.id ? 'bg-violet-500/12 text-violet-300 ring-1 ring-violet-400/25' : 'bg-white/[0.02] text-white/45 hover:bg-white/[0.05] hover:text-white/70'}"
                onclick={() => editorStore.getState().setActiveTab(tab.id)}
                title={tab.label}
              >
                <TabIcon class="h-3 w-3 shrink-0" />
                <span class="truncate">{tab.label}</span>
              </button>
            {/each}
            </div>
          </div>

          <div class="flex-1 overflow-y-auto">
            {#if activeTab === "media"}
              <div class="p-2 space-y-1">
                <button
                  class="w-full flex items-center justify-center gap-2 px-3 py-2 rounded-lg border border-dashed border-white/10 hover:border-violet-500/30 hover:bg-violet-500/5 text-white/40 hover:text-violet-400 text-xs transition-all"
                  onclick={importMedia}
                >
                  <Upload class="w-3.5 h-3.5" />
                  Import Media
                </button>
                <button
                  class="w-full flex items-center justify-center gap-2 px-3 py-2 rounded-lg border border-dashed border-white/10 hover:border-violet-500/30 hover:bg-violet-500/5 text-white/40 hover:text-violet-400 text-xs transition-all"
                  onclick={openNdeImport}
                >
                  <FolderOpen class="w-3.5 h-3.5" />
                  Import from NDE
                </button>
                <button
                  class="w-full flex items-center justify-center gap-2 px-3 py-2 rounded-lg border border-dashed border-amber-500/20 hover:border-amber-400/40 hover:bg-amber-500/8 text-amber-400/50 hover:text-amber-300 text-xs transition-all disabled:opacity-40 disabled:cursor-wait"
                  onclick={importSrtToTimeline}
                  disabled={srtImportBusy}
                >
                  <Captions class="w-3.5 h-3.5" />
                  {srtImportBusy ? 'Importing…' : 'Import SRT to Timeline'}
                </button>
                <button
                  class="w-full flex items-center justify-center gap-2 px-3 py-2 rounded-lg border border-dashed border-fuchsia-500/20 hover:border-fuchsia-400/40 hover:bg-fuchsia-500/8 text-fuchsia-400/50 hover:text-fuchsia-300 text-xs transition-all disabled:opacity-40 disabled:cursor-wait"
                  onclick={autoGenerateSrtFromVideo}
                  disabled={autoGenBusy || mediaLibrary.filter(m => m.mediaType === 'video').length === 0}
                >
                  <Sparkles class="w-3.5 h-3.5" />
                  {#if autoGenBusy}
                    {autoGenPhase || 'Generating…'}
                  {:else}
                    Auto-Generate Subtitles
                  {/if}
                </button>
              </div>
              <div class="px-2 pb-2 space-y-0.5">
                {#each mediaLibrary as media (media.id)}
                  {@const MediaIcon = getMediaIcon(media.mediaType)}
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <div
                    role="button"
                    tabindex="0"
                    class="cursor-grab active:cursor-grabbing w-full flex items-center gap-2 px-2 py-1.5 rounded-lg transition-colors text-left group {previewMediaItem?.id === media.id ? 'bg-white/10' : 'hover:bg-white/5'}"
                    onclick={() => { previewMediaItem = media; activePreviewTab = 'media'; }}
                    onmousedown={(e) => { if (e.button === 0) startMediaPointerDrag(e, media); }}
                    ondblclick={() => addMediaToTimeline(media)}
                    oncontextmenu={(e) => { e.preventDefault(); e.stopPropagation(); contextMenuMedia = { x: e.clientX, y: e.clientY, media }; }}
                  >
                    <div class="w-10 h-7 rounded bg-white/5 grid place-items-center shrink-0 overflow-hidden">
                      {#if media.thumbnailPath}
                        <img src={assetUrl(media.thumbnailPath)} alt="" class="w-full h-full object-cover" />
                      {:else if media.mediaType === "image"}
                        <img src={assetUrl(media.filePath)} alt="" class="w-full h-full object-cover" />
                      {:else}
                        <MediaIcon class="w-4 h-4 text-white/20" />
                      {/if}
                    </div>
                    <div class="min-w-0 flex-1 pointer-events-none">
                      <p class="text-[11px] text-white/70 truncate">{media.fileName}</p>
                      <p class="text-[9px] text-white/30">
                        {formatFileSize(media.fileSize)}
                        {#if media.durationSecs} · {formatDuration(media.durationSecs)}{/if}
                      </p>
                    </div>
                    <button
                      class="shrink-0 p-1 rounded hover:bg-red-500/20 text-white/0 group-hover:text-white/30 hover:!text-red-400 transition-all"
                      onclick={(e) => { e.stopPropagation(); deleteMedia(media); }}
                      aria-label="Delete media"
                    >
                      <Trash2 class="w-3.5 h-3.5" />
                    </button>
                  </div>
                {/each}
              </div>
            {:else if activeTab === "dubbing"}
              {#if currentProject}
                <DubbingPanel
                  mediaLibrary={mediaLibrary}
                  currentProject={currentProject}
                  fps={fps}
                  saveProject={saveProject}
                  updateDubbingSession={(updater) => {
                    updateCurrentProjectDubbing(updater);
                  }}
                  createDefaultTrack={createDefaultTrack}
                />
              {/if}
            {:else if activeTab === "tools"}
              <ToolsPanel mediaLibrary={mediaLibrary} />
            {:else if activeTab === "effects"}
              <div class="p-4 flex flex-col items-center justify-center text-white/40 h-full text-center">
                <Sparkles class="w-10 h-10 mb-3 opacity-50" />
                <p class="text-sm font-medium">Effects</p>
                <p class="text-[10px] text-white/30 mt-1">Coming soon</p>
              </div>
            {:else if activeTab === "transitions"}
              <div class="p-4 flex flex-col items-center justify-center text-white/40 h-full text-center">
                <ChevronRight class="w-10 h-10 mb-3 opacity-50" />
                <p class="text-sm font-medium">Transitions</p>
                <p class="text-[10px] text-white/30 mt-1">Coming soon</p>
              </div>
            {:else if activeTab === "text"}
              <div class="p-2 space-y-2">
                <div 
                  role="button"
                  tabindex="0"
                  onmousedown={(e) => { if (e.button === 0) startTextPointerDrag(e); }}
                  class="cursor-grab flex flex-col items-center justify-center p-4 border border-white/10 rounded-lg hover:border-white/30 hover:bg-white/5 transition-colors group"
                >
                  <Type class="w-8 h-8 text-white/40 group-hover:text-white/80 transition-colors mb-2" />
                  <span class="text-xs text-white/60">Basic Title</span>
                  <span class="text-[9px] text-white/30 mt-1 text-center">Drag to timeline</span>
                </div>
              </div>
            {/if}
          </div>
        </aside>
        <div
          class="w-1.5 shrink-0 cursor-ew-resize bg-zinc-900/70 transition-colors hover:bg-violet-500/40"
          onmousedown={startLeftSidebarResize}
          role="separator"
          aria-orientation="vertical"
          aria-label="Resize left sidebar"
        ></div>
      {/if}

      <!-- Center: Preview + Timeline -->
      <div class="flex flex-col flex-1 min-w-0">
        <!-- Preview Area -->
        <div class="flex-1 flex flex-col relative min-h-[180px] bg-black/40 overflow-hidden">
          <div class="absolute top-3 left-3 flex items-center gap-1 p-1 rounded-lg border border-white/10 z-20 shadow-lg {isPlayingRaw ? 'bg-black/70' : 'bg-black/40 backdrop-blur-md'}">
            <button class="px-3 py-1 rounded-md text-[10px] font-medium transition-colors {activePreviewTab === 'timeline' ? 'bg-violet-600/90 text-white' : 'text-white/40 hover:bg-white/10 hover:text-white/80'}" onclick={() => activePreviewTab = 'timeline'}>Timeline</button>
            <button class="px-3 py-1 rounded-md text-[10px] font-medium transition-colors {activePreviewTab === 'media' ? 'bg-violet-600/90 text-white' : 'text-white/40 hover:bg-white/10 hover:text-white/80'}" onclick={() => activePreviewTab = 'media'} disabled={!previewMediaItem}>Media</button>
          </div>

          <div class="absolute inset-0 flex items-center justify-center p-4">
            {#if activePreviewTab === 'timeline'}
              <div
                class="border border-white/5 rounded-sm bg-black shadow-2xl overflow-hidden flex items-center justify-center relative"
                style="aspect-ratio: {currentProject.metadata.width} / {currentProject.metadata.height}; height: 100%; max-width: 100%; container-type: size;"
              >
                <!-- DOM Overlays for real-time preview -->
                <div class="absolute inset-0 overflow-hidden pointer-events-none">
                  <div 
                    class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 origin-center"
                    style="width: {currentProject.metadata.width}px; height: {currentProject.metadata.height}px; transform: scale(min(calc(100cqw / {currentProject.metadata.width}), calc(100cqh / {currentProject.metadata.height})));"
                  >
                    {#each activePreviewItems as item (item.id)}
                      {@const scale = item.transform?.scale ?? 1}
                      {@const x = item.transform?.x ?? (currentProject.metadata.width / 2)}
                      {@const y = item.transform?.y ?? (currentProject.metadata.height / 2)}
                      {@const opacity = item.transform?.opacity ?? 1}
                      <div
                        class="absolute will-change-transform flex items-center justify-center"
                        style="
                          width: {item.transform?.width ?? currentProject.metadata.width}px;
                          height: {item.transform?.height ?? currentProject.metadata.height}px;
                          left: 0; top: 0;
                          transform: translate({x}px, {y}px) scale({scale}) translate(-50%, -50%);
                          opacity: {opacity};
                          transform-origin: center;
                        "
                      >
                        {#if item.type === 'video'}
                          <!-- svelte-ignore a11y_media_has_caption -->
                          <video 
                            data-preview-video={item.id}
                            src={assetUrl(item.src)}
                            class="w-full h-full object-contain"
                            muted playsinline preload="auto"
                          ></video>
                        {:else if item.type === 'image'}
                          <img src={assetUrl(item.src)} alt="" class="w-full h-full object-contain" />
                        {:else if item.type === 'text'}
                          <div style="font-family: {item.fontFamily}; font-size: {item.fontSize}px; color: {item.color}; text-align: {item.textAlign || 'center'}; white-space: pre-wrap; width: 100%; text-shadow: 0 2px 4px rgba(0,0,0,0.5);">
                            {item.text}
                          </div>
                        {/if}
                      </div>
                    {/each}
                  </div>
                </div>
              </div>
              <div class="absolute bottom-3 left-1/2 -translate-x-1/2 flex items-center gap-3 px-3 py-1 rounded-md border border-white/5 pointer-events-none z-10 {isPlayingRaw ? 'bg-black/80' : 'bg-black/60 backdrop-blur-sm'}">
                <span data-tc="preview" class="font-mono text-[11px] text-white/60">{currentTime}</span>
                <span class="text-[10px] text-white/20">/</span>
                <span class="font-mono text-[11px] text-white/40">{totalTime}</span>
              </div>
            {:else if activePreviewTab === 'media' && previewMediaItem}
              {#if previewMediaItem.mediaType === 'video'}
                <!-- svelte-ignore a11y_media_has_caption -->
                <video src={assetUrl(previewMediaItem.filePath)} controls class="max-w-full max-h-full object-contain outline-none border border-white/10 rounded-lg bg-black shadow-2xl"></video>
              {:else if previewMediaItem.mediaType === 'image'}
                <img src={assetUrl(previewMediaItem.filePath)} alt="Preview" class="max-w-full max-h-full object-contain border border-white/10 rounded-lg bg-black shadow-2xl" />
              {:else if previewMediaItem.mediaType === 'audio'}
                <div class="w-full max-w-md aspect-video border border-white/10 rounded-lg bg-black shadow-2xl flex flex-col items-center justify-center p-6">
                  <Music class="w-16 h-16 text-white/10 mb-6" />
                  <!-- svelte-ignore a11y_media_has_caption -->
                  <audio src={assetUrl(previewMediaItem.filePath)} controls class="w-full"></audio>
                </div>
              {:else}
                <div class="flex flex-col items-center justify-center">
                  <p class="text-sm text-white/50">Preview not available</p>
                </div>
              {/if}
            {/if}
          </div>
        </div>

        <!-- Transport Controls -->
        <div class="flex items-center justify-center gap-1 py-1.5 bg-zinc-900/60 border-y border-white/5">
          <button class="p-1.5 rounded-md hover:bg-white/5 text-white/50 hover:text-white transition-colors" onclick={() => playbackStore.getState().setCurrentFrame(0)}>
            <SkipBack class="w-3.5 h-3.5" />
          </button>
          <button
            class="p-2 rounded-full bg-white/5 hover:bg-white/10 text-white transition-colors"
            onclick={() => playbackStore.getState().togglePlayPause()}
          >
            {#if pb.isPlaying}<Pause class="w-4 h-4" />{:else}<Play class="w-4 h-4 ml-0.5" />{/if}
          </button>
          <button class="p-1.5 rounded-md hover:bg-white/5 text-white/50 hover:text-white transition-colors" onclick={() => playbackStore.getState().setCurrentFrame(totalFrames)}>
            <SkipForward class="w-3.5 h-3.5" />
          </button>

          <div class="w-px h-4 bg-white/10 mx-2"></div>

          <!-- Loop & Volume -->
          <button class="p-1.5 rounded-md transition-colors {pb.loop ? 'bg-violet-600/20 text-violet-400' : 'hover:bg-white/5 text-white/30 hover:text-white/50'}" onclick={() => playbackStore.getState().toggleLoop()}>
            <Repeat class="w-3.5 h-3.5" />
          </button>
          <button class="p-1.5 rounded-md hover:bg-white/5 text-white/30 hover:text-white/50 transition-colors" onclick={() => playbackStore.getState().toggleMute()}>
            {#if pb.muted}<VolumeX class="w-3.5 h-3.5" />{:else}<Volume2 class="w-3.5 h-3.5" />{/if}
          </button>
        </div>

        <!-- Timeline Resize Handle -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="h-1.5 bg-zinc-800 hover:bg-violet-500/40 cursor-ns-resize flex items-center justify-center transition-colors group/resize shrink-0"
          onmousedown={startTimelineResize}
        >
          <GripVertical class="w-3 h-3 text-white/10 group-hover/resize:text-white/30 rotate-90" />
        </div>

        <!-- Timeline Area -->
        <div class="shrink-0 bg-zinc-900/30 flex flex-col" style:height="{ed.timelineHeight}px">
          <!-- Timeline toolbar -->
          <div class="h-7 border-b border-white/5 bg-zinc-900/60 flex items-center px-2 gap-1 shrink-0">
            <!-- Track controls -->
            <button class="p-0.5 rounded hover:bg-white/5 text-white/25 hover:text-white/50" onclick={() => addNewTrack('video')} aria-label="Add video track">
              <Plus class="w-3 h-3" />
            </button>
            <button class="p-0.5 rounded hover:bg-white/5 text-white/25 hover:text-white/50" onclick={() => {
              if (ti.tracks.length > 0) removeTrack(ti.tracks[ti.tracks.length - 1]!.id);
            }} aria-label="Remove last track" disabled={ti.tracks.length === 0}>
              <Minus class="w-3 h-3" />
            </button>

            <div class="w-px h-3.5 bg-white/8 mx-1"></div>

            <!-- Snap toggle -->
            <button
              class="p-0.5 rounded transition-colors {ti.snapEnabled ? 'bg-violet-600/20 text-violet-400' : 'hover:bg-white/5 text-white/25 hover:text-white/50'}"
              onclick={toggleSnap}
              aria-label="Toggle snap"
            >
              <Magnet class="w-3 h-3" />
            </button>

            <div class="flex-1"></div>

            <!-- Zoom controls -->
            <button class="p-0.5 rounded hover:bg-white/5 text-white/25 hover:text-white/50" onclick={() => zoomStore.getState().zoomOut()} aria-label="Zoom out">
              <ZoomOut class="w-3 h-3" />
            </button>
            <input
              type="range"
              min="0"
              max="1"
              step="0.005"
              value={zoomToSlider(zm.level)}
              oninput={handleZoomSlider}
              class="w-20 h-1 appearance-none bg-white/10 rounded-full cursor-pointer accent-violet-500
                [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-2.5 [&::-webkit-slider-thumb]:h-2.5
                [&::-webkit-slider-thumb]:bg-violet-400 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:cursor-pointer
                [&::-webkit-slider-thumb]:shadow-[0_0_4px_rgba(139,92,246,0.5)]"
            />
            <button class="p-0.5 rounded hover:bg-white/5 text-white/25 hover:text-white/50" onclick={() => zoomStore.getState().zoomIn()} aria-label="Zoom in">
              <ZoomIn class="w-3 h-3" />
            </button>
            <button class="p-0.5 rounded hover:bg-white/5 text-white/25 hover:text-white/50" onclick={zoomToFit} aria-label="Zoom to fit">
              <Maximize2 class="w-3 h-3" />
            </button>
            <span class="text-[9px] text-white/25 font-mono w-10 text-right">{Math.round(zm.level * 100)}%</span>
          </div>

          <!-- Timeline ruler -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            id="timeline-ruler"
            class="h-5 border-b border-white/5 bg-zinc-900/50 flex items-center shrink-0 cursor-pointer overflow-hidden relative"
            onmousedown={startPlayheadDrag}
            onwheel={handleTimelineWheel}
          >
            <!-- Track header spacer -->
            <div class="w-[120px] shrink-0 flex items-center justify-center bg-[#111] border-r border-white/5 z-20">
              <span data-tc="ruler" class="text-[9px] text-white/40 font-mono tracking-wider">{currentTime}</span>
            </div>
            <!-- Ruler area -->
            <div class="flex-1 relative h-full overflow-hidden">
              <div class="absolute top-0 h-full pointer-events-none will-change-transform" style:transform="translate3d({-ti.scrollLeft}px, 0, 0)">
                {#each Array(Math.ceil((totalFrames / fps) + 10)) as _, i}
                  {@const tickPx = frameToPixel(i * fps)}
                  {#if tickPx >= ti.scrollLeft - 100 && tickPx <= ti.scrollLeft + 2000}
                    <!-- Major tick (every second) — only render visible range -->
                    <div class="absolute bottom-0" style:left="{tickPx}px">
                      <div class="w-px h-3 bg-white/15"></div>
                      <span class="absolute top-0 left-1 text-[7px] text-white/20 select-none">{i}s</span>
                    </div>
                    <!-- Half-second subdivisions (shown when zoomed enough) -->
                    {#if zm.pixelsPerSecond > 60}
                      <div class="absolute bottom-0" style:left="{frameToPixel(i * fps + Math.round(fps / 2))}px">
                        <div class="w-px h-1.5 bg-white/8"></div>
                      </div>
                    {/if}
                    <!-- Quarter-second subdivisions (shown when more zoomed) -->
                    {#if zm.pixelsPerSecond > 150}
                      <div class="absolute bottom-0" style:left="{frameToPixel(i * fps + Math.round(fps / 4))}px">
                        <div class="w-px h-1 bg-white/5"></div>
                      </div>
                      <div class="absolute bottom-0" style:left="{frameToPixel(i * fps + Math.round(fps * 3 / 4))}px">
                        <div class="w-px h-1 bg-white/5"></div>
                      </div>
                    {/if}
                  {/if}
                {/each}
              </div>
              <!-- Playhead indicator on ruler -->
              <div data-playhead="ruler" class="absolute top-0 bottom-0 w-px bg-white z-20 pointer-events-none will-change-transform drop-shadow-[0_0_2px_rgba(255,255,255,0.5)]" style:transform="translate3d({frameToPixel(pb.currentFrame) - ti.scrollLeft}px, 0, 0)">
                <div class="absolute top-0 -left-[5px] w-[11px] h-[14px] bg-white rounded-b-sm flex items-center justify-center">
                  <div class="w-px h-2 bg-black/30"></div>
                </div>
              </div>

              <!-- In/Out point indicators on ruler -->
              {#if inPoint !== null}
                <div class="absolute top-0 bottom-0 w-0.5 bg-cyan-400 z-10 pointer-events-none will-change-transform" style:transform="translate3d({frameToPixel(inPoint) - ti.scrollLeft}px, 0, 0)">
                  <span class="absolute -top-0.5 left-0.5 text-[6px] text-cyan-400 font-bold">I</span>
                </div>
              {/if}
              {#if outPoint !== null}
                <div class="absolute top-0 bottom-0 w-0.5 bg-cyan-400 z-10 pointer-events-none will-change-transform" style:transform="translate3d({frameToPixel(outPoint) - ti.scrollLeft}px, 0, 0)">
                  <span class="absolute -top-0.5 -left-2 text-[6px] text-cyan-400 font-bold">O</span>
                </div>
              {/if}
              {#if inPoint !== null && outPoint !== null}
                <div class="absolute top-0 bottom-0 bg-cyan-400/10 pointer-events-none z-5 will-change-transform" style:transform="translate3d({frameToPixel(inPoint) - ti.scrollLeft}px, 0, 0)" style:width="{frameToPixel(outPoint - inPoint)}px"></div>
              {/if}

              <!-- Preview scrubber ghost on ruler -->
              {#if previewFrame !== null && !isDraggingPlayhead}
                <div class="absolute top-0 bottom-0 w-px bg-white/20 z-5 pointer-events-none will-change-transform" style:transform="translate3d({frameToPixel(previewFrame) - ti.scrollLeft}px, 0, 0)"></div>
              {/if}

              <!-- Marker diamonds on ruler -->
              {#each markers as marker (marker.id)}
                <div class="absolute top-0 bottom-0 pointer-events-none z-10 will-change-transform" style:transform="translate3d({frameToPixel(marker.frame) - ti.scrollLeft}px, 0, 0)">
                  <div class="absolute top-0 -left-[3px] w-[7px] h-[7px] rotate-45 shadow-sm" style:background={marker.color}></div>
                </div>
              {/each}
            </div>
          </div>

          <!-- Tracks Area -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            id="tracks-container"
            class="flex-1 overflow-y-auto overflow-x-hidden relative"
            onwheel={handleTimelineWheel}
            onmousemove={handleTrackMouseMove}
            onmouseleave={handleTrackMouseLeave}
          >
            {#if ti.tracks.length === 0}
              <div class="flex items-center justify-center h-full text-white/15 text-xs pointer-events-none">
                <div class="flex flex-col items-center gap-2">
                  <Layers class="w-8 h-8" />
                  <p>Drag media here to add to timeline</p>
                </div>
              </div>
            {:else}
              <div class="relative w-full" style:min-width="{frameToPixel(totalFrames) + 200}px">
                {#each ti.tracks as track, trackIndex (track.id)}
                  <div class="flex border-b border-white/3 flex-col relative w-full group/track" style:height="{track.height}px">
                    
                    <!-- Track Header (Sticky left) -->
                    <div class="absolute left-0 top-0 h-full w-[120px] z-20 flex items-center gap-1 px-2 border-r border-white/5 bg-[#121212] shrink-0">
                      <div class="w-1.5 h-4 rounded-sm" style:background={trackColors[trackIndex % trackColors.length]}></div>
                      <span class="text-[10px] text-white/50 truncate flex-1 block">{track.name}</span>
                      <button class="p-0.5 rounded pointer-events-auto hover:bg-white/5 text-white/30" onclick={() => itemsStore.getState().toggleTrackMute(track.id)}>
                        {#if track.muted}<VolumeX class="w-2.5 h-2.5" />{:else}<Volume2 class="w-2.5 h-2.5" />{/if}
                      </button>
                      <button class="p-0.5 rounded pointer-events-auto hover:bg-white/5 text-white/30" onclick={() => itemsStore.getState().toggleTrackVisibility(track.id)}>
                        {#if !track.visible}<EyeOff class="w-2.5 h-2.5" />{:else}<Eye class="w-2.5 h-2.5" />{/if}
                      </button>
                      <button class="p-0.5 rounded pointer-events-auto opacity-0 group-hover/track:opacity-100 hover:bg-red-500/20 text-white/20 hover:text-red-400 transition-all" onclick={() => removeTrack(track.id)} aria-label="Remove track">
                        <Trash2 class="w-2.5 h-2.5" />
                      </button>
                    </div>

                    <!-- Track Canvas -->
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <div class="absolute top-0 bottom-0 right-0 bg-[#161616] overflow-hidden" style="left: 120px;">
                      <div class="relative w-full h-full will-change-transform" style:transform="translate3d({-ti.scrollLeft}px, 0, 0)">
                        
                        <!-- Clips -->
                        {#each (ti.itemsByTrackId[track.id] ?? []) as item (item.id)}
                          {@const startPx = frameToPixel(item.from)}
                          {@const widthPx = frameToPixel(item.durationInFrames)}
                          {@const isSelected = sel.selectedItemIds.includes(item.id)}
                          {@const color = trackColors[trackIndex % trackColors.length]}
                          {@const MediaIcon2 = getMediaIcon(item.type)}
                          {@const media = item.mediaId ? mediaLibrary.find((m) => m.id === item.mediaId) : null}
                          <div
                            role="button"
                            tabindex="-1"
                            class="absolute top-[3px] rounded-md cursor-pointer select-none overflow-hidden"
                            style:left="{startPx}px"
                            style:width="{Math.max(8, widthPx)}px"
                            style:height="calc(100% - 6px)"
                            style:background="{isSelected ? `${color}50` : `${color}30`}"
                            style:box-shadow="inset 0 0 0 1px {isSelected ? '#ffffff' : `${color}40`}{isSelected ? ', 0 0 0 2px rgba(255,255,255,1)' : ''}"
                            onclick={(e) => handleClipClick(e, item)}
                            onmousedown={(e) => startClipDrag(e, item.id, item.from, item.trackId)}
                          >
                            {#if media && (item.type === "image" || item.type === "video")}
                              {@const thumb = item.type === "image" ? media.filePath : (media.thumbnailPath ?? "")}
                              {#if thumb}
                                <div 
                                  class="absolute inset-0 opacity-100 pointer-events-none bg-black/40 mix-blend-screen"
                                  style:background-image="url('{assetUrl(thumb)}')"
                                  style:background-size="auto 100%"
                                  style:background-repeat="repeat-x"
                                ></div>
                              {/if}
                            {/if}

                            <div class="relative z-10 flex items-center gap-1.5 px-2 h-full pointer-events-none bg-black/20 w-full">
                              <div class="p-1 rounded-sm bg-black/40 backdrop-blur-sm">
                                <MediaIcon2 class="w-2.5 h-2.5 shrink-0" style="color: {color}; filter: drop-shadow(0 1px 1px rgba(0,0,0,1));" />
                              </div>
                              <span class="text-[10px] font-medium text-white tracking-wide truncate pr-4" style="text-shadow: 0 1px 2px rgba(0,0,0,0.8), 0 0 4px rgba(0,0,0,1);">{item.label}</span>
                            </div>
                            
                            <!-- Trim Handles -->
                            <div
                              role="slider"
                              aria-valuenow={item.from}
                              tabindex="-1"
                              data-drag-mode="trim"
                              class="absolute top-0 bottom-0 left-0 w-2 cursor-ew-resize hover:bg-white/30 z-10 rounded-l-[2px]"
                              onmousedown={(e) => startTrim(e, item.id, "left")}
                            ></div>
                            <div
                              role="slider"
                              aria-valuenow={item.from + item.durationInFrames}
                              tabindex="-1"
                              data-drag-mode="trim"
                              class="absolute top-0 bottom-0 right-0 w-2 cursor-ew-resize hover:bg-white/30 z-10 rounded-r-[2px]"
                              onmousedown={(e) => startTrim(e, item.id, "right")}
                            ></div>

                            {#if item.type === "audio" && item.waveformData}
                              <div class="absolute inset-0 flex items-end px-px opacity-30 pointer-events-none">
                                {#each item.waveformData.slice(0, Math.floor(widthPx / 2)) as peak}
                                  <div class="w-px mx-px rounded-t" style:height="{peak * 100}%" style:background={color}></div>
                                {/each}
                              </div>
                            {/if}

                            <!-- Timeline Keyframes Presentation (Professional Edition) -->
                            {#if item.keyframes && item.keyframes.length > 0}
                              <!-- Draw a faint line connecting keyframes -->
                              <div class="absolute bottom-1.5 left-0 right-0 h-px bg-white/10 pointer-events-none mx-2"></div>
                              <div class="absolute bottom-0 left-0 right-0 h-3 pointer-events-none">
                                {#each item.keyframes as kf}
                                  <div 
                                    class="absolute w-1.5 h-1.5 bg-amber-400 rotate-45 transform -translate-x-1/2 translate-y-0.5 rounded-sm shadow-[0_0_4px_rgba(251,191,36,0.6)]"
                                    style:left="{frameToPixel(kf.frameOffset)}px"
                                  ></div>
                                {/each}
                              </div>
                            {/if}
                          </div>
                        {/each}

                        <!-- Playhead through tracks -->
                        <div data-playhead="track" class="absolute top-0 bottom-0 w-px bg-white drop-shadow-[0_0_2px_rgba(255,255,255,0.7)] pointer-events-none z-10 will-change-transform" style:transform="translate3d({frameToPixel(pb.currentFrame)}px, 0, 0)"></div>

                        <!-- Active snap guide line (shown during drag) -->
                        {#if activeSnapFrame !== null && isDraggingClip}
                          <div class="absolute top-0 bottom-0 w-px bg-yellow-400 pointer-events-none z-20 opacity-80" style:left="{frameToPixel(activeSnapFrame)}px">
                            <div class="absolute top-0 w-px h-full shadow-[0_0_6px_rgba(250,204,21,0.6)]"></div>
                          </div>
                        {/if}

                        <!-- In/Out point shading through tracks -->
                        {#if inPoint !== null && outPoint !== null}
                          <div class="absolute top-0 bottom-0 bg-cyan-400/5 border-l border-r border-cyan-400/20 pointer-events-none z-5" style:left="{frameToPixel(inPoint)}px" style:width="{frameToPixel(outPoint - inPoint)}px"></div>
                        {/if}

                        <!-- Marker lines through tracks -->
                        {#each markers as marker (marker.id)}
                          <div class="absolute top-0 bottom-0 w-px pointer-events-none z-10 opacity-60" style:left="{frameToPixel(marker.frame)}px" style:background={marker.color}></div>
                        {/each}

                        <!-- Preview scrubber ghost line -->
                        {#if previewFrame !== null && !isDraggingClip && !isTrimming}
                          <div class="absolute top-0 bottom-0 w-px bg-white/15 pointer-events-none z-5" style:left="{frameToPixel(previewFrame)}px"></div>
                        {/if}

                        <!-- Snap indicator lines (at second boundaries when snap enabled) — virtualized -->
                        {#if ti.snapEnabled && zm.pixelsPerSecond > 40}
                          {#each Array(Math.ceil((totalFrames / fps) + 5)) as _, i}
                            {@const snapPx = frameToPixel(i * fps)}
                            {#if snapPx >= ti.scrollLeft - 50 && snapPx <= ti.scrollLeft + 2000}
                              <div class="absolute top-0 bottom-0 w-px bg-white/2 pointer-events-none" style:left="{snapPx}px"></div>
                            {/if}
                          {/each}
                        {/if}
                      </div>
                    </div>

                  </div>
                {/each}
              </div>
            {/if}
          </div>
        </div>
      </div>

      <!-- Right Panel: Properties -->
      {#if ed.rightSidebarOpen}
        <aside class="flex flex-col border-l border-white/5 bg-zinc-900/40 shrink-0" style:width="{ed.rightSidebarWidth}px">
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

                <!-- Volume & Audio -->
                {#if selectedItem.type === "video" || selectedItem.type === "audio"}
                  <div class="border-t border-white/5 pt-3">
                    <span class="block text-[10px] text-white/30 uppercase tracking-wider mb-2">Audio</span>
                    <div class="space-y-3">
                      <div>
                        <div class="flex items-center justify-between mb-1.5">
                          <span class="text-[9px] text-white/40 font-medium">Volume</span>
                          <span class="text-[9px] text-white/50 font-mono">{Math.round((selectedItem.volume ?? 1) * 100)}%</span>
                        </div>
                        <div class="flex items-center gap-2">
                          <button
                            class="text-white/40 hover:text-white/70 transition-colors text-[11px] shrink-0"
                            aria-label="Toggle mute"
                            onclick={() => updateSelectedItem({ volume: (selectedItem?.volume ?? 1) === 0 ? 1 : 0 })}
                          >
                            {#if (selectedItem.volume ?? 1) === 0}🔇{:else if (selectedItem.volume ?? 1) < 0.5}🔈{:else}🔊{/if}
                          </button>
                          <input
                            type="range"
                            min="0"
                            max="2"
                            step="0.01"
                            value={selectedItem.volume ?? 1}
                            oninput={(e) => updateSelectedItem({ volume: parseFloat(e.currentTarget.value) })}
                            class="flex-1 h-1 accent-violet-500 cursor-pointer"
                          />
                        </div>
                      </div>
                      <div class="grid grid-cols-2 gap-2">
                        <div>
                          <span class="text-[9px] text-white/40 block mb-1">Fade In (frames)</span>
                          <input type="number" min="0" step="1" class="w-full bg-white/5 border border-white/10 rounded px-2 py-1 text-xs text-white font-mono focus:border-violet-500/50 focus:outline-none transition-colors" value={selectedItem.audioFadeIn ?? 0} onchange={(e) => updateSelectedItem({ audioFadeIn: parseInt(e.currentTarget.value) || 0 })} />
                        </div>
                        <div>
                          <span class="text-[9px] text-white/40 block mb-1">Fade Out (frames)</span>
                          <input type="number" min="0" step="1" class="w-full bg-white/5 border border-white/10 rounded px-2 py-1 text-xs text-white font-mono focus:border-violet-500/50 focus:outline-none transition-colors" value={selectedItem.audioFadeOut ?? 0} onchange={(e) => updateSelectedItem({ audioFadeOut: parseInt(e.currentTarget.value) || 0 })} />
                        </div>
                      </div>
                    </div>
                  </div>
                {/if}

                <!-- Speed -->
                {#if selectedItem.type === "video" || selectedItem.type === "audio"}
                  <div class="border-t border-white/5 pt-3">
                    <span class="block text-[10px] text-white/30 uppercase tracking-wider mb-2">Speed</span>
                    <div class="space-y-2">
                      <div class="flex items-center justify-between mb-1">
                        <span class="text-[9px] text-white/40 font-medium">Playback Rate</span>
                        <span class="text-[9px] text-white/50 font-mono">{(selectedItem.speed ?? 1).toFixed(2)}x</span>
                      </div>
                      <div class="flex items-center gap-2">
                        <input
                          type="range"
                          min="0.25"
                          max="4"
                          step="0.05"
                          value={selectedItem.speed ?? 1}
                          oninput={(e) => updateSelectedItem({ speed: parseFloat(e.currentTarget.value) })}
                          class="flex-1 h-1 accent-violet-500 cursor-pointer"
                        />
                      </div>
                      <div class="flex gap-1 flex-wrap">
                        {#each [0.25, 0.5, 1, 1.5, 2, 4] as preset}
                          <button
                            class="px-2 py-0.5 rounded text-[9px] font-medium transition-colors {(selectedItem.speed ?? 1) === preset ? 'bg-violet-600 text-white' : 'bg-white/5 text-white/50 hover:bg-white/10 hover:text-white/70'}"
                            onclick={() => updateSelectedItem({ speed: preset })}
                          >
                            {preset}x
                          </button>
                        {/each}
                      </div>
                    </div>
                  </div>
                {/if}

                <!-- Video Fades -->
                {#if selectedItem.type === "video" || selectedItem.type === "image"}
                  <div class="border-t border-white/5 pt-3">
                    <span class="block text-[10px] text-white/30 uppercase tracking-wider mb-2">Video Fade</span>
                    <div class="grid grid-cols-2 gap-2">
                      <div>
                        <span class="text-[9px] text-white/40 block mb-1">Fade In (frames)</span>
                        <input type="number" min="0" step="1" class="w-full bg-white/5 border border-white/10 rounded px-2 py-1 text-xs text-white font-mono focus:border-violet-500/50 focus:outline-none transition-colors" value={selectedItem.fadeIn ?? 0} onchange={(e) => updateSelectedItem({ fadeIn: parseInt(e.currentTarget.value) || 0 })} />
                      </div>
                      <div>
                        <span class="text-[9px] text-white/40 block mb-1">Fade Out (frames)</span>
                        <input type="number" min="0" step="1" class="w-full bg-white/5 border border-white/10 rounded px-2 py-1 text-xs text-white font-mono focus:border-violet-500/50 focus:outline-none transition-colors" value={selectedItem.fadeOut ?? 0} onchange={(e) => updateSelectedItem({ fadeOut: parseInt(e.currentTarget.value) || 0 })} />
                      </div>
                    </div>
                  </div>
                {/if}

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
      {/if}
    </div>
  </div>
{/if}

<!-- Export Modal hidden for brevity but completely preserved logic -->
{#if showExportModal}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
    <div class="w-[480px] rounded-2xl border border-white/10 bg-zinc-900 shadow-2xl overflow-hidden p-6" style="color-scheme: dark;">
      <h2 class="text-white text-lg font-medium mb-5">Export Video</h2>
      
      <div class="space-y-4 mb-6">
        <div>
          <label for="export-codec" class="block text-xs uppercase text-white/50 mb-1.5">Codec</label>
          <select id="export-codec" class="w-full bg-[#1c1c1c] border border-white/10 rounded px-3 py-2 text-sm text-white [&>option]:bg-zinc-900 [&>option]:text-white focus:outline-none focus:border-violet-500/50" bind:value={exportCodec}>
            <option value="h264">H.264 (MP4)</option>
            <option value="hevc">H.265 / HEVC (MP4)</option>
            <option value="vp9">VP9 (WebM)</option>
          </select>
        </div>
        
        <div>
          <label for="export-quality" class="block text-xs uppercase text-white/50 mb-1.5">Quality Profile</label>
          <select id="export-quality" class="w-full bg-[#1c1c1c] border border-white/10 rounded px-3 py-2 text-sm text-white [&>option]:bg-zinc-900 [&>option]:text-white focus:outline-none focus:border-violet-500/50" bind:value={exportQuality}>
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
          <select id="export-hwaccel" class="w-full bg-[#1c1c1c] border border-white/10 rounded px-3 py-2 text-sm text-white [&>option]:bg-zinc-900 [&>option]:text-white focus:outline-none focus:border-violet-500/50" bind:value={exportHwAccel}>
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
          <button class="px-5 py-2 text-sm hover:bg-white/5 rounded-md text-white/50 transition-colors" onclick={() => showExportModal = false}>Cancel</button>
          <button class="px-6 py-2 text-sm bg-violet-600 hover:bg-violet-500 rounded-md text-white font-medium transition-colors shadow-[0_0_15px_rgba(124,58,237,0.4)]" onclick={exportVideo}>Export Now</button>
        {/if}
      </div>
      
      {#if exportError}
        <div class="mt-4 p-3 rounded-md bg-red-500/10 border border-red-500/20 text-red-400 text-xs">
          <strong>Export Failed:</strong> {exportError}
        </div>
      {/if}
    </div>
  </div>
{/if}

<!-- Context Menu overlay -->
{#if contextMenuMedia}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-[9999]"
    onclick={closeContextMenu}
    oncontextmenu={(e) => { e.preventDefault(); closeContextMenu(); }}
    onkeydown={undefined}
  >
    <div
      class="absolute min-w-[180px] py-1 rounded-xl bg-zinc-900 border border-white/10 shadow-xl shadow-black/50"
      style="left: {contextMenuMedia.x}px; top: {contextMenuMedia.y}px;"
    >
      <!-- Add to Timeline -->
      <button class="flex items-center gap-2 w-[calc(100%-8px)] px-2.5 py-1.5 text-[11px] text-left bg-transparent border-none cursor-default rounded-md mx-1 hover:bg-violet-500/20 text-white/80 hover:text-white transition-colors" onclick={() => { if (contextMenuMedia?.media) addMediaToTimeline(contextMenuMedia.media); closeContextMenu(); }}>
        <Plus class="w-3.5 h-3.5 opacity-70" /> Add to Timeline
      </button>
      {#if contextMenuMedia.media.mediaType === 'video'}
        <div class="h-px mx-2 my-1 bg-white/10"></div>
        <!-- Generate SRT submenu -->
        <div class="px-2.5 py-1 text-[9px] uppercase tracking-wider text-white/30 font-semibold">Generate Subtitles</div>
        <button
          class="flex items-center gap-2 w-[calc(100%-8px)] px-2.5 py-1.5 text-[11px] text-left bg-transparent border-none cursor-default rounded-md mx-1 hover:bg-amber-500/20 text-amber-400/80 hover:text-amber-300 transition-colors disabled:opacity-40 disabled:cursor-wait"
          disabled={autoGenBusy}
          onclick={() => { const m = contextMenuMedia?.media; closeContextMenu(); if (m) autoGenerateSrtFromVideo(m, 'km'); }}
        >
          <Sparkles class="w-3.5 h-3.5 opacity-70" /> ខ្មែរ (Khmer)
        </button>
        <button
          class="flex items-center gap-2 w-[calc(100%-8px)] px-2.5 py-1.5 text-[11px] text-left bg-transparent border-none cursor-default rounded-md mx-1 hover:bg-blue-500/20 text-blue-400/80 hover:text-blue-300 transition-colors disabled:opacity-40 disabled:cursor-wait"
          disabled={autoGenBusy}
          onclick={() => { const m = contextMenuMedia?.media; closeContextMenu(); if (m) autoGenerateSrtFromVideo(m, 'en'); }}
        >
          <Sparkles class="w-3.5 h-3.5 opacity-70" /> English
        </button>
      {/if}
      <div class="h-px mx-2 my-1 bg-white/10"></div>
      <!-- Delete Media -->
      <button class="flex items-center gap-2 w-[calc(100%-8px)] px-2.5 py-1.5 text-[11px] text-left bg-transparent border-none cursor-default rounded-md mx-1 hover:bg-red-500/20 text-red-400 hover:text-red-300 transition-colors" onclick={() => { if (contextMenuMedia?.media) deleteMedia(contextMenuMedia.media); closeContextMenu(); }}>
        <Trash2 class="w-3.5 h-3.5 opacity-70" /> Delete Media
      </button>
    </div>
  </div>
{/if}

{#if isPointerDragging && pendingDragPayload}
  <div
    class="fixed z-[9999] pointer-events-none px-3 py-1.5 rounded-lg bg-violet-600/90 text-white text-xs font-medium shadow-xl backdrop-blur-sm border border-violet-400/30 whitespace-nowrap"
    style:left="{pointerDragGhostX + 14}px"
    style:top="{pointerDragGhostY + 14}px"
  >
    {#if pendingDragPayload.source === "media" && pendingDragPayload.media}
      {pendingDragPayload.media.fileName}
    {:else}
      Title Text
    {/if}
  </div>
{/if}

<!-- ─── NDE Media Import Modal (Using FileExplorer) ───────────────────── -->
{#if showNdeExplorerModal}
  <div class="fixed inset-0 z-[9999] flex items-center justify-center bg-black/60 backdrop-blur-sm">
    <div class="w-[900px] h-[650px] flex flex-col rounded-2xl border border-white/10 bg-zinc-950 shadow-2xl overflow-hidden" style="color-scheme: dark;">
      <div class="flex items-center justify-between px-6 pt-5 pb-4 border-b border-white/5 bg-zinc-900 flex-shrink-0">
        <h2 class="text-white text-lg font-semibold flex items-center gap-2">
          <FolderOpen class="w-5 h-5 text-violet-400" />
          Select Media from NDE Workspace
        </h2>
        <button class="p-1.5 rounded-lg hover:bg-white/10 text-white/40 hover:text-white/80 transition-colors" onclick={() => showNdeExplorerModal = false}>
          <Plus class="w-5 h-5 rotate-45" />
        </button>
      </div>
      <div class="flex-1 relative overflow-hidden bg-zinc-950 dark">
        <FileExplorer window={{ data: { onSelectFile: (path: string) => { showNdeExplorerModal = false; handleNdeImport({ path }); } } }} />
      </div>
    </div>
  </div>
{/if}

<!-- ─── Unsaved Changes Modal ─────────────────────────────────────────── -->
{#if showUnsavedModal}
  <div class="fixed inset-0 z-[9999] flex items-center justify-center bg-black/60 backdrop-blur-sm">
    <div class="w-[380px] rounded-2xl border border-white/10 bg-zinc-900 shadow-2xl overflow-hidden" style="color-scheme: dark;">
      <div class="px-6 pt-6 pb-3">
        <h2 class="text-white text-base font-semibold">Unsaved Changes</h2>
        <p class="text-[12px] text-white/50 mt-2 leading-relaxed">
          You have unsaved changes. Would you like to save before leaving?
        </p>
      </div>
      <div class="flex items-center justify-end gap-2 px-6 py-4 border-t border-white/5">
        <button
          class="px-4 py-2 text-[12px] text-white/50 hover:bg-white/5 rounded-lg transition-colors"
          onclick={handleUnsavedCancel}
        >Cancel</button>
        <button
          class="px-4 py-2 text-[12px] text-red-400 hover:bg-red-500/10 rounded-lg font-medium transition-colors"
          onclick={handleUnsavedDiscard}
        >Discard</button>
        <button
          class="px-5 py-2 text-[12px] bg-violet-600 hover:bg-violet-500 text-white rounded-lg font-medium transition-colors shadow-lg shadow-violet-600/25"
          onclick={handleUnsavedSave}
        >Save & Exit</button>
      </div>
    </div>
  </div>
{/if}
