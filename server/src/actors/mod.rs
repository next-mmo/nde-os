/// Actor HTTP handlers — REST API for actor CRUD, runs, and Apify export.
use std::path::Path;

use ai_launcher_core::actor::manifest::ActorManager;
use ai_launcher_core::actor::runner::{ActorRun, ActorRunner};
use ai_launcher_core::actor::storage::RunStorage;
use ai_launcher_core::actor::template::ActorTemplate;
use tiny_http::Request;

use crate::response::*;

// ─── List Actors ───────────────────────────────────────────────────

pub fn list_actors(data_dir: &Path) -> HttpResponse {
    let mgr = ActorManager::new(data_dir);
    match mgr.list_actors() {
        Ok(actors) => ok("actors", serde_json::json!(actors)),
        Err(e) => err(500, &format!("Failed to list actors: {e}")),
    }
}

// ─── Get Actor ─────────────────────────────────────────────────────

pub fn get_actor(id: &str, data_dir: &Path) -> HttpResponse {
    let mgr = ActorManager::new(data_dir);
    match mgr.get_actor(id) {
        Ok(actor) => ok("actor", serde_json::json!(actor)),
        Err(e) => err(404, &format!("Actor not found: {e}")),
    }
}

// ─── Delete Actor ──────────────────────────────────────────────────

pub fn delete_actor(id: &str, data_dir: &Path) -> HttpResponse {
    let mgr = ActorManager::new(data_dir);
    match mgr.delete_actor(id) {
        Ok(()) => ok_msg(&format!("Actor '{}' deleted", id)),
        Err(e) => err(500, &format!("Failed to delete actor: {e}")),
    }
}

// ─── Scaffold Actor ────────────────────────────────────────────────

pub fn scaffold_actor(req: &mut Request, data_dir: &Path) -> HttpResponse {
    let body = match read_body(req) {
        Ok(b) => b,
        Err(e) => return err(400, &format!("Failed to read request body: {e}")),
    };

    let params: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => return err(400, &format!("Invalid JSON: {e}")),
    };

    let template_id = match params["template"].as_str() {
        Some(t) => t,
        None => return err(400, "Missing 'template' field"),
    };

    let actor_name = match params["name"].as_str() {
        Some(n) => n,
        None => return err(400, "Missing 'name' field"),
    };

    let template = match ActorTemplate::from_str(template_id) {
        Ok(t) => t,
        Err(e) => return err(400, &format!("Invalid template: {e}")),
    };

    let actor_id = slugify(actor_name);
    let actor_dir = data_dir.join("actors").join(&actor_id);

    if actor_dir.exists() {
        return err(409, &format!("Actor '{}' already exists", actor_id));
    }

    match template.scaffold(&actor_dir, actor_name) {
        Ok(()) => ok(
            &format!("Actor '{}' scaffolded from template '{}'", actor_id, template_id),
            serde_json::json!({
                "id": actor_id,
                "name": actor_name,
                "template": template_id,
                "path": actor_dir.display().to_string(),
            }),
        ),
        Err(e) => err(500, &format!("Failed to scaffold actor: {e}")),
    }
}

// ─── List Templates ────────────────────────────────────────────────

pub fn list_templates() -> HttpResponse {
    let templates = ActorTemplate::all();
    ok("templates", serde_json::json!(templates))
}

// ─── Run Actor ─────────────────────────────────────────────────────

pub fn run_actor(
    id: &str,
    req: &mut Request,
    rt: &tokio::runtime::Runtime,
    runner: &tokio::sync::Mutex<ActorRunner>,
) -> HttpResponse {
    let body = match read_body(req) {
        Ok(b) => b,
        Err(e) => return err(400, &format!("Failed to read request body: {e}")),
    };

    let input: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => serde_json::json!({}),
    };

    let result = rt.block_on(async {
        let runner = runner.lock().await;
        runner.run_actor(id, input).await
    });

    match result {
        Ok(run) => ok(
            &format!("Actor '{}' started", id),
            serde_json::json!(run),
        ),
        Err(e) => err(500, &format!("Failed to start actor: {e}")),
    }
}

// ─── Stop Actor ────────────────────────────────────────────────────

pub fn stop_actor(
    run_id: &str,
    rt: &tokio::runtime::Runtime,
    runner: &tokio::sync::Mutex<ActorRunner>,
) -> HttpResponse {
    let result = rt.block_on(async {
        let runner = runner.lock().await;
        runner.stop_actor(run_id).await
    });

    match result {
        Ok(()) => ok_msg(&format!("Actor run '{}' stopped", run_id)),
        Err(e) => err(500, &format!("Failed to stop actor: {e}")),
    }
}

// ─── List Runs ─────────────────────────────────────────────────────

pub fn list_runs(
    actor_id: &str,
    rt: &tokio::runtime::Runtime,
    runner: &tokio::sync::Mutex<ActorRunner>,
) -> HttpResponse {
    let result = rt.block_on(async {
        let runner = runner.lock().await;
        runner.list_runs(actor_id)
    });

    match result {
        Ok(runs) => ok("runs", serde_json::json!(runs)),
        Err(e) => err(500, &format!("Failed to list runs: {e}")),
    }
}

// ─── Get Run ───────────────────────────────────────────────────────

pub fn get_run(
    actor_id: &str,
    run_id: &str,
    rt: &tokio::runtime::Runtime,
    runner: &tokio::sync::Mutex<ActorRunner>,
) -> HttpResponse {
    let result = rt.block_on(async {
        let runner = runner.lock().await;
        runner.get_run(actor_id, run_id)
    });

    match result {
        Ok(run) => ok("run", serde_json::json!(run)),
        Err(e) => err(404, &format!("Run not found: {e}")),
    }
}

// ─── Get Run Dataset ───────────────────────────────────────────────

pub fn get_run_dataset(actor_id: &str, run_id: &str, data_dir: &Path) -> HttpResponse {
    let actors_dir = data_dir.join("actors");
    match RunStorage::open(&actors_dir, actor_id, run_id) {
        Ok(storage) => {
            match storage.dataset.export_json() {
                Ok(items) => ok("dataset", serde_json::json!({
                    "items": items,
                    "count": items.len(),
                })),
                Err(e) => err(500, &format!("Failed to read dataset: {e}")),
            }
        }
        Err(e) => err(404, &format!("Run not found: {e}")),
    }
}

// ─── Get Run Log ───────────────────────────────────────────────────

pub fn get_run_log(actor_id: &str, run_id: &str, data_dir: &Path) -> HttpResponse {
    let log_path = data_dir
        .join("actors")
        .join(actor_id)
        .join("runs")
        .join(run_id)
        .join("log.txt");

    match std::fs::read_to_string(&log_path) {
        Ok(log) => ok("log", serde_json::json!({
            "log": log,
            "lines": log.lines().count(),
        })),
        Err(_) => ok("log", serde_json::json!({
            "log": "",
            "lines": 0,
        })),
    }
}

// ─── Export Apify ──────────────────────────────────────────────────

pub fn export_apify(id: &str, data_dir: &Path) -> HttpResponse {
    let mgr = ActorManager::new(data_dir);
    match mgr.export_apify(id) {
        Ok(path) => ok(
            &format!("Apify-compatible files exported for actor '{}'", id),
            serde_json::json!({
                "actorId": id,
                "path": path.display().to_string(),
                "files": [
                    ".actor/actor.json",
                    ".actor/input_schema.json",
                    "Dockerfile",
                ],
            }),
        ),
        Err(e) => err(500, &format!("Failed to export Apify files: {e}")),
    }
}

// ─── Helpers ───────────────────────────────────────────────────────

fn read_body(req: &mut Request) -> Result<String, std::io::Error> {
    let mut body = String::new();
    req.as_reader().read_to_string(&mut body)?;
    Ok(body)
}

fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| match c {
            'a'..='z' | '0'..='9' | '-' => c,
            ' ' | '_' => '-',
            _ => '-',
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
