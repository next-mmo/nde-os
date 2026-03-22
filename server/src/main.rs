mod handlers;
mod openapi;
mod response;

use ai_launcher_core::app_manager::AppManager;
use std::path::PathBuf;
use std::sync::Arc;
use tiny_http::{Method, Request, Server};

use response::*;

/// Cross-platform base directory
fn get_base_dir() -> PathBuf {
    if cfg!(windows) {
        std::env::var("LOCALAPPDATA")
            .map(|p| PathBuf::from(p).join("ai-launcher"))
            .unwrap_or_else(|_| PathBuf::from("C:\\ai-launcher-data"))
    } else {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp"))
            .join(".ai-launcher")
    }
}

/// Route a request to the appropriate handler
fn route(req: &mut Request, mgr: &AppManager) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    let method = req.method().clone();
    let url = req.url().to_string();
    let path = url.split('?').next().unwrap_or(&url);

    // CORS preflight
    if matches!(method, Method::Options) {
        return handlers::cors_preflight();
    }

    // Static routes
    match (method.clone(), path) {
        (Method::Get, "/swagger-ui" | "/swagger-ui/" | "/docs" | "/docs/") => {
            return html(openapi::SWAGGER_HTML);
        }
        (Method::Get, "/api-docs/openapi.json") => {
            return json_resp(200, &openapi::openapi_spec());
        }
        (Method::Get, "/" | "") => {
            return html("<html><meta http-equiv='refresh' content='0;url=/swagger-ui/'></html>");
        }
        (Method::Get, "/api/health") => return handlers::health(),
        (Method::Get, "/api/system") => return handlers::system_info(mgr),
        (Method::Get, "/api/system/resources") => return handlers::system_resources(mgr),
        (Method::Get, "/api/catalog") => return handlers::catalog(mgr),
        (Method::Get, "/api/apps") => return handlers::list_apps(mgr),
        (Method::Post, "/api/apps") => return handlers::install_app(req, mgr),
        (Method::Post, "/api/store/upload") => return handlers::store_upload(req, mgr),
        _ => {}
    }

    // Dynamic routes: /api/apps/{id}/... and /api/sandbox/{id}/...
    if path.starts_with("/api/apps/") || path.starts_with("/api/sandbox/") {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        return match (method, parts.as_slice()) {
            (Method::Get,    ["api", "apps", id])            => handlers::get_app(id, mgr),
            (Method::Delete, ["api", "apps", id])            => handlers::uninstall_app(id, mgr),
            (Method::Post,   ["api", "apps", id, "launch"])  => handlers::launch_app(id, mgr),
            (Method::Post,   ["api", "apps", id, "stop"])    => handlers::stop_app(id, mgr),
            (Method::Get,    ["api", "sandbox", id, "verify"]) => handlers::verify_sandbox(id, mgr),
            (Method::Get,    ["api", "sandbox", id, "disk"])   => handlers::disk_usage(id, mgr),
            _ => err(404, &format!("Not found: {}", path)),
        };
    }

    if path.starts_with("/api/store/") {
        return err(404, &format!("Not found: {}", path));
    }

    err(404, &format!("Not found: {}", path))
}

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
    println!("    GET    /api/system/resources");
    println!("    GET    /api/catalog");
    println!("    GET    /api/apps");
    println!("    POST   /api/apps");
    println!("    GET    /api/apps/{{id}}");
    println!("    DELETE /api/apps/{{id}}");
    println!("    POST   /api/apps/{{id}}/launch");
    println!("    POST   /api/apps/{{id}}/stop");
    println!("    GET    /api/sandbox/{{id}}/verify");
    println!("    GET    /api/sandbox/{{id}}/disk");
    println!("    POST   /api/store/upload          ← NEW: folder/zip/git");
    println!();

    loop {
        match server.recv() {
            Ok(mut request) => {
                let response = route(&mut request, &mgr);
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
