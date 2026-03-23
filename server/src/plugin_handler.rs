/// Plugin API handlers — CRUD for the plugin engine.
use ai_launcher_core::plugins::PluginEngine;
use std::io::Cursor;
use std::sync::Mutex;
use tiny_http::Response;

use crate::response::*;

/// GET /api/plugins — list all plugins.
pub fn list_plugins(engine: &Mutex<PluginEngine>) -> Response<Cursor<Vec<u8>>> {
    match engine.lock() {
        Ok(e) => ok("Plugin list", e.status()),
        Err(_) => err(500, "Plugin engine lock failed"),
    }
}

/// GET /api/plugins/{id} — get plugin details.
pub fn get_plugin(id: &str, engine: &Mutex<PluginEngine>) -> Response<Cursor<Vec<u8>>> {
    match engine.lock() {
        Ok(e) => match e.get(id) {
            Some(status) => ok(&format!("Plugin '{}'", id), status),
            None => err(404, &format!("Plugin '{}' not found", id)),
        },
        Err(_) => err(500, "Plugin engine lock failed"),
    }
}

/// POST /api/plugins/discover — scan for plugins.
pub fn discover_plugins(
    _rt: &tokio::runtime::Runtime,
    engine: &Mutex<PluginEngine>,
) -> Response<Cursor<Vec<u8>>> {
    match engine.lock() {
        Ok(mut e) => match e.discover() {
            Ok(manifests) => ok(
                &format!("Discovered {} plugin(s)", manifests.len()),
                manifests.iter().map(|m| &m.id).collect::<Vec<_>>(),
            ),
            Err(e) => err(500, &e.to_string()),
        },
        Err(_) => err(500, "Plugin engine lock failed"),
    }
}

/// POST /api/plugins/{id}/install — install a plugin.
pub fn install_plugin(
    id: &str,
    rt: &tokio::runtime::Runtime,
    engine: &Mutex<PluginEngine>,
) -> Response<Cursor<Vec<u8>>> {
    let mut e = match engine.lock() {
        Ok(e) => e,
        Err(_) => return err(500, "Plugin engine lock failed"),
    };

    match rt.block_on(e.install(id)) {
        Ok(()) => ok(&format!("Plugin '{}' installed", id), "installed"),
        Err(e) => err(500, &e.to_string()),
    }
}

/// POST /api/plugins/{id}/start — start a plugin.
pub fn start_plugin(
    id: &str,
    rt: &tokio::runtime::Runtime,
    engine: &Mutex<PluginEngine>,
) -> Response<Cursor<Vec<u8>>> {
    let mut e = match engine.lock() {
        Ok(e) => e,
        Err(_) => return err(500, "Plugin engine lock failed"),
    };

    match rt.block_on(e.start(id)) {
        Ok(()) => ok(&format!("Plugin '{}' started", id), "running"),
        Err(e) => err(500, &e.to_string()),
    }
}

/// POST /api/plugins/{id}/stop — stop a plugin.
pub fn stop_plugin(
    id: &str,
    rt: &tokio::runtime::Runtime,
    engine: &Mutex<PluginEngine>,
) -> Response<Cursor<Vec<u8>>> {
    let mut e = match engine.lock() {
        Ok(e) => e,
        Err(_) => return err(500, "Plugin engine lock failed"),
    };

    match rt.block_on(e.stop(id)) {
        Ok(()) => ok(&format!("Plugin '{}' stopped", id), "stopped"),
        Err(e) => err(500, &e.to_string()),
    }
}
