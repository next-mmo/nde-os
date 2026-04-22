use serde_json::{json, Value};

pub fn paths() -> Value {
    json!({
        "/api/kfa/align":{
            "post":{
                "tags":["kfa"],
                "summary":"Forced-align Khmer audio (multipart)",
                "operationId":"kfaAlignMultipart",
                "description":"Upload a 16 kHz mono WAV file and a Khmer text transcript. Returns word-level start/end timestamps.\n\n**Downloads the ONNX model (~150 MB) on first call.** Subsequent calls are instant.\n\n`audio` must be WAV (16 kHz mono strongly preferred; other sample rates are resampled).",
                "requestBody":{
                    "required":true,
                    "content":{
                        "multipart/form-data":{
                            "schema":{
                                "type":"object",
                                "required":["audio","text"],
                                "properties":{
                                    "audio":{"type":"string","format":"binary","description":"WAV audio file (16 kHz mono)"},
                                    "text":{"type":"string","description":"Khmer transcript, one sentence per line"}
                                }
                            }
                        }
                    }
                },
                "responses":{
                    "200":{"description":"Alignment results","content":{"application/json":{"schema":{"$ref":"#/components/schemas/KfaAlignResponse"}}}},
                    "400":{"description":"Invalid input — missing audio/text field or bad WAV format"},
                    "500":{"description":"Alignment failed or model unavailable"}
                }
            }
        },
        "/api/kfa/align-json":{
            "post":{
                "tags":["kfa"],
                "summary":"Forced-align Khmer audio (JSON/base64)",
                "operationId":"kfaAlignJson",
                "description":"Same as `/api/kfa/align` but accepts a JSON body with the WAV audio base64-encoded. Convenient for automation scripts.",
                "requestBody":{
                    "required":true,
                    "content":{
                        "application/json":{
                            "schema":{"$ref":"#/components/schemas/KfaAlignJsonRequest"},
                            "example":{
                                "audio_base64":"UklGRiQAAABXQVZFZm10IBAAAA…",
                                "text":"ការប្រើប្រាស់បច្ចេកវិទ្យា"
                            }
                        }
                    }
                },
                "responses":{
                    "200":{"description":"Alignment results","content":{"application/json":{"schema":{"$ref":"#/components/schemas/KfaAlignResponse"}}}},
                    "400":{"description":"Invalid base64, missing fields, or bad WAV"},
                    "500":{"description":"Alignment failed"}
                }
            }
        },
        "/api/kfa/align-srt":{
            "post":{
                "tags":["kfa"],
                "summary":"Forced-align Khmer audio (multipart) -> SRT",
                "operationId":"kfaAlignSrtMultipart",
                "description":"Upload a 16 kHz mono WAV file and a Khmer text transcript. Returns SRT subtitle text directly.",
                "requestBody":{
                    "required":true,
                    "content":{
                        "multipart/form-data":{
                            "schema":{
                                "type":"object",
                                "required":["audio","text"],
                                "properties":{
                                    "audio":{"type":"string","format":"binary","description":"WAV audio file (16 kHz mono)"},
                                    "text":{"type":"string","description":"Khmer transcript, one sentence per line"}
                                }
                            }
                        }
                    }
                },
                "responses":{
                    "200":{"description":"SRT generation successful","content":{"application/json":{"schema":{"$ref":"#/components/schemas/KfaSrtResponse"}}}},
                    "400":{"description":"Invalid input"},
                    "500":{"description":"Alignment failed"}
                }
            }
        },
        "/api/kfa/align-srt-json":{
            "post":{
                "tags":["kfa"],
                "summary":"Forced-align Khmer audio (JSON) -> SRT",
                "operationId":"kfaAlignSrtJson",
                "description":"Accepts JSON body with base64 audio and text. Returns SRT subtitle text directly.",
                "requestBody":{
                    "required":true,
                    "content":{"application/json":{"schema":{"$ref":"#/components/schemas/KfaAlignJsonRequest"}}}
                },
                "responses":{
                    "200":{"description":"SRT generation successful","content":{"application/json":{"schema":{"$ref":"#/components/schemas/KfaSrtResponse"}}}},
                    "400":{"description":"Invalid input"},
                    "500":{"description":"Alignment failed"}
                }
            }
        },
        "/api/kfa/transcribe":{
            "post":{
                "tags":["kfa"],
                "summary":"Transcribe Khmer audio (multipart)",
                "operationId":"kfaTranscribeMultipart",
                "description":"Upload a 16 kHz mono WAV file. Returns raw Khmer transcript using CTC greedy decoding.",
                "requestBody":{
                    "required":true,
                    "content":{
                        "multipart/form-data":{
                            "schema":{
                                "type":"object",
                                "required":["audio"],
                                "properties":{
                                    "audio":{"type":"string","format":"binary","description":"WAV audio file (16 kHz mono)"}
                                }
                            }
                        }
                    }
                },
                "responses":{
                    "200":{"description":"Transcription successful","content":{"application/json":{"schema":{"$ref":"#/components/schemas/KfaTranscribeResponse"}}}},
                    "400":{"description":"Invalid input"},
                    "500":{"description":"Transcription failed"}
                }
            }
        },
        "/api/kfa/transcribe-json":{
            "post":{
                "tags":["kfa"],
                "summary":"Transcribe Khmer audio (JSON)",
                "operationId":"kfaTranscribeJson",
                "description":"Accepts JSON body with base64 audio. Returns raw Khmer transcript using CTC greedy decoding.",
                "requestBody":{
                    "required":true,
                    "content":{
                        "application/json":{
                            "schema":{
                                "type":"object",
                                "required":["audio_base64"],
                                "properties":{
                                    "audio_base64":{"type":"string","description":"Base64-encoded WAV audio file"}
                                }
                            }
                        }
                    }
                },
                "responses":{
                    "200":{"description":"Transcription successful","content":{"application/json":{"schema":{"$ref":"#/components/schemas/KfaTranscribeResponse"}}}},
                    "400":{"description":"Invalid input"},
                    "500":{"description":"Transcription failed"}
                }
            }
        }
    })
}
