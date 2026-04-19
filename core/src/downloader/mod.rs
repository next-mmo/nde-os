//! Download Center — provider-pluggable media downloader.
//!
//! Resolves a source URL into a playlist of downloadable items, then streams
//! each item to disk. Supports HLS (`.m3u8`) sources via ffmpeg remux and
//! direct HTTP downloads via reqwest.
//!
//! Architecture:
//!  - [`provider::MediaProvider`] — trait implemented per site (e.g. quickplay)
//!  - [`engine::DownloadEngine`] — queue + worker pool + progress events
//!  - [`jobs::JobStore`] — on-disk job persistence in `{data_dir}/downloads/jobs.json`

pub mod engine;
pub mod jobs;
pub mod provider;
pub mod providers;

pub use engine::{DownloadEngine, ProgressEvent};
pub use jobs::{DownloadItem, ItemStatus, Job, JobStatus, JobStore};
pub use provider::{MediaProvider, Playlist, PlaylistItem, StreamInfo, StreamKind};
