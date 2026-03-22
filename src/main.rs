mod app_manager;
mod manifest;
mod sandbox;
mod uv_env;

use app_manager::AppManager;
use manifest::*;
use serde_json::{json, Value};
use std::io::Cursor;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use tiny_http::{Header, Method, Request, Response, Server};

/// Cross-platform base directory
fn get_base_dir() -> PathBuf {
    if cfg!(windows) {
        // %LOCALAPPDATA%\ai-launcher or fallback
        std::env::var("LOCALAPPDATA")
            .map(|p| PathBuf::from(p).join("ai-launcher"))
            .unwrap_or_else(|_| PathBuf::from("C:\\ai-launcher-data"))
    } else {
        dirs_or_home().join(".ai-launcher")
    }
}

fn dirs_or_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

// ─── JSON helpers ───

fn json_resp(status: u16, body: &Value) -> Response<Cursor<Vec<u8>>> {
    let data = serde_json::to_string_pretty(body).unwrap_or_default();
    Response::from_data(data.into_bytes())
        .with_status_code(tiny_http::StatusCode(status))
        .with_header(Header::from_bytes("Content-Type", "application/json").unwrap())
        .with_header(Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap())
}

fn ok<T: serde::Serialize>(msg: &str, data: T) -> Response<Cursor<Vec<u8>>> {
    json_resp(200, &json!({"success":true,"message":msg,"data":data}))
}
fn created<T: serde::Serialize>(msg: &str, data: T) -> Response<Cursor<Vec<u8>>> {
    json_resp(201, &json!({"success":true,"message":msg,"data":data}))
}
fn err(status: u16, msg: &str) -> Response<Cursor<Vec<u8>>> {
    json_resp(status, &json!({"success":false,"message":msg,"data":null}))
}
fn html(body: &str) -> Response<Cursor<Vec<u8>>> {
    Response::from_data(body.as_bytes().to_vec())
        .with_header(Header::from_bytes("Content-Type", "text/html; charset=utf-8").unwrap())
}

fn read_body(req: &mut Request) -> Option<String> {
    let mut buf = String::new();
    req.as_reader().read_to_string(&mut buf).ok()?;
    Some(buf)
}

// ─── Router ───

fn handle(req: &mut Request, mgr: &AppManager) -> Response<Cursor<Vec<u8>>> {
    let method = req.method().clone();
    let url = req.url().to_string();
    let path = url.split('?').next().unwrap_or(&url);

    if matches!(method, Method::Options) {
        return Response::from_data(Vec::new())
            .with_header(Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap())
            .with_header(Header::from_bytes("Access-Control-Allow-Methods", "GET,POST,DELETE,OPTIONS").unwrap())
            .with_header(Header::from_bytes("Access-Control-Allow-Headers", "Content-Type").unwrap());
    }

    match (method, path) {
        (Method::Get, "/swagger-ui" | "/swagger-ui/" | "/docs" | "/docs/") => html(SWAGGER_HTML),
        (Method::Get, "/api-docs/openapi.json") => json_resp(200, &openapi_spec()),
        (Method::Get, "/" | "") => html("<html><meta http-equiv='refresh' content='0;url=/swagger-ui/'></html>"),

        (Method::Get, "/api/health") => ok("AI Launcher is running", "ok"),

        (Method::Get, "/api/system") => {
            let py_cmd = AppManifest::python_cmd();
            let py = Command::new(py_cmd).arg("--version").output().ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string());
            let gpu = Command::new("nvidia-smi").output()
                .map(|o| o.status.success()).unwrap_or(false);
            ok("System info", json!({
                "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
                "python_version": py,
                "gpu_detected": gpu,
                "uv": mgr.uv_info(),
                "base_dir": mgr.base_dir().to_string_lossy(),
                "total_apps": mgr.total_count(),
                "running_apps": mgr.running_count(),
            }))
        }

        (Method::Get, "/api/catalog") => {
            let cat = mgr.catalog();
            ok(&format!("{} app(s) available", cat.len()), cat)
        }

        (Method::Get, "/api/apps") => {
            let apps = mgr.list_apps();
            ok(&format!("{} app(s) installed", apps.len()), apps)
        }

        (Method::Post, "/api/apps") => {
            let body = match read_body(req) {
                Some(b) => b,
                None => return err(400, "Missing request body"),
            };
            match serde_json::from_str::<InstallRequest>(&body) {
                Ok(ir) => match mgr.install(&ir.manifest) {
                    Ok(()) => created(
                        &format!("'{}' installed", ir.manifest.name),
                        mgr.get_app(&ir.manifest.id),
                    ),
                    Err(e) => {
                        let msg = e.to_string();
                        if msg.contains("already installed") { err(409, &msg) }
                        else { err(400, &msg) }
                    }
                },
                Err(e) => err(400, &format!("Invalid JSON: {}", e)),
            }
        }

        // Dynamic routes
        (ref m, p) if p.starts_with("/api/apps/") || p.starts_with("/api/sandbox/") => {
            let parts: Vec<&str> = p.trim_start_matches('/').split('/').collect();
            match (m, parts.as_slice()) {
                (Method::Get, ["api", "apps", id]) => match mgr.get_app(id) {
                    Some(app) => ok(&format!("App '{}'", id), app),
                    None => err(404, &format!("App '{}' not found", id)),
                },
                (Method::Delete, ["api", "apps", id]) => match mgr.uninstall(id) {
                    Ok(()) => ok(&format!("'{}' uninstalled", id), "uninstalled"),
                    Err(e) => err(404, &e.to_string()),
                },
                (Method::Post, ["api", "apps", id, "launch"]) => match mgr.launch(id) {
                    Ok((pid, port)) => ok(
                        &format!("Launched PID:{} port:{}", pid, port),
                        mgr.get_app(id),
                    ),
                    Err(e) => {
                        let msg = e.to_string();
                        if msg.contains("not installed") { err(404, &msg) }
                        else if msg.contains("already running") { err(409, &msg) }
                        else { err(500, &msg) }
                    }
                },
                (Method::Post, ["api", "apps", id, "stop"]) => match mgr.stop(id) {
                    Ok(()) => ok(&format!("'{}' stopped", id), "stopped"),
                    Err(e) => err(404, &e.to_string()),
                },
                (Method::Get, ["api", "sandbox", id, "verify"]) => match mgr.verify_sandbox(id) {
                    Ok(r) => ok("Sandbox verified", r),
                    Err(e) => err(500, &e.to_string()),
                },
                (Method::Get, ["api", "sandbox", id, "disk"]) => match mgr.disk_usage(id) {
                    Ok(bytes) => {
                        let human = if bytes > 1_073_741_824 { format!("{:.2} GB", bytes as f64/1_073_741_824.0) }
                        else if bytes > 1_048_576 { format!("{:.2} MB", bytes as f64/1_048_576.0) }
                        else { format!("{:.2} KB", bytes as f64/1024.0) };
                        ok("Disk usage", json!({"app_id":id,"bytes":bytes,"human_readable":human}))
                    }
                    Err(e) => err(404, &e.to_string()),
                },
                _ => err(404, &format!("Not found: {}", p)),
            }
        }

        _ => err(404, &format!("Not found: {}", path)),
    }
}

// ─── OpenAPI Spec ───

fn openapi_spec() -> Value {
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

// ─── Swagger UI ───

const SWAGGER_HTML: &str = r##"<!DOCTYPE html>
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

// ─── Main ───

fn main() {
    let base_dir = get_base_dir();
    std::fs::create_dir_all(&base_dir).ok();

    let mgr = Arc::new(AppManager::new(&base_dir).expect("Failed to init AppManager"));
    let server = Server::http("0.0.0.0:8080").expect("Failed to bind :8080");

    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    println!();
    println!("  +=============================================+");
    println!("  |  AI Launcher API v0.2.0                     |");
    println!("  |  Cross-Platform Sandboxed App Manager       |");
    println!("  +=============================================+");
    println!();
    println!("  Platform:    {}/{}", os, arch);
    println!("  Data dir:    {}", base_dir.display());
    println!("  Server:      http://localhost:8080");
    println!("  Swagger UI:  http://localhost:8080/swagger-ui/");
    println!("  OpenAPI:     http://localhost:8080/api-docs/openapi.json");
    println!();
    println!("  Endpoints:");
    println!("    GET    /api/health");
    println!("    GET    /api/system");
    println!("    GET    /api/catalog");
    println!("    GET    /api/apps");
    println!("    POST   /api/apps");
    println!("    GET    /api/apps/{{id}}");
    println!("    DELETE /api/apps/{{id}}");
    println!("    POST   /api/apps/{{id}}/launch");
    println!("    POST   /api/apps/{{id}}/stop");
    println!("    GET    /api/sandbox/{{id}}/verify");
    println!("    GET    /api/sandbox/{{id}}/disk");
    println!();

    loop {
        match server.recv() {
            Ok(mut request) => {
                let response = handle(&mut request, &mgr);
                if let Err(e) = request.respond(response) {
                    eprintln!("Response error: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Recv error: {}", e);
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    }
}
