//! Download Center REST handlers.
//!
//! Routes:
//!   POST   /api/downloads/resolve        → playlist preview
//!   POST   /api/downloads                 → start a job
//!   GET    /api/downloads                 → list jobs
//!   GET    /api/downloads/{id}            → job detail (polled by UI)
//!   POST   /api/downloads/{id}/pause      → pause running job
//!   POST   /api/downloads/{id}/resume     → resume paused/failed job
//!   POST   /api/downloads/{id}/cancel     → cancel + mark cancelled
//!   DELETE /api/downloads/{id}            → remove from store (files kept)

use crate::response::*;
use ai_launcher_core::downloader::{
    providers, DownloadEngine, DownloadItem, ItemStatus, Job, JobStatus, JobStore,
};
use chrono::Utc;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Deserialize)]
pub struct ResolveRequest {
    pub url: String,
}

#[derive(Deserialize)]
pub struct StartRequest {
    pub url: String,
    /// Which item ids to download. `None` or empty → all items.
    #[serde(default)]
    pub item_ids: Option<Vec<String>>,
    /// Override output dir. Defaults to `{data_dir}/downloads/{provider}/{title}`.
    #[serde(default)]
    pub output_dir: Option<String>,
}

/// Sanitize a filename / directory segment: strip path separators and
/// control chars; trim trailing dots/spaces which break on Windows.
fn sanitize_segment(s: &str) -> String {
    let mut out: String = s
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    out = out.trim().trim_end_matches('.').to_string();
    if out.is_empty() {
        out.push('_');
    }
    if out.len() > 120 {
        out.truncate(120);
    }
    out
}

/// POST /api/downloads/resolve — fetch a playlist preview without downloading.
pub fn resolve(
    req: &mut tiny_http::Request,
    rt: &tokio::runtime::Runtime,
) -> HttpResponse {
    let payload: ResolveRequest = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let Some(provider) = providers::resolve_for_url(&payload.url) else {
        return err(400, "No provider matches this URL");
    };

    let result = rt.block_on(async { provider.resolve_playlist(&payload.url).await });
    match result {
        Ok(playlist) => ok("Playlist resolved", playlist),
        Err(e) => err(502, &format!("Resolve failed: {:#}", e)),
    }
}

/// POST /api/downloads — start a new download job.
pub fn start(
    req: &mut tiny_http::Request,
    data_dir: &Path,
    rt: &tokio::runtime::Runtime,
    engine: &Arc<DownloadEngine>,
) -> HttpResponse {
    let payload: StartRequest = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let Some(provider) = providers::resolve_for_url(&payload.url) else {
        return err(400, "No provider matches this URL");
    };

    let playlist = match rt.block_on(async { provider.resolve_playlist(&payload.url).await }) {
        Ok(p) => p,
        Err(e) => return err(502, &format!("Resolve failed: {:#}", e)),
    };

    let wanted: std::collections::HashSet<String> = payload
        .item_ids
        .unwrap_or_default()
        .into_iter()
        .collect();

    let selected: Vec<_> = if wanted.is_empty() {
        playlist.items.iter().cloned().collect()
    } else {
        playlist
            .items
            .iter()
            .filter(|it| wanted.contains(&it.id))
            .cloned()
            .collect()
    };
    if selected.is_empty() {
        return err(400, "No matching episodes to download");
    }

    let default_dir = data_dir
        .join("downloads")
        .join(sanitize_segment(provider.id()))
        .join(sanitize_segment(&playlist.title));
    let output_dir = payload
        .output_dir
        .map(PathBuf::from)
        .unwrap_or(default_dir);
    if let Err(e) = std::fs::create_dir_all(&output_dir) {
        return err(500, &format!("mkdir {}: {}", output_dir.display(), e));
    }

    let items: Vec<DownloadItem> = selected
        .into_iter()
        .map(|it| {
            let filename = format!("{:03}_{}.mp4", it.index, sanitize_segment(&it.title));
            DownloadItem {
                id: it.id,
                title: it.title,
                index: it.index,
                output_path: output_dir.join(filename),
                status: ItemStatus::Pending,
                progress: 0.0,
                bytes_downloaded: 0,
                bytes_total: 0,
                error: None,
            }
        })
        .collect();

    let job = Job {
        id: uuid::Uuid::new_v4().to_string(),
        provider: playlist.provider.clone(),
        source_url: playlist.source_url.clone(),
        playlist_id: playlist.playlist_id.clone(),
        title: playlist.title.clone(),
        cover: playlist.cover.clone(),
        output_dir,
        status: JobStatus::Queued,
        items,
        context: playlist.context.clone(),
        created_at: Utc::now().timestamp(),
        updated_at: Utc::now().timestamp(),
    };

    let job_id = job.id.clone();
    if let Err(e) = engine.store().insert(job.clone()) {
        return err(500, &format!("persist job: {}", e));
    }

    (**engine).clone().spawn(rt.handle(), job_id.clone());

    created("Job started", serde_json::json!({ "job_id": job_id, "job": job }))
}

/// GET /api/downloads — list all jobs.
pub fn list(engine: &Arc<DownloadEngine>) -> HttpResponse {
    let jobs = engine.store().list();
    ok(&format!("{} jobs", jobs.len()), jobs)
}

/// GET /api/downloads/{id}
pub fn get(id: &str, engine: &Arc<DownloadEngine>) -> HttpResponse {
    match engine.store().get(id) {
        Some(j) => ok("Job details", j),
        None => err(404, &format!("Job {} not found", id)),
    }
}

/// POST /api/downloads/{id}/pause
pub fn pause(id: &str, rt: &tokio::runtime::Runtime, engine: &Arc<DownloadEngine>) -> HttpResponse {
    let engine_cloned = engine.clone();
    let id = id.to_string();
    match rt.block_on(async move { engine_cloned.pause(&id).await }) {
        Ok(()) => ok_msg("Job paused"),
        Err(e) => err(500, &format!("Pause failed: {}", e)),
    }
}

/// POST /api/downloads/{id}/resume
pub fn resume(id: &str, rt: &tokio::runtime::Runtime, engine: &Arc<DownloadEngine>) -> HttpResponse {
    let store: &JobStore = engine.store();
    let Some(job) = store.get(id) else {
        return err(404, &format!("Job {} not found", id));
    };
    if matches!(job.status, JobStatus::Running) {
        return ok_msg("Job already running");
    }

    // Reset failed items back to pending so the engine retries them.
    if let Err(e) = store.update(id, |j| {
        for it in j.items.iter_mut() {
            if matches!(it.status, ItemStatus::Failed) {
                it.status = ItemStatus::Pending;
                it.error = None;
            }
        }
        j.status = JobStatus::Queued;
    }) {
        return err(500, &format!("update job: {}", e));
    }

    (**engine).clone().spawn(rt.handle(), id.to_string());
    ok_msg("Job resumed")
}

/// POST /api/downloads/{id}/cancel
pub fn cancel(id: &str, engine: &Arc<DownloadEngine>) -> HttpResponse {
    let Some(_job) = engine.store().get(id) else {
        return err(404, &format!("Job {} not found", id));
    };
    engine.cancel(id);
    if let Err(e) = engine.store().update(id, |j| {
        j.status = JobStatus::Cancelled;
        for it in j.items.iter_mut() {
            if matches!(it.status, ItemStatus::Downloading | ItemStatus::Pending) {
                it.status = ItemStatus::Skipped;
            }
        }
    }) {
        return err(500, &format!("update job: {}", e));
    }
    ok_msg("Job cancelled")
}

/// DELETE /api/downloads/{id}
pub fn delete(id: &str, engine: &Arc<DownloadEngine>) -> HttpResponse {
    engine.cancel(id);
    match engine.store().remove(id) {
        Ok(true) => ok_msg("Job removed"),
        Ok(false) => err(404, &format!("Job {} not found", id)),
        Err(e) => err(500, &format!("remove job: {}", e)),
    }
}

/// GET /api/downloads/providers — list available providers.
pub fn list_providers() -> HttpResponse {
    let providers: Vec<serde_json::Value> = providers::default_providers()
        .iter()
        .map(|p| {
            serde_json::json!({
                "id": p.id(),
                "name": p.display_name(),
            })
        })
        .collect();
    ok(&format!("{} providers", providers.len()), providers)
}
