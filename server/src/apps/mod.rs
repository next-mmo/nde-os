use ai_launcher_core::app_manager::AppManager;
use ai_launcher_core::manifest::*;
use ai_launcher_core::system_metrics::snapshot_resource_usage;
use serde_json::json;
use std::process::Command;
use tiny_http::{Header, Request, Response};

use crate::response::*;

/// Handle CORS preflight
pub fn cors_preflight() -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_data(Vec::new())
        .with_header(Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap())
        .with_header(
            Header::from_bytes(
                "Access-Control-Allow-Methods",
                "GET,POST,PUT,DELETE,OPTIONS",
            )
            .unwrap(),
        )
        .with_header(Header::from_bytes("Access-Control-Allow-Headers", "Content-Type").unwrap())
}

/// GET /api/health
pub fn health() -> HttpResponse {
    ok("AI Launcher is running", "ok")
}

/// GET /api/system
pub fn system_info(mgr: &AppManager) -> HttpResponse {
    let py_cmd = AppManifest::python_cmd();
    let py = Command::new(py_cmd)
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());
    let gpu = Command::new("nvidia-smi")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    ok(
        "System info",
        json!({
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "python_version": py,
            "gpu_detected": gpu,
            "uv": mgr.uv_info(),
            "base_dir": mgr.base_dir().to_string_lossy(),
            "total_apps": mgr.total_count(),
            "running_apps": mgr.running_count(),
        }),
    )
}

/// GET /api/system/resources
pub fn system_resources(mgr: &AppManager) -> HttpResponse {
    match snapshot_resource_usage(mgr.base_dir()) {
        Ok(usage) => ok("System resource usage", usage),
        Err(error) => err(500, &error.to_string()),
    }
}

/// GET /api/catalog
pub fn catalog(mgr: &AppManager) -> HttpResponse {
    let cat = mgr.catalog();
    ok(&format!("{} app(s) available", cat.len()), cat)
}

/// GET /api/apps
pub fn list_apps(mgr: &AppManager) -> HttpResponse {
    let apps = mgr.list_apps();
    ok(&format!("{} app(s) installed", apps.len()), apps)
}

/// POST /api/apps
pub fn install_app(req: &mut Request, mgr: &AppManager) -> HttpResponse {
    let ir: InstallRequest = match parse_body(req) {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    match mgr.install(&ir.manifest) {
        Ok(()) => created(
            &format!("'{}' installed", ir.manifest.name),
            mgr.get_app(&ir.manifest.id),
        ),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("already installed") {
                err(409, &msg)
            } else {
                err(400, &msg)
            }
        }
    }
}

/// GET /api/apps/{id}
pub fn get_app(id: &str, mgr: &AppManager) -> HttpResponse {
    match mgr.get_app(id) {
        Some(app) => ok(&format!("App '{}'", id), app),
        None => err(404, &format!("App '{}' not found", id)),
    }
}

/// DELETE /api/apps/{id}
pub fn uninstall_app(id: &str, mgr: &AppManager) -> HttpResponse {
    match mgr.uninstall(id) {
        Ok(()) => ok(&format!("'{}' uninstalled", id), "uninstalled"),
        Err(e) => err(404, &e.to_string()),
    }
}

/// POST /api/apps/{id}/launch
pub fn launch_app(id: &str, mgr: &AppManager) -> HttpResponse {
    match mgr.launch(id) {
        Ok((pid, port)) => ok(
            &format!("Launched PID:{} port:{}", pid, port),
            json!({ "pid": pid, "port": port }),
        ),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("not installed") {
                err(404, &msg)
            } else if msg.contains("already running") {
                err(409, &msg)
            } else {
                err(500, &msg)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_base_dir() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("ai-launcher-server-test-{}", unique));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn test_manifest() -> AppManifest {
        AppManifest {
            id: "server-launch-test".into(),
            name: "Server Launch Test".into(),
            description: "Regression fixture for HTTP launch payloads".into(),
            author: "tests".into(),
            repo: None,
            runtime: ai_launcher_core::manifest::AppRuntime::Python,
            python_version: "3".into(),
            node_version: None,
            package_manager: None,
            needs_gpu: false,
            pip_deps: vec![],
            launch_cmd: if cfg!(windows) {
                "ping 127.0.0.1 -n 30 > NUL".into()
            } else {
                "sleep 30".into()
            },
            port: 43123,
            env: vec![],
            disk_size: "1MB".into(),
            tags: vec!["test".into()],
        }
    }

    fn response_json(response: Response<std::io::Cursor<Vec<u8>>>) -> serde_json::Value {
        let mut body = String::new();
        response.into_reader().read_to_string(&mut body).unwrap();
        serde_json::from_str(&body).unwrap()
    }

    #[test]
    fn launch_app_returns_pid_and_port_payload() {
        let base_dir = temp_base_dir();
        let mgr = AppManager::new(&base_dir).unwrap();
        let manifest = test_manifest();

        mgr.install(&manifest).unwrap();
        let response = launch_app(&manifest.id, &mgr);
        let json = response_json(response);

        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["port"].as_u64(), Some(manifest.port as u64));
        assert!(json["data"]["pid"].as_u64().unwrap() > 0);
        assert!(json["data"].get("manifest").is_none());

        mgr.stop(&manifest.id).ok();
        mgr.uninstall(&manifest.id).ok();
        std::fs::remove_dir_all(base_dir).ok();
    }

    #[test]
    fn system_resources_returns_percentages() {
        let base_dir = temp_base_dir();
        let mgr = AppManager::new(&base_dir).unwrap();

        let response = system_resources(&mgr);
        let json = response_json(response);

        assert_eq!(json["success"], true);
        assert!(json["data"]["memory_total_bytes"].as_u64().unwrap() > 0);
        assert!(json["data"]["disk_total_bytes"].as_u64().unwrap() > 0);
        assert!(json["data"]["memory_percent"].as_u64().unwrap() <= 100);
        assert!(json["data"]["disk_percent"].as_u64().unwrap() <= 100);

        std::fs::remove_dir_all(base_dir).ok();
    }
}

/// POST /api/apps/{id}/stop
pub fn stop_app(id: &str, mgr: &AppManager) -> HttpResponse {
    match mgr.stop(id) {
        Ok(()) => ok(&format!("'{}' stopped", id), "stopped"),
        Err(e) => err(404, &e.to_string()),
    }
}

/// GET /api/sandbox/{id}/verify
pub fn verify_sandbox(id: &str, mgr: &AppManager) -> HttpResponse {
    match mgr.verify_sandbox(id) {
        Ok(r) => ok("Sandbox verified", r),
        Err(e) => err(500, &e.to_string()),
    }
}

/// GET /api/sandbox/{id}/disk
pub fn disk_usage(id: &str, mgr: &AppManager) -> HttpResponse {
    match mgr.disk_usage(id) {
        Ok(bytes) => {
            let human = if bytes > 1_073_741_824 {
                format!("{:.2} GB", bytes as f64 / 1_073_741_824.0)
            } else if bytes > 1_048_576 {
                format!("{:.2} MB", bytes as f64 / 1_048_576.0)
            } else {
                format!("{:.2} KB", bytes as f64 / 1024.0)
            };
            ok(
                "Disk usage",
                json!({"app_id":id,"bytes":bytes,"human_readable":human}),
            )
        }
        Err(e) => err(404, &e.to_string()),
    }
}

/// POST /api/store/upload
/// Accept folder, zip, or git url uploads with validation and trial install
pub fn store_upload(req: &mut Request, mgr: &AppManager) -> HttpResponse {
    let upload_req: StoreUploadRequest = match parse_body(req) {
        Ok(r) => r,
        Err(resp) => return resp,
    };

    match mgr.upload_to_store(&upload_req) {
        Ok(result) => {
            if result.accepted {
                created(
                    &format!(
                        "App '{}' uploaded and installed successfully",
                        result.app_name.as_deref().unwrap_or("unknown")
                    ),
                    &result,
                )
            } else {
                json_resp(
                    400,
                    &json!({
                        "success": false,
                        "message": "Upload validation or install failed",
                        "data": result,
                    }),
                )
            }
        }
        Err(e) => err(500, &format!("Upload error: {}", e)),
    }
}
