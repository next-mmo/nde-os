use serde_json::{json, Value};

pub fn components() -> Value {
    json!({
        "AppManifest":{"type":"object","properties":{"id":{"type":"string"},"name":{"type":"string"},"description":{"type":"string"},"author":{"type":"string"},"python_version":{"type":"string"},"needs_gpu":{"type":"boolean"},"pip_deps":{"type":"array","items":{"type":"string"}},"launch_cmd":{"type":"string"},"port":{"type":"integer"},"disk_size":{"type":"string"},"tags":{"type":"array","items":{"type":"string"}}}},
        "AppStatus":{"type":"object","properties":{"state":{"type":"string","enum":["NotInstalled","Installed","Running","Error"]},"pid":{"type":"integer"},"port":{"type":"integer"}}},
        "InstalledApp":{"type":"object","properties":{"manifest":{"$ref":"#/components/schemas/AppManifest"},"status":{"$ref":"#/components/schemas/AppStatus"},"workspace":{"type":"string"},"installed_at":{"type":"string"},"last_run":{"type":"string"}}},
        "InstallRequest":{"type":"object","required":["manifest"],"properties":{"manifest":{"$ref":"#/components/schemas/AppManifest"}}},
        "StoreUploadRequest":{"type":"object","required":["source_type"],"properties":{
            "source_type":{"type":"string","enum":["folder","zip","git_url"],"description":"Upload source type"},
            "source_path":{"type":"string","description":"Local path to folder or ZIP file (required for folder/zip)"},
            "git_url":{"type":"string","description":"Git repository URL (required for git_url)"}
        }},
        "StoreUploadResult":{"type":"object","properties":{
            "accepted":{"type":"boolean","description":"Whether the upload was accepted"},
            "app_id":{"type":"string","nullable":true},
            "app_name":{"type":"string","nullable":true},
            "validation_errors":{"type":"array","items":{"$ref":"#/components/schemas/ValidationError"}},
            "install_log":{"type":"array","items":{"type":"string"}}
        }},
        "ValidationError":{"type":"object","properties":{
            "field":{"type":"string","description":"Field that failed validation"},
            "message":{"type":"string","description":"Error message"}
        }},
        "LaunchResult":{"type":"object","properties":{"pid":{"type":"integer"},"port":{"type":"integer"}}},
        "SandboxVerifyResult":{"type":"object","properties":{"path_traversal_blocked":{"type":"boolean"},"absolute_escape_blocked":{"type":"boolean"},"symlink_escape_blocked":{"type":"boolean"},"valid_path_works":{"type":"boolean"},"sandbox_root":{"type":"string"},"platform":{"type":"string"}}},
        "SystemInfo":{"type":"object","properties":{"os":{"type":"string"},"arch":{"type":"string"},"python_version":{"type":"string"},"gpu_detected":{"type":"boolean"},"base_dir":{"type":"string"},"total_apps":{"type":"integer"},"running_apps":{"type":"integer"}}},
        "ResourceUsage":{"type":"object","properties":{"memory_used_bytes":{"type":"integer"},"memory_total_bytes":{"type":"integer"},"memory_percent":{"type":"integer"},"disk_used_bytes":{"type":"integer"},"disk_total_bytes":{"type":"integer"},"disk_percent":{"type":"integer"},"disk_mount_point":{"type":"string"}}},
        "ApiResponse":{"type":"object","properties":{"success":{"type":"boolean"},"message":{"type":"string"},"data":{}}},
        "DiskUsage":{"type":"object","properties":{"app_id":{"type":"string"},"bytes":{"type":"integer"},"human_readable":{"type":"string"}}},
        "ChatRequest":{"type":"object","required":["message"],"properties":{"message":{"type":"string","description":"User message to send to the agent"},"conversation_id":{"type":"string","description":"Optional conversation ID to continue"}}},
        "ChatResponse":{"type":"object","properties":{"response":{"type":"string","description":"Agent's response text"},"conversation_id":{"type":"string","description":"Conversation ID for follow-up messages"}}},
        "ConversationSummary":{"type":"object","properties":{"id":{"type":"string"},"title":{"type":"string"},"channel":{"type":"string"},"created_at":{"type":"string"},"updated_at":{"type":"string"}}},
        "StoredMessage":{"type":"object","properties":{"id":{"type":"integer"},"role":{"type":"string"},"content":{"type":"string","nullable":true},"tool_calls":{"type":"string","nullable":true},"tool_call_id":{"type":"string","nullable":true},"created_at":{"type":"string"}}},
        "AgentConfigInfo":{"type":"object","properties":{"name":{"type":"string"},"provider":{"type":"string"},"model":{"type":"string"},"max_iterations":{"type":"integer"},"tools":{"type":"array","items":{"type":"string"}},"workspace":{"type":"string"}}},
        "KfaAlignJsonRequest":{
            "type":"object",
            "required":["audio_base64","text"],
            "properties":{
                "audio_base64":{"type":"string","description":"Base64-encoded WAV audio file (16 kHz mono)"},
                "text":{"type":"string","description":"Khmer text transcript — one sentence per line"}
            }
        },
        "KfaSegment":{
            "type":"object",
            "properties":{
                "text":{"type":"string","description":"Aligned text segment"},
                "start":{"type":"number","format":"double","description":"Segment start time in seconds (inclusive of gap to previous)"},
                "end":{"type":"number","format":"double","description":"Segment end time in seconds"},
                "actual_start":{"type":"number","format":"double","description":"Precise start (excluding padding)"},
                "actual_end":{"type":"number","format":"double","description":"Precise end (excluding padding)"},
                "score":{"type":"number","format":"float","description":"Confidence score [0,1]"}
            }
        },
        "KfaAlignResponse":{
            "type":"object",
            "properties":{
                "success":{"type":"boolean"},
                "message":{"type":"string"},
                "data":{
                    "type":"object",
                    "properties":{
                        "segments":{"type":"array","items":{"$ref":"#/components/schemas/KfaSegment"}}
                    }
                }
            }
        },
        "KfaSrtResponse":{
            "type":"object",
            "properties":{
                "success":{"type":"boolean"},
                "message":{"type":"string"},
                "data":{
                    "type":"object",
                    "properties":{
                        "srt":{"type":"string","description":"Raw SRT formatted subtitle text"}
                    }
                }
            }
        },
        "KfaTranscribeResponse":{
            "type":"object",
            "properties":{
                "success":{"type":"boolean"},
                "message":{"type":"string"},
                "data":{
                    "type":"object",
                    "properties":{
                        "text":{"type":"string","description":"Transcribed Khmer text"}
                    }
                }
            }
        },
        "TranslateProviderInfo":{
            "type":"object",
            "properties":{
                "id":{"type":"string"},
                "name":{"type":"string"},
                "description":{"type":"string"},
                "requires_api_key":{"type":"boolean"},
                "accuracy_tier":{"type":"string"}
            }
        },
        "TranslateSrtRequest":{
            "type":"object",
            "properties":{
                "srt_content":{"type":"string","description":"SRT content as a string","nullable":true},
                "srt_path":{"type":"string","description":"Path to SRT file on disk","nullable":true},
                "source_lang":{"type":"string","description":"Source language code","default":"en"},
                "target_lang":{"type":"string","description":"Target language code","default":"km"},
                "provider":{"type":"object","description":"Provider config (type: 'google', 'llm', 'nde_agent')","nullable":true},
                "output_path":{"type":"string","description":"Path to write output SRT","nullable":true}
            },
            "example":{
                "srt_content":"1\n00:00:00,000 --> 00:00:05,000\nHello world",
                "source_lang":"en",
                "target_lang":"km",
                "provider":{
                    "type":"nde_agent"
                }
            }
        },
        "TranslateTextRequest":{
            "type":"object",
            "required":["text"],
            "properties":{
                "text":{"type":"string","description":"Text to translate"},
                "source_lang":{"type":"string","description":"Source language code","default":"en"},
                "target_lang":{"type":"string","description":"Target language code","default":"km"},
                "provider":{"type":"object","description":"Provider config","nullable":true}
            },
            "example":{
                "text":"Hello world",
                "source_lang":"en",
                "target_lang":"km",
                "provider":{
                    "type":"nde_agent"
                }
            }
        }
    })
}
