#!/usr/bin/env python3
"""
Speaker diarization for NDE-OS FreeCut.

Uses whisperx (Whisper + VAD + forced alignment + pyannote speaker diarization)
to transcribe audio and assign speaker labels to each segment.

Usage:
    python diarize.py <audio_file> <output_json> [--hf_token TOKEN] [--model SIZE] [--language CODE] [--min_speakers N] [--max_speakers N] [--device DEVICE]

Output JSON format:
    {
      "language": "en",
      "speakers": ["SPEAKER_00", "SPEAKER_01"],
      "segments": [
        {"start": 0.5, "end": 2.3, "text": "Hello", "speaker": "SPEAKER_00"},
        ...
      ]
    }
"""

import argparse
import json
import sys
import os
import platform

# Inject NDE-OS standard ffmpeg location into PATH so whisperx/torchaudio can find it
def _patch_ffmpeg_path():
    home = os.path.expanduser("~")
    base_dir = os.environ.get("LOCALAPPDATA", home) if platform.system() == "Windows" else home
    suffix = "ai-launcher\\ffmpeg\\bin" if platform.system() == "Windows" else ".ai-launcher/.ffmpeg"
    ffmpeg_dir = os.path.join(base_dir, suffix)
    
    if os.path.isdir(ffmpeg_dir):
        os.environ["PATH"] = ffmpeg_dir + os.pathsep + os.environ.get("PATH", "")

_patch_ffmpeg_path()



def main():
    parser = argparse.ArgumentParser(description="Speaker diarization via whisperx + pyannote")
    parser.add_argument("audio_file", help="Path to input audio file (WAV/MP3/etc)")
    parser.add_argument("output_json", help="Path to write output JSON")
    parser.add_argument("--hf_token", default=None, help="HuggingFace auth token for pyannote models")
    parser.add_argument("--model", default="base", help="Whisper model size (tiny/base/small/medium/large-v2)")
    parser.add_argument("--language", default=None, help="Language code (en/zh/km etc). Auto-detect if omitted.")
    parser.add_argument("--min_speakers", type=int, default=None, help="Minimum expected speakers")
    parser.add_argument("--max_speakers", type=int, default=None, help="Maximum expected speakers")
    parser.add_argument("--device", default="cpu", help="Device (cpu/cuda)")
    args = parser.parse_args()

    if not os.path.isfile(args.audio_file):
        print(f"Error: audio file not found: {args.audio_file}", file=sys.stderr)
        sys.exit(1)

    try:
        import whisperx
    except ImportError:
        print("Error: whisperx is not installed. Run: pip install whisperx", file=sys.stderr)
        sys.exit(1)

    device = args.device
    compute_type = "int8" if device == "cpu" else "float16"

    # 1. Transcribe with whisperx
    print(f"[diarize] Loading whisperx model={args.model} device={device}...", file=sys.stderr)
    model = whisperx.load_model(args.model, device=device, compute_type=compute_type)
    audio = whisperx.load_audio(args.audio_file)

    print("[diarize] Transcribing...", file=sys.stderr)
    result = model.transcribe(audio, language=args.language)
    detected_language = result.get("language", "en")
    print(f"[diarize] Detected language: {detected_language}", file=sys.stderr)

    # 2. Align (for precise word timestamps)
    try:
        print("[diarize] Loading alignment model...", file=sys.stderr)
        model_a, metadata = whisperx.load_align_model(language_code=detected_language, device=device)
        result = whisperx.align(result["segments"], model_a, metadata, audio, device=device)
        print(f"[diarize] Aligned {len(result['segments'])} segments", file=sys.stderr)
    except Exception as e:
        print(f"[diarize] Alignment failed (non-fatal): {e}", file=sys.stderr)

    # 3. Diarize (assign speakers)
    speakers_found = set()
    if args.hf_token:
        try:
            print("[diarize] Running speaker diarization (pyannote)...", file=sys.stderr)
            import whisperx.diarize
            # whisperx.diarize.DiarizationPipeline uses `token` not `use_auth_token` in current versions
            diarize_model = whisperx.diarize.DiarizationPipeline(model_name="pyannote/speaker-diarization-3.1", token=args.hf_token, device=device)

            diarize_kwargs = {}
            if args.min_speakers is not None:
                diarize_kwargs["min_speakers"] = args.min_speakers
            if args.max_speakers is not None:
                diarize_kwargs["max_speakers"] = args.max_speakers

            diarize_segments = diarize_model(args.audio_file, **diarize_kwargs)
            result = whisperx.assign_word_speakers(diarize_segments, result)

            for seg in result.get("segments", []):
                spk = seg.get("speaker")
                if spk:
                    speakers_found.add(spk)

            print(f"[diarize] Found {len(speakers_found)} speakers: {sorted(speakers_found)}", file=sys.stderr)
        except Exception as e:
            print(f"[diarize] Diarization failed (non-fatal, will use single speaker): {e}", file=sys.stderr)
    else:
        print("[diarize] No HF token — skipping speaker diarization", file=sys.stderr)

    # 4. Build output
    output_segments = []
    for seg in result.get("segments", []):
        text = seg.get("text", "").strip()
        if not text:
            continue
        output_segments.append({
            "start": round(seg.get("start", 0.0), 3),
            "end": round(seg.get("end", 0.0), 3),
            "text": text,
            "speaker": seg.get("speaker", None),
        })

    output = {
        "language": detected_language,
        "speakers": sorted(speakers_found),
        "segments": output_segments,
    }

    # Write output
    os.makedirs(os.path.dirname(os.path.abspath(args.output_json)), exist_ok=True)
    with open(args.output_json, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"[diarize] Wrote {len(output_segments)} segments to {args.output_json}", file=sys.stderr)
    print(f"[diarize] Speakers: {sorted(speakers_found) if speakers_found else ['(single speaker)']}", file=sys.stderr)


if __name__ == "__main__":
    main()
