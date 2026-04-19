<svelte:options runes={true} />

<script lang="ts">
  import { invoke, convertFileSrc } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount, onDestroy } from "svelte";
  import { ChevronRight } from "@lucide/svelte";

  import type { 
    MediaItem, Project, DubbingSession, DubbingToolReport, DubbingSpeaker,
    DubbingSegment, DubbingImportResult, DubbingProgress, DubbingRuntimeInstallResult,
    WhisperSettings, DubStudioJob, DubStudioPart
  } from "../types";
  import type { TimelineItem } from "../stores";
  import { itemsStore } from "../stores/items";
  import { historyStore } from "../stores/history";

  interface Props {
    mediaLibrary: MediaItem[];
    currentProject: Project;
    fps: number;
    saveProject: () => Promise<void>;
    updateDubbingSession: (updater: (s: DubbingSession) => DubbingSession) => void;
    createDefaultTrack: (kind: "video" | "audio") => string;
  }
  let { mediaLibrary, currentProject, fps, saveProject, updateDubbingSession, createDefaultTrack }: Props = $props();

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
      llm: { enabled: false, model: null, mode: "translate" },
      updatedAt: null,
      lastGeneratedAt: null,
      ...partial,
    };
  }

  let dubbingSession = $derived(currentProject.dubbing ?? createDefaultDubbingSession());
  
  function patchCurrentProjectDubbing(patch: Partial<DubbingSession>) {
    updateDubbingSession(session => ({ ...session, ...patch }));
  }

  let dubStudioJob = $state<DubStudioJob | null>(null);
  let dubStudioBusy = $state(false);
  let dubStudioError = $state<string | null>(null);
  let dubStudioStatus = $state<string | null>(null);
  let dubStudioSegDuration = $state(60);
  let dubStudioDubbingPart = $state<number | null>(null);
  let dubStudioMerging = $state(false);
  let dubStudioEditingSrt = $state<{ partIndex: number; content: string } | null>(null);
  let dubStudioTargetLang = $state('km');
  let dubStudioDubAllBusy = $state(false);
  let dubStudioSavingSrt = $state(false);

  let dubbingTools = $state<DubbingToolReport | null>(null);
  let dubbingBusy = $state(false);
  let dubbingError = $state<string | null>(null);
  let dubbingStatus = $state<string | null>(null);
  let dubbingProgress = $state<DubbingProgress | null>(null);
  let runtimeInstallBusy = $state(false);
  let setupCopyStatus = $state<string | null>(null);
  let whisperModel = $state("base");
  let whisperLanguage = $state("auto");
  let diarizeEnabled = $state(false);
  let hfToken = $state("");
  let diarizeMinSpeakers = $state<number | null>(null);
  let diarizeMaxSpeakers = $state<number | null>(null);

  let needsDubbingSetup = $derived.by(() => {
    if (!dubbingTools) return true;
    return !dubbingTools.whisperAvailable || !dubbingTools.edgeTtsAvailable;
  });
  let readyDubSegments = $derived(dubbingSession.segments.filter((segment: DubbingSegment) => segment.audioPath).length);
  let activeSpeakerCount = $derived(dubbingSession.speakers.length);

    function mergeDubbingSpeakers(current: DubbingSpeaker[], incoming: DubbingSpeaker[]): DubbingSpeaker[] {
    const byId = new Map(current.map((speaker) => [speaker.id, speaker]));
    for (const speaker of incoming) {
      byId.set(speaker.id, {
        ...speaker,
        ...(byId.get(speaker.id) ?? {}),
        rvc: {
          enabled: false,
          pythonPath: null,
          cliPath: null,
          modelPath: null,
          indexPath: null,
          pitchShift: 0,
          enabled: speaker.rvc?.enabled ?? false, ...(speaker.rvc ?? {}),
          ...(byId.get(speaker.id)?.rvc ?? {}),
        },
      });
    }
    return [...byId.values()].sort((left, right) => left.label.localeCompare(right.label));
  }

    function updateDubbingSpeaker(speakerId: string, patch: Partial<DubbingSpeaker>) {
    updateDubbingSession((session: DubbingSession) => ({
      ...session,
      speakers: session.speakers.map((speaker) =>
        speaker.id === speakerId
          ? { ...speaker, ...patch, rvc: { enabled: speaker.rvc?.enabled ?? false, ...(speaker.rvc ?? {}), ...(patch.rvc ?? {}) } }
          : speaker
      ),
    }));
  }

    function updateDubbingSegment(segmentId: string, patch: Partial<DubbingSegment>) {
    updateDubbingSession((session: DubbingSession) => ({
      ...session,
      segments: session.segments.map((segment) =>
        segment.id === segmentId ? { ...segment, ...patch } : segment
      ),
    }));
  }

    function addDubbingSpeaker() {
    updateDubbingSession((session: DubbingSession) => ({
      ...session,
      speakers: [
        ...session.speakers,
        {
          id: crypto.randomUUID(),
          label: `Speaker ${session.speakers.length + 1}`,
          voice: dubbingTools?.edgeVoices?.[0] ?? "en-US-AriaNeural",
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
        },
      ],
    }));
  }

    function removeDubbingSpeaker(speakerId: string) {
    updateDubbingSession((session: DubbingSession) => {
      const remaining = session.speakers.filter((speaker) => speaker.id !== speakerId);
      const fallback = remaining[0]?.id ?? "speaker-narrator";
      return {
        ...session,
        speakers: remaining.length > 0 ? remaining : createDefaultDubbingSession().speakers,
        segments: session.segments.map((segment) =>
          segment.speakerId === speakerId ? { ...segment, speakerId: fallback } : segment
        ),
      };
    });
  }

    function ensureSourceMediaSelection() {
    if (!currentProject) return;
    const media = mediaLibrary.find((item) => item.id === dubbingSession.sourceMediaId)
      ?? mediaLibrary.find((item) => item.mediaType === "video")
      ?? mediaLibrary.find((item) => item.mediaType === "audio")
      ?? null;
    if (!media) return;
    patchCurrentProjectDubbing({
      sourceMediaId: media.id,
      sourceMediaPath: media.filePath,
    });
  }

    async function detectDubbingTools() {
    try {
      dubbingError = null;
      dubbingTools = await invoke("freecut_detect_dubbing_tools");
      dubbingStatus = "Refreshed local dubbing tools";
      if ((dubbingTools?.edgeVoices?.length ?? 0) > 0 && currentProject?.dubbing?.speakers?.length) {
        updateDubbingSession((session: DubbingSession) => ({
          ...session,
          speakers: session.speakers.map((speaker, index) => ({
            ...speaker,
            voice: speaker.voice?.trim() ? speaker.voice : dubbingTools!.edgeVoices[index % dubbingTools!.edgeVoices.length]!,
          })),
        }));
      }
    } catch (e: any) {
      dubbingError = e?.toString?.() ?? "Failed to detect local dubbing tools";
    }
  }

    async function installDubbingRuntime(runtime: "core" | "whisper" | "edge_tts" | "diarization") {
    try {
      runtimeInstallBusy = true;
      dubbingError = null;
      const result: DubbingRuntimeInstallResult = await invoke("freecut_install_dubbing_runtime", { runtime });
      dubbingStatus = result.message;
      setupCopyStatus = result.message;
      await detectDubbingTools();
    } catch (e: any) {
      dubbingError = e?.toString?.() ?? "Failed to install dubbing runtime";
    } finally {
      runtimeInstallBusy = false;
    }
  }

    function openServiceHubForVoice() {
    import("🍎/state/desktop.svelte").then(({ openServiceHub }) => {
      openServiceHub({ require: ["voice-runtime"], returnTo: "freecut" });
    });
  }

    function openServiceHubForWhisperX() {
    import("🍎/state/desktop.svelte").then(({ openServiceHub }) => {
      openServiceHub({ require: ["whisperx"], returnTo: "freecut", autoInstall: true });
    });
  }

    function openSettingsApiKeys() {
    import("🍎/state/desktop.svelte").then(({ openStaticApp }) => {
      openStaticApp("settings" as any, { tab: "api-keys", returnTo: "freecut" });
    });
  }

    async function reloadGlobalHfToken() {
    try {
      const globalSettings = await invoke<{ hfToken?: string | null }>("get_global_settings");
      if (globalSettings.hfToken) hfToken = globalSettings.hfToken;
      else if (!globalSettings.hfToken) hfToken = "";
    } catch {}
  }

    async function importDubbingSrt() {
    if (!currentProject) return;
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "Subtitles", extensions: ["srt"] }],
      });
      if (!selected || Array.isArray(selected) || typeof selected !== "string") return;
      const imported: DubbingImportResult = await invoke("freecut_import_dubbing_srt", { filePath: selected });
      updateDubbingSession((session: DubbingSession) => ({
        ...session,
        ingestMode: "srt",
        importedSrtPath: imported.importedSrtPath,
        segments: imported.segments,
        speakers: mergeDubbingSpeakers(session.speakers, imported.speakers),
        updatedAt: new Date().toISOString(),
      }));
      dubbingStatus = `Loaded ${imported.segments.length} subtitle segments`;
      await saveProject();
    } catch (e: any) {
      dubbingError = e?.toString?.() ?? "Failed to import SRT";
    }
  }

    async function generateDubbingAssets() {
    if (!currentProject) return;
    try {
      dubbingBusy = true;
      dubbingError = null;
      ensureSourceMediaSelection();
      const sourceMedia = mediaLibrary.find((item) => item.id === dubbingSession.sourceMediaId) ?? null;
      const nextSession: DubbingSession = {
        ...dubbingSession,
        sourceMediaId: sourceMedia?.id ?? dubbingSession.sourceMediaId,
        sourceMediaPath: sourceMedia?.filePath ?? dubbingSession.sourceMediaPath,
      };
      const whisper: WhisperSettings = {
        model: whisperModel,
        language: whisperLanguage,
        task: "transcribe",
        diarize: diarizeEnabled || null,
        hfToken: diarizeEnabled && hfToken.trim() ? hfToken.trim() : null,
        minSpeakers: diarizeEnabled ? diarizeMinSpeakers : null,
        maxSpeakers: diarizeEnabled ? diarizeMaxSpeakers : null,
      };
      const generated: DubbingSession = await invoke("freecut_generate_dub_assets", {
        projectId: currentProject.id,
        session: nextSession,
        whisper,
      });
      patchCurrentProjectDubbing(generated);
      dubbingStatus = `Generated ${generated.segments.filter((segment) => segment.audioPath).length} dub clips`;
      await saveProject();
    } catch (e: any) {
      dubbingError = e?.toString?.() ?? "Failed to generate dubbing assets";
    } finally {
      dubbingBusy = false;
    }
  }

    function ensureNamedTrack(kind: "video" | "audio", desiredName: string): string {
    const existing = itemsStore.getState().tracks.find((track) => track.kind === kind && track.name === desiredName);
    if (existing) return existing.id;
    const trackId = createDefaultTrack(kind);
    itemsStore.getState().updateTrack(trackId, { name: desiredName });
    return trackId;
  }

    function placeDubbingOnTimeline() {
    if (!currentProject || dubbingSession.segments.length === 0) return;
    historyStore.getState().push();
    const width = currentProject.metadata.width;
    const height = currentProject.metadata.height;
    const audioTrackId = ensureNamedTrack("audio", "Dub Voice");
    const textTrackId = ensureNamedTrack("video", "Dub Captions");
    const items: TimelineItem[] = [];

    for (const [index, segment] of dubbingSession.segments.entries()) {
      const from = Math.max(0, Math.round(segment.startSecs * fps));
      const duration = Math.max(1, Math.round((segment.endSecs - segment.startSecs) * fps));
      const speaker = dubbingSession.speakers.find((item) => item.id === segment.speakerId);
      const clipText = segment.outputText ?? segment.text;

      if (segment.audioPath) {
        items.push({
          id: crypto.randomUUID(),
          trackId: audioTrackId,
          from,
          durationInFrames: duration,
          label: `${speaker?.label ?? "Dub"} ${index + 1}`,
          type: "audio",
          src: segment.audioPath,
          sourceStart: 0,
          sourceEnd: duration,
          sourceDuration: duration,
          sourceFps: fps,
          speed: 1,
          volume: 1,
        });
      }

      items.push({
        id: crypto.randomUUID(),
        trackId: textTrackId,
        from,
        durationInFrames: duration,
        label: `Caption ${index + 1}`,
        type: "text",
        text: clipText,
        fontSize: 42,
        fontFamily: "Inter",
        color: "#ffffff",
        textAlign: "center",
        fillColor: "#ffffff",
        transform: { x: width / 2, y: height - 140, rotation: 0, opacity: 1 },
      });
    }

    itemsStore.getState().addItems(items);
    dubbingStatus = `Placed ${items.length} dubbing clips on the timeline`;
  }

    async function dubStudioSplit() {
    const sourceMedia = mediaLibrary.find((m) => m.mediaType === 'video');
    if (!sourceMedia) { dubStudioError = 'No video in media library'; return; }
    try {
      dubStudioBusy = true;
      dubStudioError = null;
      dubStudioStatus = 'Splitting video & transcribing...';
      const resp = await fetch('http://localhost:8080/api/freecut/dub/split', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          input_path: sourceMedia.filePath,
          segment_duration_secs: dubStudioSegDuration,
          target_lang: dubStudioTargetLang,
        }),
      });
      const data = await resp.json();
      if (data.success) {
        dubStudioJob = data.data;
        dubStudioStatus = `Split into ${data.data.total_parts} parts — ready for editing`;
      } else {
        dubStudioError = data.message ?? 'Split failed';
      }
    } catch (e: any) {
      const msg = e?.message ?? String(e);
      dubStudioError = msg.includes('Load failed') || msg.includes('fetch')
        ? 'Cannot reach NDE-OS server (localhost:8080). Is it running?'
        : msg;
    } finally {
      dubStudioBusy = false;
    }
  }

  async function dubStudioDubPart(partIndex: number) {
    if (!dubStudioJob) return;
    try {
      dubStudioDubbingPart = partIndex;
      dubStudioError = null;
      dubStudioStatus = `Dubbing part ${partIndex + 1}...`;
      const body: any = { job_id: dubStudioJob.id, part_index: partIndex, export_mode: 'MergeAll' };
      const resp = await fetch('http://localhost:8080/api/freecut/dub/part', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      const data = await resp.json();
      if (data.success) {
        dubStudioJob.parts[partIndex].status = 'Dubbed';
        dubStudioJob.parts[partIndex].dubbed_path = data.data.dubbed_path;
        dubStudioJob = { ...dubStudioJob };
        dubStudioStatus = `Part ${partIndex + 1} dubbed ✅`;
      } else {
        dubStudioJob.parts[partIndex].status = 'Error';
        dubStudioJob.parts[partIndex].error = data.message;
        dubStudioJob = { ...dubStudioJob };
        dubStudioError = data.message;
      }
    } catch (e: any) {
      dubStudioError = e?.message ?? 'Dub failed';
    } finally {
      dubStudioDubbingPart = null;
    }
  }

  async function dubStudioDubAll() {
    if (!dubStudioJob) return;
    try {
      dubStudioDubAllBusy = true;
      dubStudioError = null;
      const undubbed = dubStudioJob.parts.filter(p => p.status !== 'Dubbed').length;
      dubStudioStatus = `Dubbing all ${undubbed} remaining parts in queue...`;
      const resp = await fetch('http://localhost:8080/api/freecut/dub/all', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ job_id: dubStudioJob.id, export_mode: 'MergeAll' }),
      });
      const data = await resp.json();
      if (data.success) {
        dubStudioJob = data.data.job;
        const results = data.data.results ?? [];
        const dubbed = results.filter((r: any) => r.status === 'dubbed' || r.status === 'already_dubbed').length;
        const errors = results.filter((r: any) => r.status === 'error').length;
        dubStudioStatus = `Queue complete: ${dubbed} dubbed${errors ? `, ${errors} errors` : ''}`;
      } else {
        dubStudioError = data.message ?? 'Dub all failed';
      }
    } catch (e: any) {
      dubStudioError = e?.message ?? 'Dub all failed';
    } finally {
      dubStudioDubAllBusy = false;
    }
  }

  async function dubStudioMerge() {
    if (!dubStudioJob) return;
    const sourceMedia = mediaLibrary.find((m) => m.mediaType === 'video');
    try {
      dubStudioMerging = true;
      dubStudioError = null;
      dubStudioStatus = 'Merging all dubbed parts → final video...';
      const resp = await fetch('http://localhost:8080/api/freecut/dub/merge', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          job_id: dubStudioJob.id,
          output_path: sourceMedia?.filePath?.replace(/\.[^.]+$/, '_dubbed_full.mp4'),
        }),
      });
      const data = await resp.json();
      if (data.success) {
        dubStudioStatus = `✅ Merged! Output: ${data.data.output_path}`;
      } else {
        dubStudioError = data.message ?? 'Merge failed';
      }
    } catch (e: any) {
      dubStudioError = e?.message ?? 'Merge failed';
    } finally {
      dubStudioMerging = false;
    }
  }

  async function dubStudioReadSrt(partIndex: number, srtType: 'translated' | 'original' = 'translated') {
    if (!dubStudioJob) return;
    try {
      const resp = await fetch('http://localhost:8080/api/freecut/dub/srt/read', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ job_id: dubStudioJob.id, part_index: partIndex, srt_type: srtType }),
      });
      const data = await resp.json();
      if (data.success) {
        dubStudioEditingSrt = { partIndex, content: data.data.content ?? '' };
      } else {
        dubStudioEditingSrt = { partIndex, content: `Error: ${data.message}` };
      }
    } catch (e) {
      dubStudioEditingSrt = { partIndex, content: 'Error loading SRT — is the server running?' };
    }
  }

  async function dubStudioSaveSrt() {
    if (!dubStudioJob || !dubStudioEditingSrt) return;
    try {
      dubStudioSavingSrt = true;
      const resp = await fetch('http://localhost:8080/api/freecut/dub/srt/save', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          job_id: dubStudioJob.id,
          part_index: dubStudioEditingSrt.partIndex,
          content: dubStudioEditingSrt.content,
        }),
      });
      const data = await resp.json();
      if (data.success) {
        dubStudioStatus = `SRT saved for part ${dubStudioEditingSrt.partIndex + 1}`;
      } else {
        dubStudioError = data.message ?? 'Failed to save SRT';
      }
    } catch (e: any) {
      dubStudioError = e?.message ?? 'Failed to save SRT';
    } finally {
      dubStudioSavingSrt = false;
    }
  }

  function dubStudioFormatTime(secs: number): string {
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${m}:${s.toString().padStart(2, '0')}`;
  }

  let unlisten: Array<() => void> = [];
  onMount(async () => {
    detectDubbingTools().catch(() => {});
    // Auto-load global HF token from persisted settings
    try {
      const globalSettings = await invoke<{ hfToken?: string | null }>("get_global_settings");
      if (globalSettings.hfToken && !hfToken) {
        hfToken = globalSettings.hfToken;
      }
    } catch {}
    unlisten.push(
      await listen("freecut://dubbing-progress", (event: any) => {
        dubbingProgress = event.payload;
        dubbingStatus = event.payload?.message ?? null;
      }),
      await listen("freecut://dubbing-ready", (event: any) => {
        dubbingProgress = null;
        dubbingStatus = `Generated ${event.payload?.session?.segments?.length ?? 0} dub segments`;
      })
    );

    const onGlobalSettingsUpdated = () => { reloadGlobalHfToken(); };
    globalThis.addEventListener("nde:global-settings-updated", onGlobalSettingsUpdated);
    unlisten.push(() => globalThis.removeEventListener("nde:global-settings-updated", onGlobalSettingsUpdated));
  });
  onDestroy(() => {
    unlisten.forEach((fn) => fn());
  });
</script>

              <div class="p-3.5 space-y-3">
                <!-- ─── Tool Status & Setup ─────────────────────────────── -->
                <div class="rounded-2xl border border-white/8 bg-linear-to-br from-white/[0.05] to-white/[0.015] p-3 space-y-3">
                  <div class="flex items-start justify-between gap-3">
                    <div class="min-w-0">
                      <p class="text-xs font-semibold text-white/85">🎬 Movie Dubbing</p>
                      <p class="text-[10px] text-white/40 mt-1">Split → Edit SRT → Dub each part → Merge into final video.</p>
                    </div>
                    <button class="shrink-0 rounded-lg bg-white/5 px-2.5 py-1.5 text-[10px] text-white/70 transition-colors hover:bg-white/10" onclick={detectDubbingTools}>
                      Refresh
                    </button>
                  </div>

                  <div class="grid grid-cols-3 gap-2">
                    <div class="rounded-xl border border-white/6 bg-black/20 px-2.5 py-2">
                      <p class="text-[9px] uppercase tracking-wider text-white/30">Whisper</p>
                      <p class="text-[11px] mt-1 {dubbingTools?.whisperAvailable ? 'text-emerald-300' : 'text-white/45'}">
                        {dubbingTools?.whisperAvailable ? "Ready" : "Missing"}
                      </p>
                    </div>
                    <div class="rounded-xl border border-white/6 bg-black/20 px-2.5 py-2">
                      <p class="text-[9px] uppercase tracking-wider text-white/30">Edge TTS</p>
                      <p class="text-[11px] mt-1 {dubbingTools?.edgeTtsAvailable ? 'text-emerald-300' : 'text-white/45'}">
                        {dubbingTools?.edgeTtsAvailable ? "Ready" : "Missing"}
                      </p>
                    </div>
                    <button
                      type="button"
                      class="rounded-xl border border-white/6 bg-black/20 px-2.5 py-2 text-left transition-colors {dubbingTools?.whisperxAvailable ? '' : 'hover:border-violet-400/40 hover:bg-violet-500/10 cursor-pointer'}"
                      onclick={() => { if (!dubbingTools?.whisperxAvailable) openServiceHubForWhisperX(); }}
                      disabled={dubbingTools?.whisperxAvailable}
                      title={dubbingTools?.whisperxAvailable ? "WhisperX ready" : "Install via Service Hub"}
                    >
                      <p class="text-[9px] uppercase tracking-wider text-white/30">WhisperX</p>
                      <p class="text-[11px] mt-1 {dubbingTools?.whisperxAvailable ? 'text-emerald-300' : 'text-amber-300/70'}">
                        {dubbingTools?.whisperxAvailable ? "Ready" : "Install →"}
                      </p>
                    </button>
                    <div class="rounded-xl border border-white/6 bg-black/20 px-2.5 py-2">
                      <p class="text-[9px] uppercase tracking-wider text-white/30">NDE LLM</p>
                      <p class="text-[11px] mt-1 {dubbingTools?.ndeLlmAvailable ? 'text-emerald-300' : 'text-white/45'}">
                        {dubbingTools?.ndeLlmAvailable ? (dubbingTools?.ndeActiveModel ?? "Ready") : "Unavailable"}
                      </p>
                    </div>
                    <div class="rounded-xl border border-white/6 bg-black/20 px-2.5 py-2">
                      <p class="text-[9px] uppercase tracking-wider text-white/30">RVC Lane</p>
                      <p class="text-[11px] mt-1 {dubbingTools?.rvcAvailable ? 'text-emerald-300' : 'text-white/45'}">
                        {dubbingTools?.rvcAvailable ? "Python Ready" : "Needs Python"}
                      </p>
                    </div>
                  </div>

                  {#if needsDubbingSetup}
                    <div class="rounded-2xl border border-amber-400/20 bg-amber-400/8 p-3 space-y-3">
                      <div class="flex items-start justify-between gap-3">
                        <div class="min-w-0">
                          <p class="text-xs font-semibold text-amber-100">Setup Required</p>
                          <p class="text-[10px] text-amber-100/70 mt-1">Voice Runtime (Edge TTS + Whisper) must be installed via the global Service Hub.</p>
                        </div>
                      </div>
                      <button
                        class="w-full rounded-lg bg-violet-600 px-3 py-2.5 text-[11px] font-medium text-white transition-colors hover:bg-violet-500 flex items-center justify-center gap-2"
                        onclick={openServiceHubForVoice}
                      >
                        🔧 Open Service Hub — Install Voice Runtime
                      </button>
                      <p class="text-[9px] text-amber-100/50 text-center">After installing, press Refresh above.</p>
                    </div>
                  {/if}
                </div>

                <!-- ─── Split & Dub Workstation ─────────────────────────── -->
                <div class="rounded-2xl border border-white/8 bg-white/[0.02] p-3 space-y-3">
                  <div class="flex items-center justify-between">
                    <p class="text-xs font-semibold text-white/80">✂️ Split & Dub Workstation</p>
                    {#if dubStudioJob}
                      <button
                        class="rounded-lg bg-white/5 px-2 py-1 text-[9px] text-white/50 hover:bg-white/10 transition-colors"
                        onclick={() => { dubStudioJob = null; dubStudioEditingSrt = null; dubStudioError = null; dubStudioStatus = null; }}
                      >New Job</button>
                    {/if}
                  </div>

                  {#if !dubStudioJob}
                    <!-- Source & Split config -->
                    <div class="grid grid-cols-2 gap-2">
                      <label class="block">
                        <span class="text-[9px] uppercase tracking-wider text-white/35">Source Video</span>
                        <select
                          class="mt-1 w-full rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white"
                          value={dubbingSession.sourceMediaId ?? ""}
                          onchange={(e) => {
                            const media = mediaLibrary.find((item) => item.id === e.currentTarget.value) ?? null;
                            patchCurrentProjectDubbing({ sourceMediaId: media?.id ?? null, sourceMediaPath: media?.filePath ?? null });
                          }}
                        >
                          <option value="">Select video</option>
                          {#each mediaLibrary.filter((item) => item.mediaType === "video") as media (media.id)}
                            <option value={media.id}>{media.fileName}</option>
                          {/each}
                        </select>
                      </label>
                      <label class="block">
                        <span class="text-[9px] uppercase tracking-wider text-white/35">Target Language</span>
                        <select
                          class="mt-1 w-full rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white"
                          bind:value={dubStudioTargetLang}
                        >
                          <option value="km">🇰🇭 Khmer</option>
                          <option value="en">🇺🇸 English</option>
                          <option value="zh">🇨🇳 Chinese</option>
                        </select>
                      </label>
                    </div>
                    <div class="space-y-2">
                      <label class="text-[10px] text-white/50 block">Segment Duration (seconds)</label>
                      <div class="flex gap-2">
                        <input type="number" min="10" max="600" step="10" bind:value={dubStudioSegDuration}
                          class="flex-1 rounded-lg border border-white/10 bg-black/30 px-3 py-2 text-xs text-white/80 outline-none focus:border-violet-500/40" />
                        <button
                          class="rounded-lg bg-violet-600 px-4 py-2 text-[11px] font-medium text-white transition-colors hover:bg-violet-500 disabled:opacity-40"
                          onclick={dubStudioSplit}
                          disabled={dubStudioBusy || mediaLibrary.filter(m => m.mediaType === 'video').length === 0}
                        >
                          {dubStudioBusy ? '⏳ Splitting...' : '✂️ Split & Transcribe'}
                        </button>
                      </div>
                      {#if mediaLibrary.filter(m => m.mediaType === 'video').length === 0}
                        <p class="text-[10px] text-amber-300/70">Import a video first in the Media tab.</p>
                      {/if}
                    </div>
                  {:else}
                    <!-- Job overview stats -->
                    <div class="grid grid-cols-3 gap-2">
                      <div class="rounded-xl border border-white/6 bg-black/20 px-2.5 py-2">
                        <p class="text-[9px] uppercase tracking-wider text-white/30">Parts</p>
                        <p class="mt-1 text-sm font-semibold text-white/85">{dubStudioJob.total_parts}</p>
                      </div>
                      <div class="rounded-xl border border-white/6 bg-black/20 px-2.5 py-2">
                        <p class="text-[9px] uppercase tracking-wider text-white/30">Dubbed</p>
                        <p class="mt-1 text-sm font-semibold text-emerald-300">{dubStudioJob.parts.filter(p => p.status === 'Dubbed').length} / {dubStudioJob.total_parts}</p>
                      </div>
                      <div class="rounded-xl border border-white/6 bg-black/20 px-2.5 py-2">
                        <p class="text-[9px] uppercase tracking-wider text-white/30">Duration</p>
                        <p class="mt-1 text-sm font-semibold text-white/85">{dubStudioFormatTime(dubStudioJob.total_duration_secs)}</p>
                      </div>
                    </div>

                    <!-- Progress bar -->
                    {@const dubbedCount = dubStudioJob.parts.filter(p => p.status === 'Dubbed').length}
                    <div class="w-full h-1.5 rounded-full bg-white/5 overflow-hidden">
                      <div class="h-full rounded-full bg-linear-to-r from-violet-500 to-emerald-500 transition-all duration-500" style:width="{(dubbedCount / dubStudioJob.total_parts) * 100}%"></div>
                    </div>

                    <!-- Parts list -->
                    <div class="space-y-1.5 max-h-[320px] overflow-y-auto">
                      {#each dubStudioJob.parts as part (part.index)}
                        <div class="rounded-xl border {part.status === 'Dubbed' ? 'border-emerald-500/20 bg-emerald-500/5' : part.status === 'Error' ? 'border-red-500/20 bg-red-500/5' : 'border-white/6 bg-black/20'} p-2.5 space-y-1.5">
                          <div class="flex items-center justify-between">
                            <div class="flex items-center gap-2">
                              <span class="text-[10px] font-bold text-white/70">Part {part.index + 1}</span>
                              <span class="text-[9px] text-white/30">{dubStudioFormatTime(part.start_secs)} → {dubStudioFormatTime(part.end_secs)}</span>
                              <span class="text-[9px] px-1.5 py-0.5 rounded-full {part.status === 'Dubbed' ? 'bg-emerald-500/20 text-emerald-300' : part.status === 'Error' ? 'bg-red-500/20 text-red-300' : part.status === 'SrtReady' ? 'bg-blue-500/20 text-blue-300' : 'bg-white/10 text-white/40'}">
                                {part.status}
                              </span>
                            </div>
                            <div class="flex gap-1">
                              {#if part.status === 'SrtReady' || part.status === 'Dubbed'}
                                <button class="rounded-md bg-white/5 px-2 py-1 text-[9px] text-white/60 hover:bg-white/10 hover:text-white/80 transition-colors"
                                  onclick={() => dubStudioReadSrt(part.index)}>📝 Edit SRT</button>
                              {/if}
                              {#if part.status !== 'Dubbed'}
                                <button class="rounded-md bg-violet-600/80 px-2 py-1 text-[9px] text-white hover:bg-violet-500 transition-colors disabled:opacity-40"
                                  onclick={() => dubStudioDubPart(part.index)}
                                  disabled={dubStudioDubbingPart !== null || dubStudioDubAllBusy}>
                                  {dubStudioDubbingPart === part.index ? '⏳ Dubbing...' : '🎙 Dub'}
                                </button>
                              {:else}
                                <span class="text-[9px] text-emerald-400 px-2 py-1">✅ Done</span>
                              {/if}
                            </div>
                          </div>
                          {#if part.error}
                            <p class="text-[9px] text-red-300/70 truncate">{part.error}</p>
                          {/if}
                        </div>
                      {/each}
                    </div>

                    <!-- SRT Editor -->
                    {#if dubStudioEditingSrt}
                      <div class="rounded-xl border border-violet-500/20 bg-violet-500/5 p-2.5 space-y-1.5">
                        <div class="flex items-center justify-between">
                          <p class="text-[10px] font-semibold text-violet-300">📝 Editing SRT — Part {dubStudioEditingSrt.partIndex + 1}</p>
                          <div class="flex gap-1">
                            <button class="rounded-md bg-white/5 px-2 py-1 text-[9px] text-white/60 hover:bg-white/10 transition-colors"
                              onclick={() => dubStudioReadSrt(dubStudioEditingSrt!.partIndex, 'original')}>View Original</button>
                            <button class="text-[9px] text-white/40 hover:text-white/70 px-2 py-1" onclick={() => dubStudioEditingSrt = null}>✕</button>
                          </div>
                        </div>
                        <textarea
                          class="w-full h-40 rounded-lg border border-white/10 bg-black/40 px-3 py-2 text-[10px] text-white/80 font-mono outline-none focus:border-violet-500/40 resize-y"
                          bind:value={dubStudioEditingSrt.content}
                        />
                        <div class="flex gap-2">
                          <button
                            class="flex-1 rounded-lg bg-violet-600 px-3 py-2 text-[10px] font-medium text-white hover:bg-violet-500 transition-colors disabled:opacity-40"
                            onclick={dubStudioSaveSrt}
                            disabled={dubStudioSavingSrt}
                          >
                            {dubStudioSavingSrt ? '⏳ Saving...' : '💾 Save SRT'}
                          </button>
                          <button
                            class="rounded-lg bg-white/5 px-3 py-2 text-[10px] text-white/60 hover:bg-white/10 transition-colors disabled:opacity-40"
                            onclick={() => { dubStudioSaveSrt().then(() => dubStudioDubPart(dubStudioEditingSrt!.partIndex)); }}
                            disabled={dubStudioSavingSrt || dubStudioDubbingPart !== null}
                          >
                            Save & Dub
                          </button>
                        </div>
                        <p class="text-[9px] text-white/30">Edit the translated subtitles, then Save & Dub to re-synthesize.</p>
                      </div>
                    {/if}

                    <!-- Action buttons -->
                    <div class="flex gap-2">
                      <button
                        class="flex-1 rounded-lg bg-violet-600 px-3 py-2.5 text-[11px] font-medium text-white transition-colors hover:bg-violet-500 disabled:opacity-40 flex items-center justify-center gap-2"
                        onclick={dubStudioDubAll}
                        disabled={dubStudioDubAllBusy || dubStudioDubbingPart !== null || dubStudioJob.parts.every(p => p.status === 'Dubbed')}
                      >
                        {#if dubStudioDubAllBusy}
                          ⏳ Dubbing Queue...
                        {:else}
                          🎙 Dub All ({dubStudioJob.parts.filter(p => p.status !== 'Dubbed').length} remaining)
                        {/if}
                      </button>
                    </div>
                    <div class="flex gap-2">
                      <button
                        class="flex-1 rounded-lg bg-emerald-600 px-3 py-2.5 text-[11px] font-medium text-white transition-colors hover:bg-emerald-500 disabled:opacity-40 flex items-center justify-center gap-2"
                        onclick={dubStudioMerge}
                        disabled={dubStudioMerging || dubStudioJob.parts.some(p => p.status !== 'Dubbed')}
                      >
                        {dubStudioMerging ? '⏳ Merging...' : '🔗 Merge All → Final Video'}
                      </button>
                    </div>
                    {#if dubStudioJob.parts.some(p => p.status !== 'Dubbed')}
                      <p class="text-[9px] text-amber-300/60 text-center">All parts must be dubbed before merging.</p>
                    {/if}
                  {/if}

                  {#if dubStudioError}
                    <div class="rounded-lg border border-red-500/20 bg-red-500/8 px-3 py-2">
                      <p class="text-[10px] text-red-300">{dubStudioError}</p>
                    </div>
                  {/if}
                  {#if dubStudioStatus}
                    <p class="text-[10px] text-white/40 text-center">{dubStudioStatus}</p>
                  {/if}
                </div>

                <!-- ─── Quick Dub (single file, no split) ──────────────── -->
                <details class="rounded-2xl border border-white/8 bg-white/[0.02] overflow-hidden group">
                  <summary class="p-3 cursor-pointer text-xs font-semibold text-white/60 hover:text-white/80 transition-colors flex items-center gap-2">
                    <ChevronRight class="w-3.5 h-3.5 transition-transform group-open:rotate-90" />
                    Quick Dub (single file, no splitting)
                  </summary>
                  <div class="p-3 pt-0 space-y-3 border-t border-white/5">
                    <div class="grid grid-cols-2 gap-2 mt-3">
                      <label class="block">
                        <span class="text-[9px] uppercase tracking-wider text-white/35">Source Media</span>
                        <select
                          class="mt-1 w-full rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white"
                          value={dubbingSession.sourceMediaId ?? ""}
                          onchange={(e) => {
                            const media = mediaLibrary.find((item) => item.id === e.currentTarget.value) ?? null;
                            patchCurrentProjectDubbing({ sourceMediaId: media?.id ?? null, sourceMediaPath: media?.filePath ?? null });
                          }}
                        >
                          <option value="">Select media</option>
                          {#each mediaLibrary.filter((item) => item.mediaType !== "image") as media (media.id)}
                            <option value={media.id}>{media.fileName}</option>
                          {/each}
                        </select>
                      </label>
                      <label class="block">
                        <span class="text-[9px] uppercase tracking-wider text-white/35">Ingest</span>
                        <select
                          class="mt-1 w-full rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white"
                          value={dubbingSession.ingestMode}
                          onchange={(e) => patchCurrentProjectDubbing({ ingestMode: e.currentTarget.value as "srt" | "whisper" })}
                        >
                          <option value="srt">Local SRT</option>
                          <option value="whisper">Whisper Local</option>
                        </select>
                      </label>
                    </div>

                    {#if dubbingSession.ingestMode === "whisper"}
                      <div class="grid grid-cols-2 gap-2">
                        <label class="block">
                          <span class="text-[9px] uppercase tracking-wider text-white/35">Whisper Model</span>
                          <input type="text" class="mt-1 w-full rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white" bind:value={whisperModel} />
                        </label>
                        <label class="block">
                          <span class="text-[9px] uppercase tracking-wider text-white/35">Whisper Language</span>
                          <input type="text" class="mt-1 w-full rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white" bind:value={whisperLanguage} />
                        </label>
                      </div>

                      <!-- Speaker Diarization -->
                      <div class="rounded-xl border border-white/6 bg-black/20 p-2.5 space-y-2">
                        <div class="flex items-center justify-between">
                          <label class="flex items-center gap-2 text-[10px] text-white/70 cursor-pointer">
                            <input type="checkbox" bind:checked={diarizeEnabled} />
                            🎯 Auto-detect speakers
                          </label>
                          {#if !dubbingTools?.whisperxAvailable}
                            <button
                              class="rounded-md bg-violet-600/60 px-2 py-1 text-[9px] text-white/80 hover:bg-violet-500/60 transition-colors disabled:opacity-40"
                              onclick={() => installDubbingRuntime('diarization')}
                              disabled={runtimeInstallBusy}
                            >
                              {runtimeInstallBusy ? '⏳ Installing...' : '📦 Install WhisperX'}
                            </button>
                          {/if}
                        </div>
                        {#if diarizeEnabled}
                          <p class="text-[9px] text-white/35">Uses WhisperX + pyannote.audio to detect and label each speaker automatically.</p>
                          <div class="space-y-2">
                            <div class="rounded-lg border border-white/10 bg-white/5 px-2.5 py-2 flex items-center justify-between gap-2">
                              <div class="min-w-0">
                                <p class="text-[9px] uppercase tracking-wider text-white/35">HuggingFace Token</p>
                                <p class="text-[11px] mt-0.5 {hfToken ? 'text-emerald-300' : 'text-amber-300/70'}">
                                  {hfToken ? "✓ Set in NDE Settings" : "Not set"}
                                </p>
                              </div>
                              <button
                                type="button"
                                class="shrink-0 rounded-md bg-violet-600/70 px-2.5 py-1.5 text-[10px] text-white hover:bg-violet-500/70 transition-colors"
                                onclick={openSettingsApiKeys}
                              >
                                ⚙️ Open NDE Settings
                              </button>
                            </div>
                            <p class="text-[8px] text-white/25">Manage the token in ⚙️ Settings → 🔑 API Keys & Tokens. It's encrypted in your OS keychain and shared across projects.</p>
                            <div class="grid grid-cols-2 gap-2">
                              <label class="block">
                                <span class="text-[9px] uppercase tracking-wider text-white/35">Min Speakers</span>
                                <input type="number" min="1" max="20" class="mt-1 w-full rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white" value={diarizeMinSpeakers ?? ''} onchange={(e) => diarizeMinSpeakers = e.currentTarget.value ? parseInt(e.currentTarget.value) : null} placeholder="Auto" />
                              </label>
                              <label class="block">
                                <span class="text-[9px] uppercase tracking-wider text-white/35">Max Speakers</span>
                                <input type="number" min="1" max="20" class="mt-1 w-full rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white" value={diarizeMaxSpeakers ?? ''} onchange={(e) => diarizeMaxSpeakers = e.currentTarget.value ? parseInt(e.currentTarget.value) : null} placeholder="Auto" />
                              </label>
                            </div>
                          </div>
                          {#if !dubbingTools?.whisperxAvailable}
                            <div class="rounded-lg border border-amber-400/20 bg-amber-400/8 px-2.5 py-2">
                              <p class="text-[9px] text-amber-200/80">⚠️ WhisperX not installed. Click "Install WhisperX" above or install via Service Hub.</p>
                            </div>
                          {/if}
                        {/if}
                      </div>
                    {/if}

                    <div class="grid grid-cols-3 gap-2">
                      <button class="rounded-lg bg-white/5 px-3 py-2 text-[11px] text-white/75 transition-colors hover:bg-white/10" onclick={importDubbingSrt}>
                        Import SRT
                      </button>
                      <button class="rounded-lg bg-violet-600 px-3 py-2 text-[11px] text-white transition-colors hover:bg-violet-500 disabled:opacity-50" onclick={generateDubbingAssets} disabled={dubbingBusy}>
                        {dubbingBusy ? "Generating..." : "Generate"}
                      </button>
                      <button class="rounded-lg bg-cyan-500/15 px-3 py-2 text-[11px] text-cyan-200 transition-colors hover:bg-cyan-500/25 disabled:opacity-50" onclick={placeDubbingOnTimeline} disabled={!dubbingSession.segments.some((segment) => segment.audioPath)}>
                        Timeline
                      </button>
                    </div>

                    <div class="grid grid-cols-3 gap-2">
                      <div class="rounded-xl border border-white/6 bg-black/20 px-2.5 py-2">
                        <p class="text-[9px] uppercase tracking-wider text-white/30">Segments</p>
                        <p class="mt-1 text-sm font-semibold text-white/85">{dubbingSession.segments.length}</p>
                      </div>
                      <div class="rounded-xl border border-white/6 bg-black/20 px-2.5 py-2">
                        <p class="text-[9px] uppercase tracking-wider text-white/30">Ready</p>
                        <p class="mt-1 text-sm font-semibold text-emerald-300">{readyDubSegments}</p>
                      </div>
                      <div class="rounded-xl border border-white/6 bg-black/20 px-2.5 py-2">
                        <p class="text-[9px] uppercase tracking-wider text-white/30">Speakers</p>
                        <p class="mt-1 text-sm font-semibold text-white/85">{activeSpeakerCount}</p>
                      </div>
                    </div>

                    {#if dubbingProgress}
                      <div class="rounded-lg border border-violet-500/20 bg-violet-500/5 px-3 py-2">
                        <p class="text-[11px] text-violet-200">{dubbingProgress.message}</p>
                        <p class="text-[10px] text-violet-200/60 mt-1">{dubbingProgress.phase} · {dubbingProgress.current}/{dubbingProgress.total}</p>
                      </div>
                    {/if}
                    {#if dubbingStatus}
                      <p class="text-[10px] text-emerald-300/80">{dubbingStatus}</p>
                    {/if}
                    {#if dubbingError}
                      <p class="text-[10px] text-red-300">{dubbingError}</p>
                    {/if}
                  </div>
                </details>

                <!-- ─── Speakers ───────────────────────────────────────── -->
                <details class="rounded-2xl border border-white/8 bg-white/[0.02] overflow-hidden group">
                  <summary class="p-3 cursor-pointer text-xs font-semibold text-white/60 hover:text-white/80 transition-colors flex items-center justify-between">
                    <span class="flex items-center gap-2">
                      <ChevronRight class="w-3.5 h-3.5 transition-transform group-open:rotate-90" />
                      Speakers ({activeSpeakerCount})
                    </span>
                    <button class="rounded-lg bg-white/5 px-2.5 py-1.5 text-[10px] text-white/70 transition-colors hover:bg-white/10" onclick={(e) => { e.preventDefault(); addDubbingSpeaker(); }}>
                      + Add
                    </button>
                  </summary>
                  <div class="p-3 pt-0 space-y-2 border-t border-white/5 mt-0">
                    {#each dubbingSession.speakers as speaker (speaker.id)}
                      <div class="rounded-xl border border-white/6 bg-black/20 p-2.5 space-y-2 mt-2">
                        <div class="flex items-center gap-2">
                          <input
                            type="text"
                            class="flex-1 rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white"
                            value={speaker.label}
                            onchange={(e) => updateDubbingSpeaker(speaker.id, { label: e.currentTarget.value })}
                          />
                          <button class="rounded-lg bg-red-500/10 px-2 py-1.5 text-[10px] text-red-200 transition-colors hover:bg-red-500/20" onclick={() => removeDubbingSpeaker(speaker.id)}>
                            ✕
                          </button>
                        </div>
                        <div class="grid grid-cols-2 gap-2">
                          <input type="text" class="rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white" value={speaker.voice} placeholder="Voice" onchange={(e) => updateDubbingSpeaker(speaker.id, { voice: e.currentTarget.value })} />
                          <input type="text" class="rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white" value={speaker.rate ?? ""} placeholder="Rate" onchange={(e) => updateDubbingSpeaker(speaker.id, { rate: e.currentTarget.value })} />
                          <input type="text" class="rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white" value={speaker.pitch ?? ""} placeholder="Pitch" onchange={(e) => updateDubbingSpeaker(speaker.id, { pitch: e.currentTarget.value })} />
                          <input type="text" class="rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white" value={speaker.volume ?? ""} placeholder="Volume" onchange={(e) => updateDubbingSpeaker(speaker.id, { volume: e.currentTarget.value })} />
                        </div>
                        <label class="flex items-center gap-2 text-[10px] text-white/55">
                          <input
                            type="checkbox"
                            checked={speaker.rvc?.enabled ?? false}
                            onchange={(e) => updateDubbingSpeaker(speaker.id, { rvc: { enabled: speaker.rvc?.enabled ?? false, ...(speaker.rvc ?? {}), enabled: e.currentTarget.checked } })}
                          />
                          Enable RVC post-process
                        </label>
                        {#if speaker.rvc?.enabled}
                          <div class="grid grid-cols-1 gap-2">
                            <input type="text" class="rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white" value={speaker.rvc?.cliPath ?? ""} placeholder="RVC CLI path" onchange={(e) => updateDubbingSpeaker(speaker.id, { rvc: { enabled: speaker.rvc?.enabled ?? false, ...(speaker.rvc ?? {}), cliPath: e.currentTarget.value } })} />
                            <input type="text" class="rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white" value={speaker.rvc?.modelPath ?? ""} placeholder="RVC model (.pth)" onchange={(e) => updateDubbingSpeaker(speaker.id, { rvc: { enabled: speaker.rvc?.enabled ?? false, ...(speaker.rvc ?? {}), modelPath: e.currentTarget.value } })} />
                            <input type="text" class="rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white" value={speaker.rvc?.indexPath ?? ""} placeholder="RVC index (.index)" onchange={(e) => updateDubbingSpeaker(speaker.id, { rvc: { enabled: speaker.rvc?.enabled ?? false, ...(speaker.rvc ?? {}), indexPath: e.currentTarget.value } })} />
                          </div>
                        {/if}
                      </div>
                    {/each}
                  </div>
                </details>

                <!-- ─── LLM Config ─────────────────────────────────────── -->
                <details class="rounded-2xl border border-white/8 bg-white/[0.02] overflow-hidden group">
                  <summary class="p-3 cursor-pointer text-xs font-semibold text-white/60 hover:text-white/80 transition-colors flex items-center gap-2">
                    <ChevronRight class="w-3.5 h-3.5 transition-transform group-open:rotate-90" />
                    NDE LLM Translation
                  </summary>
                  <div class="p-3 pt-0 space-y-3 border-t border-white/5">
                    <div class="flex items-center justify-between mt-3">
                      <p class="text-[10px] text-white/35">Use NDE LLM to translate or polish subtitles before TTS.</p>
                      <label class="flex items-center gap-2 text-[10px] text-white/55">
                        <input
                          type="checkbox"
                          checked={dubbingSession.llm?.enabled ?? false}
                          onchange={(e) => patchCurrentProjectDubbing({ llm: { ...(dubbingSession.llm ?? { enabled: false }), enabled: e.currentTarget.checked } })}
                        />
                        Enable
                      </label>
                    </div>
                    <div class="grid grid-cols-2 gap-2">
                      <div class="rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white/70">
                        {#if dubbingTools?.ndeLlmAvailable}
                          Active: {dubbingTools?.ndeActiveModel ?? "configured"}
                        {:else}
                          NDE LLM unavailable
                        {/if}
                      </div>
                      <select class="rounded-lg border border-white/10 bg-white/5 px-2 py-2 text-[11px] text-white" value={dubbingSession.llm?.mode ?? "translate"} onchange={(e) => patchCurrentProjectDubbing({ llm: { ...(dubbingSession.llm ?? { enabled: false }), mode: e.currentTarget.value } })}>
                        <option value="translate">Translate</option>
                        <option value="polish">Polish</option>
                      </select>
                    </div>
                  </div>
                </details>
              </div>
