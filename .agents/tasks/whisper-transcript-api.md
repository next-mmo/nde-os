# Status
🟢 done by AI

# Feature
Transcript API using Whisper with GPU and Service Hub Model Download

# Purpose
Implement a backend API for transcribing audio files using the Whisper model with GPU acceleration. The feature allows users to dynamically download required Whisper models via the service hub and provides a Swagger UI for API documentation and testing.

# Inputs & Outputs
- **Inputs:** 
  - Audio files (e.g., `.wav`, `.mp3`) via `multipart/form-data`.
  - Optional model selection parameters.
- **Outputs:** 
  - Transcribed text payload in JSON format.
  - Status/progress updates for model downloads.

# Edge Cases & Security
- **Missing Models:** If a requested model is not found locally, trigger the service hub download or return a 404/400 with download instructions.
- **GPU Unavailability:** Implement graceful fallback to CPU or a clear 503 error if GPU is strictly required.
- **Invalid File Formats:** Return 400 Bad Request for unsupported media formats.
- **Concurrency & Resource Limits:** Prevent out-of-memory (OOM) errors by queuing requests or limiting concurrent Whisper instances.

# Task Checklist
- [x] Scaffold new REST API endpoints for transcription (`/api/transcript`)
- [x] Implement Swagger UI and OpenAPI documentation for the new endpoints
- [x] Create a service hub integration to download Whisper models on demand
- [x] Integrate Whisper inference logic with GPU support (e.g., via `llama-cpp-python` or native bindings)
- [x] Add basic error handling for failed transcriptions and large file support handling.
- [ ] Write scoped unit and integration tests.

# Definition of Done
- **Local DoD:**
  - Users can download models via the service hub.
  - API endpoint successfully transcribes valid audio files using the GPU.
  - Swagger UI accurately documents the new endpoints.
- **Global DoD:**
  - Code complies with NDE-OS global quality standards (no mocks, explicit error handling via `anyhow::Result`, zero panics).
  - Scoped tests run and pass successfully.
  - Feature aligns flawlessly with user goals.
