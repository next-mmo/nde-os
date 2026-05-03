pub mod system;
pub mod catalog;
pub mod apps;
pub mod sandbox;
pub mod agent;
pub mod kfa;
pub mod translate;
pub mod schemas;
pub mod whisper;

use serde_json::{json, Value};

/// Full OpenAPI 3.0.3 specification for the AI Launcher API.
pub fn openapi_spec() -> Value {
    let mut paths = serde_json::Map::new();
    paths.extend(system::paths().as_object().unwrap().clone());
    paths.extend(catalog::paths().as_object().unwrap().clone());
    paths.extend(apps::paths().as_object().unwrap().clone());
    paths.extend(sandbox::paths().as_object().unwrap().clone());
    paths.extend(agent::paths().as_object().unwrap().clone());
    paths.extend(kfa::paths().as_object().unwrap().clone());
    paths.extend(translate::paths().as_object().unwrap().clone());
    paths.extend(whisper::paths().as_object().unwrap().clone());

    let mut all_schemas = serde_json::Map::new();
    all_schemas.extend(schemas::components().as_object().unwrap().clone());

    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "AI Launcher API",
            "version": "0.2.0",
            "description": "Cross-platform sandboxed AI App Launcher.\n\n**Supports:** Linux (native) and Windows (native, no WSL required)\n\n**Package manager:** uv (by Astral) — 10-100x faster than pip, auto-installs Python, per-app venvs\n\nBuilt in Rust with filesystem jailing, path validation, symlink defense, and environment isolation.",
            "license": {"name": "MIT"}
        },
        "servers": [{"url": "http://localhost:8080", "description": "Local server"}],
        "tags": [
            {"name":"apps","description":"App lifecycle: install, launch, stop, uninstall"},
            {"name":"catalog","description":"Browse available AI apps"},
            {"name":"sandbox","description":"Sandbox security & disk usage"},
            {"name":"store","description":"Store: upload apps via folder, zip, or git URL"},
            {"name":"system","description":"Health & system info"},
            {"name":"agent","description":"Agent chat, conversations, and config"},
            {"name":"kfa","description":"Khmer Forced Aligner — word-level timestamp alignment using wav2vec2 CTC ONNX"},
            {"name":"translate","description":"Standalone SRT translation service with pluggable providers"},
            {"name":"whisper","description":"Whisper audio transcription operations"}
        ],
        "paths": paths,
        "components": {
            "schemas": all_schemas
        }
    })
}

/// Swagger UI HTML (loaded from CDN)
pub const SWAGGER_HTML: &str = r##"<!DOCTYPE html>
<html lang="en"><head><meta charset="UTF-8">
<title>AI Launcher API</title>
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui.css">
<style>
body{margin:0;background:#0a0a14}
.swagger-ui .topbar{display:none}
#hdr{background:linear-gradient(135deg,#0a0a14,#101020);padding:24px 36px;border-bottom:1px solid #1a1a2e}
#hdr h1{margin:0;color:#d0d0e8;font-family:system-ui;font-size:24px;font-weight:800}
#hdr p{margin:5px 0 0;color:#50506a;font-family:monospace;font-size:13px}
#hdr span{color:#5ce0a0}
#hdr .plat{margin-top:8px;display:flex;gap:8px}
#hdr .plat span{padding:3px 10px;border-radius:4px;font-size:11px;font-family:monospace}
.swagger-ui{background:#f8f8fc}
</style></head><body>
<div id="hdr">
  <h1>AI Launcher API <span>v0.2.0</span></h1>
  <p>Cross-platform sandboxed AI app manager — Rust + tiny_http</p>
  <div class="plat">
    <span style="background:#0a2016;color:#5ce0a0;border:1px solid #1a3a28">Linux</span>
    <span style="background:#081a30;color:#5eaaff;border:1px solid #103050">Windows</span>
    <span style="background:#1a1800;color:#c8a820;border:1px solid #2a2808">No WSL needed</span>
    <span style="background:#1a0a20;color:#c080f0;border:1px solid #2a1840">uv powered</span>
  </div>
</div>
<div id="swagger-ui"></div>
<script src="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
<script>SwaggerUIBundle({url:'/api-docs/openapi.json',dom_id:'#swagger-ui',deepLinking:true,presets:[SwaggerUIBundle.presets.apis,SwaggerUIBundle.SwaggerUIStandalonePreset],layout:'BaseLayout',defaultModelsExpandDepth:2,docExpansion:'list',tryItOutEnabled:true});</script>
</body></html>"##;
