// ─── FreeCut Shared Types ────────────────────────────────────────────────────

export type ProjectSummary = { id: string; name: string; updatedAt: string };

export type Project = {
  id: string; name: string; description: string; duration: number;
  metadata: { width: number; height: number; fps: number; backgroundColor: string };
  timeline: any;
  createdAt: string; updatedAt: string; schemaVersion: number;
  dubbing?: DubbingSession | null;
};

export type MediaItem = {
  id: string; fileName: string; filePath: string; fileSize: number;
  mediaType: "video" | "audio" | "image";
  width?: number; height?: number; durationSecs?: number; fps?: number;
  codec?: string; thumbnailPath?: string;
};

export type DubbingRvcConfig = {
  enabled: boolean;
  pythonPath?: string | null;
  cliPath?: string | null;
  modelPath?: string | null;
  indexPath?: string | null;
  pitchShift?: number | null;
};

export type DubbingSpeaker = {
  id: string;
  label: string;
  voice: string;
  rate?: string | null;
  pitch?: string | null;
  volume?: string | null;
  rvc?: DubbingRvcConfig | null;
};

export type DubbingSegment = {
  id: string;
  startSecs: number;
  endSecs: number;
  text: string;
  outputText?: string | null;
  speakerId?: string | null;
  audioPath?: string | null;
  status?: string | null;
};

export type DubbingLlmConfig = {
  enabled: boolean;
  model?: string | null;
  mode?: string | null;
};

export type DubbingSession = {
  sourceMediaId?: string | null;
  sourceMediaPath?: string | null;
  sourceLanguage: string;
  targetLanguage: string;
  ingestMode: "srt" | "whisper";
  importedSrtPath?: string | null;
  outputDir?: string | null;
  notes?: string | null;
  segments: DubbingSegment[];
  speakers: DubbingSpeaker[];
  llm?: DubbingLlmConfig | null;
  updatedAt?: string | null;
  lastGeneratedAt?: string | null;
};

export type DubbingToolReport = {
  whisperAvailable: boolean;
  whisperxAvailable: boolean;
  edgeTtsAvailable: boolean;
  ndeLlmAvailable: boolean;
  pythonAvailable: boolean;
  rvcAvailable: boolean;
  edgeVoices: string[];
  ndeActiveModel?: string | null;
  details: string[];
};

export type DubbingImportResult = {
  importedSrtPath: string;
  segments: DubbingSegment[];
  speakers: DubbingSpeaker[];
};

export type DubbingProgress = {
  phase: string;
  current: number;
  total: number;
  message: string;
};

export type DubbingRuntimeInstallResult = {
  runtime: string;
  installedPackages: string[];
  workspacePath: string;
  binPath: string;
  message: string;
};

export type WhisperSettings = {
  model?: string | null;
  language?: string | null;
  task?: string | null;
  diarize?: boolean | null;
  hfToken?: string | null;
  minSpeakers?: number | null;
  maxSpeakers?: number | null;
};

export type DubStudioJob = {
  id: string;
  input_path: string;
  workspace: string;
  segment_duration_secs: number;
  total_duration_secs: number;
  total_parts: number;
  parts: DubStudioPart[];
  created_at: string;
};

export type DubStudioPart = {
  index: number;
  video_path: string;
  start_secs: number;
  end_secs: number;
  duration_secs: number;
  orig_srt_path: string;
  translated_srt_path: string;
  dubbed_path: string | null;
  status: 'Pending' | 'SrtReady' | 'Dubbed' | 'Error';
  error: string | null;
};

export type Marker = { id: string; frame: number; label: string; color: string };
