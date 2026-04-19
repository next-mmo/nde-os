//! Download engine — resolves streams and pipes them to disk.
//!
//! - HLS (`.m3u8`) sources → `ffmpeg -c copy` remux to MP4.
//! - HTTP direct sources → streamed via reqwest with chunked writes.
//!
//! Progress is emitted on a broadcast channel so the server can fan out
//! SSE to any number of UI clients. Cancellation is cooperative via an
//! [`Arc<AtomicBool>`] checked between items and between ffmpeg progress
//! ticks.

use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::broadcast;

use crate::downloader::jobs::{DownloadItem, ItemStatus, Job, JobStatus, JobStore};
use crate::downloader::provider::{MediaProvider, StreamKind};
use crate::downloader::providers;
use crate::media::ffmpeg::FfmpegBins;

/// Progress/state event for one job. Emitted on the broadcast channel.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProgressEvent {
    JobStarted {
        job_id: String,
    },
    ItemStarted {
        job_id: String,
        item_id: String,
    },
    ItemProgress {
        job_id: String,
        item_id: String,
        bytes_downloaded: u64,
        bytes_total: u64,
        /// 0.0 – 100.0. `None` when duration is not yet known.
        percent: Option<f32>,
    },
    ItemCompleted {
        job_id: String,
        item_id: String,
        output_path: String,
    },
    ItemFailed {
        job_id: String,
        item_id: String,
        error: String,
    },
    JobCompleted {
        job_id: String,
    },
    JobFailed {
        job_id: String,
        error: String,
    },
    JobCancelled {
        job_id: String,
    },
}

/// Per-job cancel flag. Set to `true` to abort the running task.
type CancelHandle = Arc<AtomicBool>;

/// Shared engine state: persistent job store + live cancel flags + progress
/// broadcast. Cheap to clone.
#[derive(Clone)]
pub struct DownloadEngine {
    store: Arc<JobStore>,
    ffmpeg: Arc<FfmpegBins>,
    tx: broadcast::Sender<ProgressEvent>,
    cancels: Arc<Mutex<HashMap<String, CancelHandle>>>,
}

impl DownloadEngine {
    pub fn new(store: Arc<JobStore>, ffmpeg: FfmpegBins) -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self {
            store,
            ffmpeg: Arc::new(ffmpeg),
            tx,
            cancels: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn store(&self) -> &JobStore {
        &self.store
    }

    /// Subscribe to live progress events.
    pub fn subscribe(&self) -> broadcast::Receiver<ProgressEvent> {
        self.tx.subscribe()
    }

    /// Request cancellation of a running job. No-op if not running.
    pub fn cancel(&self, job_id: &str) {
        if let Ok(map) = self.cancels.lock() {
            if let Some(flag) = map.get(job_id) {
                flag.store(true, Ordering::SeqCst);
            }
        }
    }

    /// Mark job Paused — same effect as cancel, but status reflects intent.
    pub async fn pause(&self, job_id: &str) -> Result<()> {
        self.cancel(job_id);
        self.store.update(job_id, |j| {
            if j.status == JobStatus::Running {
                j.status = JobStatus::Paused;
            }
        })?;
        Ok(())
    }

    /// Spawn a tokio task that runs the job to completion (or cancellation).
    /// Only items with status Pending/Failed are processed.
    pub fn spawn(self, rt: &tokio::runtime::Handle, job_id: String) {
        rt.spawn(async move {
            if let Err(e) = self.run(&job_id).await {
                tracing::error!(job = %job_id, error = %e, "download job errored");
                let _ = self.tx.send(ProgressEvent::JobFailed {
                    job_id: job_id.clone(),
                    error: e.to_string(),
                });
                let _ = self.store.update(&job_id, |j| {
                    j.status = JobStatus::Failed;
                });
            }
        });
    }

    async fn run(&self, job_id: &str) -> Result<()> {
        let job = self
            .store
            .get(job_id)
            .ok_or_else(|| anyhow!("job not found: {}", job_id))?;
        let provider = providers::default_providers()
            .into_iter()
            .find(|p| p.id() == job.provider)
            .ok_or_else(|| anyhow!("unknown provider: {}", job.provider))?;

        let cancel = Arc::new(AtomicBool::new(false));
        self.cancels
            .lock()
            .map_err(|_| anyhow!("cancels lock poisoned"))?
            .insert(job_id.to_string(), cancel.clone());

        self.store.update(job_id, |j| j.status = JobStatus::Running)?;
        let _ = self.tx.send(ProgressEvent::JobStarted {
            job_id: job_id.to_string(),
        });

        std::fs::create_dir_all(&job.output_dir)
            .with_context(|| format!("mkdir {}", job.output_dir.display()))?;

        let pending_ids: Vec<String> = job
            .items
            .iter()
            .filter(|i| matches!(i.status, ItemStatus::Pending | ItemStatus::Failed))
            .map(|i| i.id.clone())
            .collect();

        for item_id in pending_ids {
            if cancel.load(Ordering::SeqCst) {
                let _ = self.tx.send(ProgressEvent::JobCancelled {
                    job_id: job_id.to_string(),
                });
                return Ok(());
            }

            // Re-read latest job state on each iteration (item paths may have changed).
            let current = self
                .store
                .get(job_id)
                .ok_or_else(|| anyhow!("job vanished mid-run"))?;
            let item = match current.items.iter().find(|i| i.id == item_id) {
                Some(it) => it.clone(),
                None => continue,
            };

            self.store.update(job_id, |j| {
                if let Some(it) = j.items.iter_mut().find(|i| i.id == item_id) {
                    it.status = ItemStatus::Downloading;
                    it.error = None;
                }
            })?;
            let _ = self.tx.send(ProgressEvent::ItemStarted {
                job_id: job_id.to_string(),
                item_id: item_id.clone(),
            });

            let result = self
                .download_item(&current, &item, provider.as_ref(), cancel.clone())
                .await;

            match result {
                Ok(()) => {
                    self.store.update(job_id, |j| {
                        if let Some(it) = j.items.iter_mut().find(|i| i.id == item_id) {
                            it.status = ItemStatus::Completed;
                            it.progress = 100.0;
                        }
                    })?;
                    let _ = self.tx.send(ProgressEvent::ItemCompleted {
                        job_id: job_id.to_string(),
                        item_id: item_id.clone(),
                        output_path: item.output_path.display().to_string(),
                    });
                }
                Err(e) if cancel.load(Ordering::SeqCst) => {
                    // Cancellation is not a failure.
                    tracing::info!(job = %job_id, item = %item_id, "item cancelled: {}", e);
                    let _ = self.tx.send(ProgressEvent::JobCancelled {
                        job_id: job_id.to_string(),
                    });
                    return Ok(());
                }
                Err(e) => {
                    let msg = format!("{:#}", e);
                    tracing::warn!(job = %job_id, item = %item_id, "item failed: {}", msg);
                    self.store.update(job_id, |j| {
                        if let Some(it) = j.items.iter_mut().find(|i| i.id == item_id) {
                            it.status = ItemStatus::Failed;
                            it.error = Some(msg.clone());
                        }
                    })?;
                    let _ = self.tx.send(ProgressEvent::ItemFailed {
                        job_id: job_id.to_string(),
                        item_id: item_id.clone(),
                        error: msg,
                    });
                }
            }
        }

        // Final status roll-up.
        let final_job = self
            .store
            .get(job_id)
            .ok_or_else(|| anyhow!("job vanished before finalize"))?;
        let any_failed = final_job
            .items
            .iter()
            .any(|i| matches!(i.status, ItemStatus::Failed));
        let all_done = final_job
            .items
            .iter()
            .all(|i| matches!(i.status, ItemStatus::Completed | ItemStatus::Skipped));
        let final_status = if all_done {
            JobStatus::Completed
        } else if any_failed {
            JobStatus::Failed
        } else {
            JobStatus::Queued
        };
        self.store.update(job_id, |j| j.status = final_status)?;

        self.cancels
            .lock()
            .map_err(|_| anyhow!("cancels lock poisoned"))?
            .remove(job_id);

        match final_status {
            JobStatus::Completed => {
                let _ = self.tx.send(ProgressEvent::JobCompleted {
                    job_id: job_id.to_string(),
                });
            }
            JobStatus::Failed => {
                let _ = self.tx.send(ProgressEvent::JobFailed {
                    job_id: job_id.to_string(),
                    error: "one or more items failed".to_string(),
                });
            }
            _ => {}
        }

        Ok(())
    }

    async fn download_item(
        &self,
        job: &Job,
        item: &DownloadItem,
        provider: &dyn MediaProvider,
        cancel: CancelHandle,
    ) -> Result<()> {
        // Rebuild a Playlist-shaped context (cheap, avoids re-fetching /detail).
        let playlist = crate::downloader::provider::Playlist {
            provider: job.provider.clone(),
            source_url: job.source_url.clone(),
            playlist_id: job.playlist_id.clone(),
            title: job.title.clone(),
            cover: job.cover.clone(),
            synopsis: None,
            items: vec![],
            context: job.context.clone(),
        };

        let stream = provider
            .resolve_stream(&playlist, &item.id)
            .await
            .with_context(|| format!("resolve stream for item {}", item.id))?;

        if let Some(parent) = item.output_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("mkdir {}", parent.display()))?;
        }

        match stream.kind {
            StreamKind::Hls => {
                self.download_hls(
                    job.id.clone(),
                    item.id.clone(),
                    &stream.url,
                    &stream.headers,
                    &item.output_path,
                    cancel,
                )
                .await
            }
            StreamKind::Http => {
                self.download_http(
                    job.id.clone(),
                    item.id.clone(),
                    &stream.url,
                    &stream.headers,
                    &item.output_path,
                    cancel,
                )
                .await
            }
        }
    }

    async fn download_hls(
        &self,
        job_id: String,
        item_id: String,
        url: &str,
        headers: &[(String, String)],
        output_path: &PathBuf,
        cancel: CancelHandle,
    ) -> Result<()> {
        // Build -headers payload (CRLF-separated per ffmpeg docs).
        let mut hdr_str = String::new();
        for (k, v) in headers {
            hdr_str.push_str(k);
            hdr_str.push_str(": ");
            hdr_str.push_str(v);
            hdr_str.push_str("\r\n");
        }

        let mut cmd = TokioCommand::new(&self.ffmpeg.ffmpeg);
        cmd.arg("-y");
        if !hdr_str.is_empty() {
            cmd.arg("-headers").arg(&hdr_str);
        }
        cmd.arg("-i")
            .arg(url)
            .arg("-c")
            .arg("copy")
            .arg("-bsf:a")
            .arg("aac_adtstoasc")
            .arg("-progress")
            .arg("pipe:2")
            .arg("-nostats")
            .arg("-loglevel")
            .arg("error")
            .arg(output_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let mut child = cmd.spawn().with_context(|| "spawn ffmpeg")?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow!("ffmpeg stderr unavailable"))?;
        let mut reader = BufReader::new(stderr).lines();

        let mut duration_us: Option<u64> = None;
        let mut last_tick = std::time::Instant::now();
        let mut last_emit_bytes: u64 = 0;
        let mut last_emit_time: u64 = 0;

        loop {
            if cancel.load(Ordering::SeqCst) {
                let _ = child.start_kill();
                let _ = child.wait().await;
                let _ = std::fs::remove_file(output_path);
                return Err(anyhow!("cancelled"));
            }

            tokio::select! {
                line = reader.next_line() => {
                    match line {
                        Ok(Some(line)) => {
                            // ffmpeg -progress key=value lines.
                            if let Some((k, v)) = line.split_once('=') {
                                let k = k.trim();
                                let v = v.trim();
                                match k {
                                    "total_size" => {
                                        last_emit_bytes = v.parse().unwrap_or(0);
                                    }
                                    "out_time_us" | "out_time_ms" => {
                                        // -progress reports out_time_us (microseconds) in modern ffmpeg;
                                        // older builds label it out_time_ms but the unit is still microseconds.
                                        last_emit_time = v.parse().unwrap_or(0);
                                    }
                                    "progress" => {
                                        // Emit at end of each progress block ("continue" or "end").
                                        if last_tick.elapsed().as_millis() > 400 || v == "end" {
                                            let percent = duration_us
                                                .filter(|d| *d > 0)
                                                .map(|d| ((last_emit_time as f64 / d as f64) * 100.0).min(100.0) as f32);
                                            let _ = self.tx.send(ProgressEvent::ItemProgress {
                                                job_id: job_id.clone(),
                                                item_id: item_id.clone(),
                                                bytes_downloaded: last_emit_bytes,
                                                bytes_total: 0,
                                                percent,
                                            });
                                            // Throttle disk writes: update store at most every ~1s.
                                            if last_tick.elapsed().as_millis() > 1000 {
                                                let _ = self.store.update(&job_id, |j| {
                                                    if let Some(it) = j.items.iter_mut().find(|i| i.id == item_id) {
                                                        it.bytes_downloaded = last_emit_bytes;
                                                        if let Some(p) = percent { it.progress = p; }
                                                    }
                                                });
                                            }
                                            last_tick = std::time::Instant::now();
                                        }
                                    }
                                    _ => {}
                                }
                            } else if let Some(idx) = line.find("Duration:") {
                                // Parse "Duration: HH:MM:SS.xx" from stderr when ffmpeg prints it
                                // under -loglevel warning/info. Under -loglevel error we won't see
                                // it, but leaving the parse in place is harmless.
                                if let Some(dur) = parse_duration(&line[idx + "Duration:".len()..]) {
                                    duration_us = Some(dur);
                                }
                            }
                        }
                        Ok(None) => break,
                        Err(e) => return Err(anyhow!("ffmpeg stderr read: {}", e)),
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(250)) => {
                    // wake periodically to check cancel flag
                }
            }
        }

        let status = child.wait().await.with_context(|| "ffmpeg wait")?;
        if !status.success() {
            let code = status.code().unwrap_or(-1);
            let _ = std::fs::remove_file(output_path);
            return Err(anyhow!("ffmpeg exited with status {}", code));
        }
        Ok(())
    }

    async fn download_http(
        &self,
        job_id: String,
        item_id: String,
        url: &str,
        headers: &[(String, String)],
        output_path: &PathBuf,
        cancel: CancelHandle,
    ) -> Result<()> {
        use futures::StreamExt;
        use tokio::io::AsyncWriteExt;

        let client = reqwest::Client::new();
        let mut req = client.get(url);
        for (k, v) in headers {
            req = req.header(k, v);
        }
        let resp = req
            .send()
            .await
            .with_context(|| format!("GET {}", url))?
            .error_for_status()
            .with_context(|| format!("upstream error for {}", url))?;
        let bytes_total = resp.content_length().unwrap_or(0);

        let mut file = tokio::fs::File::create(output_path)
            .await
            .with_context(|| format!("create {}", output_path.display()))?;
        let mut stream = resp.bytes_stream();
        let mut bytes_downloaded: u64 = 0;
        let mut last_tick = std::time::Instant::now();

        while let Some(chunk) = stream.next().await {
            if cancel.load(Ordering::SeqCst) {
                drop(file);
                let _ = std::fs::remove_file(output_path);
                return Err(anyhow!("cancelled"));
            }
            let chunk = chunk.with_context(|| "read body chunk")?;
            file.write_all(&chunk).await.with_context(|| "write chunk")?;
            bytes_downloaded += chunk.len() as u64;

            if last_tick.elapsed().as_millis() > 400 {
                let percent = if bytes_total > 0 {
                    Some(((bytes_downloaded as f64 / bytes_total as f64) * 100.0) as f32)
                } else {
                    None
                };
                let _ = self.tx.send(ProgressEvent::ItemProgress {
                    job_id: job_id.clone(),
                    item_id: item_id.clone(),
                    bytes_downloaded,
                    bytes_total,
                    percent,
                });
                last_tick = std::time::Instant::now();
            }
        }
        file.flush().await.ok();
        let _ = self.store.update(&job_id, |j| {
            if let Some(it) = j.items.iter_mut().find(|i| i.id == item_id) {
                it.bytes_downloaded = bytes_downloaded;
                it.bytes_total = bytes_total;
                it.progress = 100.0;
            }
        });
        Ok(())
    }
}

/// Parse "HH:MM:SS.xx" into microseconds. Returns None on parse failure.
fn parse_duration(s: &str) -> Option<u64> {
    let s = s.trim();
    let end = s.find(|c: char| c == ',').unwrap_or(s.len());
    let ts = &s[..end].trim();
    let mut parts = ts.split(':');
    let h: u64 = parts.next()?.trim().parse().ok()?;
    let m: u64 = parts.next()?.trim().parse().ok()?;
    let sec_str = parts.next()?.trim();
    let secs: f64 = sec_str.parse().ok()?;
    let total = h as f64 * 3600.0 + m as f64 * 60.0 + secs;
    Some((total * 1_000_000.0) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ffmpeg_duration() {
        let d = parse_duration(" 00:00:30.50, start: 0.000, bitrate: 1024 kb/s").unwrap();
        assert_eq!(d, 30_500_000);
        let d = parse_duration("01:00:00.00").unwrap();
        assert_eq!(d, 3_600_000_000);
    }

    #[test]
    fn rejects_malformed_duration() {
        assert!(parse_duration("nope").is_none());
        assert!(parse_duration("12:34").is_none());
    }
}
