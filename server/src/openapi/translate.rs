use serde_json::{json, Value};

pub fn paths() -> Value {
    json!({
        "/api/translate/providers":{
            "get":{
                "tags":["translate"],
                "summary":"List available translation providers",
                "operationId":"translateProviders",
                "responses":{"200":{"description":"List of providers","content":{"application/json":{"schema":{"type":"array","items":{"$ref":"#/components/schemas/TranslateProviderInfo"}}}}}}
            }
        },
        "/api/translate/srt":{
            "post":{
                "tags":["translate"],
                "summary":"Translate an SRT file (JSON)",
                "operationId":"translateSrt",
                "description":"Translate an SRT file via JSON payload using a specific provider (Google, LLM, or NDE Agent).",
                "requestBody":{
                    "required":true,
                    "content":{"application/json":{"schema":{"$ref":"#/components/schemas/TranslateSrtRequest"}}}
                },
                "responses":{
                    "200":{"description":"Translated SRT result","content":{"application/json":{"schema":{"$ref":"#/components/schemas/ApiResponse"}}}},
                    "400":{"description":"Invalid input"},
                    "500":{"description":"Translation failed"}
                }
            }
        },
        "/api/translate/srt-multipart":{
            "post":{
                "tags":["translate"],
                "summary":"Translate an SRT file (multipart form)",
                "operationId":"translateSrtMultipart",
                "description":"Translate an SRT file uploaded as multipart/form-data.",
                "requestBody":{
                    "required":true,
                    "content":{
                        "multipart/form-data":{
                            "schema":{
                                "type":"object",
                                "required":["file"],
                                "properties":{
                                    "file":{"type":"string","format":"binary","description":"SRT file to translate"},
                                    "source_lang":{"type":"string","description":"Source language code (default: en)"},
                                    "target_lang":{"type":"string","description":"Target language code (default: km)"},
                                    "provider":{"type":"string","description":"Provider config JSON (e.g. {\"type\":\"nde_agent\"})"}
                                }
                            }
                        }
                    }
                },
                "responses":{
                    "200":{"description":"Translated SRT result","content":{"application/json":{"schema":{"$ref":"#/components/schemas/ApiResponse"}}}},
                    "400":{"description":"Invalid input"},
                    "500":{"description":"Translation failed"}
                }
            }
        },
        "/api/translate/text":{
            "post":{
                "tags":["translate"],
                "summary":"Translate a single text string",
                "operationId":"translateText",
                "description":"Translate a short text string.",
                "requestBody":{
                    "required":true,
                    "content":{"application/json":{"schema":{"$ref":"#/components/schemas/TranslateTextRequest"}}}
                },
                "responses":{
                    "200":{"description":"Translated text","content":{"application/json":{"schema":{"$ref":"#/components/schemas/ApiResponse"}}}},
                    "400":{"description":"Invalid input"},
                    "500":{"description":"Translation failed"}
                }
            }
        }
    })
}
