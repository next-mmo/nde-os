use ai_launcher_core::app_manager::AppManager;
use ai_launcher_core::manifest::*;
use serde_json::json;
use std::io::Cursor;
use std::process::Command;
use tiny_http::{Header, Method, Request, Response};

use crate::response::*;

/// Handle CORS preflight
pub fn cors_preflight() -> Response<Cursor<Vec<u8>>> {
    Response::from_data(Vec::new())
        .with_header(Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap())
        .with_header(Header::from_bytes("Access-Control-Allow-Methods", "GET,POST,DELETE,OPTIONS").unwrap())
        .with_header(Header::from_bytes("Access-Control-Allow-Headers", "Content-Type").unwrap())
}

/// GET /api/health
pub fn health() -> Response<Cursor<Vec<u8>>> {
    ok("AI Launcher is running", "ok")
}

/// GET /api/system
pub fn system_info(mgr: &AppManager) -> Response<Cursor<Vec<u8>>> {
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

/// GET /api/catalog
pub fn catalog(mgr: &AppManager) -> Response<Cursor<Vec<u8>>> {
    let cat = mgr.catalog();
    ok(&format!("{} app(s) available", cat.len()), cat)
}

/// GET /api/apps
pub fn list_apps(mgr: &AppManager) -> Response<Cursor<Vec<u8>>> {
    let apps = mgr.list_apps();
    ok(&format!("{} app(s) installed", apps.len()), apps)
}

/// POST /api/apps
pub fn install_app(req: &mut Request, mgr: &AppManager) -> Response<Cursor<Vec<u8>>> {
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

/// GET /api/apps/{id}
pub fn get_app(id: &str, mgr: &AppManager) -> Response<Cursor<Vec<u8>>> {
    match mgr.get_app(id) {
        Some(app) => ok(&format!("App '{}'", id), app),
        None => err(404, &format!("App '{}' not found", id)),
    }
}

/// DELETE /api/apps/{id}
pub fn uninstall_app(id: &str, mgr: &AppManager) -> Response<Cursor<Vec<u8>>> {
    match mgr.uninstall(id) {
        Ok(()) => ok(&format!("'{}' uninstalled", id), "uninstalled"),
        Err(e) => err(404, &e.to_string()),
    }
}

/// POST /api/apps/{id}/launch
pub fn launch_app(id: &str, mgr: &AppManager) -> Response<Cursor<Vec<u8>>> {
    match mgr.launch(id) {
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
    }
}

/// POST /api/apps/{id}/stop
pub fn stop_app(id: &str, mgr: &AppManager) -> Response<Cursor<Vec<u8>>> {
    match mgr.stop(id) {
        Ok(()) => ok(&format!("'{}' stopped", id), "stopped"),
        Err(e) => err(404, &e.to_string()),
    }
}

/// GET /api/sandbox/{id}/verify
pub fn verify_sandbox(id: &str, mgr: &AppManager) -> Response<Cursor<Vec<u8>>> {
    match mgr.verify_sandbox(id) {
        Ok(r) => ok("Sandbox verified", r),
        Err(e) => err(500, &e.to_string()),
    }
}

/// GET /api/sandbox/{id}/disk
pub fn disk_usage(id: &str, mgr: &AppManager) -> Response<Cursor<Vec<u8>>> {
    match mgr.disk_usage(id) {
        Ok(bytes) => {
            let human = if bytes > 1_073_741_824 { format!("{:.2} GB", bytes as f64/1_073_741_824.0) }
            else if bytes > 1_048_576 { format!("{:.2} MB", bytes as f64/1_048_576.0) }
            else { format!("{:.2} KB", bytes as f64/1024.0) };
            ok("Disk usage", json!({"app_id":id,"bytes":bytes,"human_readable":human}))
        }
        Err(e) => err(404, &e.to_string()),
    }
}
