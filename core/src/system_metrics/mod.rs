use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use sysinfo::{Disks, System};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceUsage {
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub memory_percent: u8,
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
    pub disk_percent: u8,
    pub disk_mount_point: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiskSnapshot {
    pub mount_point: PathBuf,
    pub total_bytes: u64,
    pub available_bytes: u64,
}

pub fn snapshot_resource_usage(base_dir: &Path) -> Result<ResourceUsage> {
    let mut system = System::new();
    system.refresh_memory();

    let memory_total_bytes = system.total_memory();
    let memory_used_bytes = system.used_memory();

    let resolved_base_dir = std::fs::canonicalize(base_dir).unwrap_or_else(|_| base_dir.to_path_buf());
    let disks = Disks::new_with_refreshed_list();
    let disk_snapshots: Vec<DiskSnapshot> = disks
        .iter()
        .map(|disk| DiskSnapshot {
            mount_point: disk.mount_point().to_path_buf(),
            total_bytes: disk.total_space(),
            available_bytes: disk.available_space(),
        })
        .filter(|disk| disk.total_bytes > 0)
        .collect();

    let disk = select_disk_snapshot(&resolved_base_dir, &disk_snapshots)
        .or_else(|| disk_snapshots.first())
        .ok_or_else(|| anyhow!("No disk metrics available for {}", resolved_base_dir.to_string_lossy()))?;

    let disk_used_bytes = disk.total_bytes.saturating_sub(disk.available_bytes);

    Ok(ResourceUsage {
        memory_used_bytes,
        memory_total_bytes,
        memory_percent: percent_used(memory_used_bytes, memory_total_bytes),
        disk_used_bytes,
        disk_total_bytes: disk.total_bytes,
        disk_percent: percent_used(disk_used_bytes, disk.total_bytes),
        disk_mount_point: disk.mount_point.to_string_lossy().to_string(),
    })
}

pub(crate) fn select_disk_snapshot<'a>(path: &Path, disks: &'a [DiskSnapshot]) -> Option<&'a DiskSnapshot> {
    disks
        .iter()
        .filter(|disk| path_has_prefix(path, &disk.mount_point))
        .max_by_key(|disk| disk.mount_point.components().count())
}

fn percent_used(used: u64, total: u64) -> u8 {
    if total == 0 {
        return 0;
    }

    let percent = ((used as f64 / total as f64) * 100.0).round();
    percent.clamp(0.0, 100.0) as u8
}

fn path_has_prefix(path: &Path, prefix: &Path) -> bool {
    let mut path_components = path.components();

    for prefix_component in prefix.components() {
        let Some(path_component) = path_components.next() else {
            return false;
        };

        if !os_str_eq(path_component.as_os_str(), prefix_component.as_os_str()) {
            return false;
        }
    }

    true
}

fn os_str_eq(left: &OsStr, right: &OsStr) -> bool {
    if cfg!(windows) {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    } else {
        left == right
    }
}
