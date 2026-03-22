use serde_json::json;

/// Full OpenAPI 3.0.3 specification for the AI Launcher API.
pub fn openapi_spec() -> serde_json::Value {
    json!({
        "openapi":"3.0.3",
        "info":{
            "title":"AI Launcher API",
            "version":"0.2.0",
            "description":"Cross-platform sandboxed AI App Launcher.\n\n**Supports:** Linux (native) and Windows (native, no WSL required)\n\n**Package manager:** uv (by Astral) — 10-100x faster than pip, auto-installs Python, per-app venvs\n\nBuilt in Rust with filesystem jailing, path validation, symlink defense, and environment isolation.",
            "license":{"name":"MIT"}
        },
        "servers":[{"url":"http://localhost:8080","description":"Local server"}],
        "tags":[
            {"name":"apps","description":"App lifecycle: install, launch, stop, uninstall"},
            {"name":"catalog","description":"Browse available AI apps"},
            {"name":"sandbox","description":"Sandbox security & disk usage"},
            {"name":"system","description":"Health & system info"}
        ],
        "paths":{
            "/api/health":{"get":{"tags":["system"],"summary":"Health check","operationId":"healthCheck","responses":{"200":{"description":"Healthy"}}}},
            "/api/system":{"get":{"tags":["system"],"summary":"System info (OS, Python, GPU)","operationId":"getSystemInfo","responses":{"200":{"description":"System details"}}}},
            "/api/catalog":{"get":{"tags":["catalog"],"summary":"List available apps","operationId":"getCatalog","responses":{"200":{"description":"App catalog"}}}},
            "/api/apps":{
                "get":{"tags":["apps"],"summary":"List installed apps","operationId":"listApps","responses":{"200":{"description":"Installed apps"}}},
                "post":{"tags":["apps"],"summary":"Install app into sandbox","operationId":"installApp",
                    "description":"Creates sandboxed workspace, verifies security, creates uv venv with pinned Python version, installs pip deps via uv (10-100x faster than pip).",
                    "requestBody":{"required":true,"content":{"application/json":{"schema":{"$ref":"#/components/schemas/InstallRequest"},
                        "example":{"manifest":{"id":"gradio-demo","name":"Gradio Demo","description":"Test app","author":"ai-launcher","python_version":"3","needs_gpu":false,"pip_deps":["gradio"],"launch_cmd":"python3 app.py","port":7860,"env":[],"disk_size":"~200MB","tags":["demo"]}}}}},
                    "responses":{"201":{"description":"Installed"},"400":{"description":"Failed"},"409":{"description":"Already installed"}}}
            },
            "/api/apps/{app_id}":{
                "get":{"tags":["apps"],"summary":"Get app details","operationId":"getApp","parameters":[{"name":"app_id","in":"path","required":true,"schema":{"type":"string"}}],"responses":{"200":{"description":"App details"},"404":{"description":"Not found"}}},
                "delete":{"tags":["apps"],"summary":"Uninstall app and remove workspace","operationId":"uninstallApp","parameters":[{"name":"app_id","in":"path","required":true,"schema":{"type":"string"}}],"responses":{"200":{"description":"Uninstalled"},"404":{"description":"Not found"}}}
            },
            "/api/apps/{app_id}/launch":{"post":{"tags":["apps"],"summary":"Launch app in sandbox","description":"Spawns process inside uv venv with jailed HOME, TMPDIR, TEMP, USERPROFILE, PYTHONPATH, APPDATA, VIRTUAL_ENV (cross-platform)","operationId":"launchApp","parameters":[{"name":"app_id","in":"path","required":true,"schema":{"type":"string"}}],"responses":{"200":{"description":"Launched"},"404":{"description":"Not installed"},"409":{"description":"Already running"}}}},
            "/api/apps/{app_id}/stop":{"post":{"tags":["apps"],"summary":"Stop running app","operationId":"stopApp","parameters":[{"name":"app_id","in":"path","required":true,"schema":{"type":"string"}}],"responses":{"200":{"description":"Stopped"},"404":{"description":"Not running"}}}},
            "/api/sandbox/{app_id}/verify":{"get":{"tags":["sandbox"],"summary":"Verify sandbox security","description":"Tests: path traversal (Unix+Windows paths), absolute escape, symlink escape, valid path resolution","operationId":"verifySandbox","parameters":[{"name":"app_id","in":"path","required":true,"schema":{"type":"string"}}],"responses":{"200":{"description":"Verification results"}}}},
            "/api/sandbox/{app_id}/disk":{"get":{"tags":["sandbox"],"summary":"Workspace disk usage","operationId":"getDiskUsage","parameters":[{"name":"app_id","in":"path","required":true,"schema":{"type":"string"}}],"responses":{"200":{"description":"Disk usage"}}}}
        },
        "components":{"schemas":{
            "AppManifest":{"type":"object","properties":{"id":{"type":"string"},"name":{"type":"string"},"description":{"type":"string"},"author":{"type":"string"},"python_version":{"type":"string"},"needs_gpu":{"type":"boolean"},"pip_deps":{"type":"array","items":{"type":"string"}},"launch_cmd":{"type":"string"},"port":{"type":"integer"},"disk_size":{"type":"string"},"tags":{"type":"array","items":{"type":"string"}}}},
            "AppStatus":{"type":"object","properties":{"state":{"type":"string","enum":["NotInstalled","Installed","Running","Error"]},"pid":{"type":"integer"},"port":{"type":"integer"}}},
            "InstalledApp":{"type":"object","properties":{"manifest":{"$ref":"#/components/schemas/AppManifest"},"status":{"$ref":"#/components/schemas/AppStatus"},"workspace":{"type":"string"},"installed_at":{"type":"string"},"last_run":{"type":"string"}}},
            "InstallRequest":{"type":"object","required":["manifest"],"properties":{"manifest":{"$ref":"#/components/schemas/AppManifest"}}},
            "SandboxVerifyResult":{"type":"object","properties":{"path_traversal_blocked":{"type":"boolean"},"absolute_escape_blocked":{"type":"boolean"},"symlink_escape_blocked":{"type":"boolean"},"valid_path_works":{"type":"boolean"},"sandbox_root":{"type":"string"},"platform":{"type":"string"}}},
            "SystemInfo":{"type":"object","properties":{"os":{"type":"string"},"arch":{"type":"string"},"python_version":{"type":"string"},"gpu_detected":{"type":"boolean"},"base_dir":{"type":"string"},"total_apps":{"type":"integer"},"running_apps":{"type":"integer"}}},
            "ApiResponse":{"type":"object","properties":{"success":{"type":"boolean"},"message":{"type":"string"},"data":{}}},
            "DiskUsage":{"type":"object","properties":{"app_id":{"type":"string"},"bytes":{"type":"integer"},"human_readable":{"type":"string"}}}
        }}
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
