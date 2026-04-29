use serde_json::{json, Value};

pub fn paths() -> Value {
    json!({
        "/api/transcript": {
            "post": {
                "tags": ["whisper"],
                "summary": "Transcribe audio using Whisper",
                "operationId": "whisperTranscript",
                "description": "Upload an audio file to convert speech to text using OpenAI Whisper. Requires the `openai-whisper` package to be available in the voice runtime. Supports optional `model_size` (default: base).",
                "requestBody": {
                    "required": true,
                    "content": {
                        "multipart/form-data": {
                            "schema": {
                                "type": "object",
                                "required": ["audio"],
                                "properties": {
                                    "audio": {"type": "string", "format": "binary", "description": "Audio file to transcribe"},
                                    "model_size": {"type": "string", "description": "Whisper model size (tiny, base, small, medium, large, etc.)"}
                                }
                            }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Transcription successful",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "success": {"type": "boolean"},
                                        "message": {"type": "string"},
                                        "data": {
                                            "type": "object",
                                            "properties": {
                                                "text": {"type": "string", "description": "The transcribed text"},
                                                "segments": {
                                                    "type": "array",
                                                    "items": {
                                                        "type": "object",
                                                        "properties": {
                                                            "start": {"type": "number"},
                                                            "end": {"type": "number"},
                                                            "text": {"type": "string"}
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    "400": {"description": "Invalid input — missing audio file"},
                    "500": {"description": "Transcription failed"}
                }
            }
        }
    })
}
