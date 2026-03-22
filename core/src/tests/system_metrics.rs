use crate::system_metrics::{select_disk_snapshot, snapshot_resource_usage, DiskSnapshot};
use crate::tests::temp_base;
use std::path::PathBuf;

#[test]
fn select_disk_snapshot_prefers_longest_mount_prefix() {
    let target_path = if cfg!(windows) {
        PathBuf::from(r"C:\Users\dila\Downloads\ai-launcher")
    } else {
        PathBuf::from("/home/dila/projects/ai-launcher")
    };

    let disks = if cfg!(windows) {
        vec![
            DiskSnapshot {
                mount_point: PathBuf::from(r"C:\"),
                total_bytes: 100,
                available_bytes: 50,
            },
            DiskSnapshot {
                mount_point: PathBuf::from(r"C:\Users"),
                total_bytes: 100,
                available_bytes: 50,
            },
        ]
    } else {
        vec![
            DiskSnapshot {
                mount_point: PathBuf::from("/"),
                total_bytes: 100,
                available_bytes: 50,
            },
            DiskSnapshot {
                mount_point: PathBuf::from("/home"),
                total_bytes: 100,
                available_bytes: 50,
            },
        ]
    };

    let selected = select_disk_snapshot(&target_path, &disks).expect("expected a matching disk");

    assert_eq!(selected.mount_point, disks[1].mount_point);
}

#[test]
fn snapshot_resource_usage_reports_totals_and_percentages() {
    let temp_dir = temp_base();
    let usage = snapshot_resource_usage(temp_dir.path()).expect("resource usage should load");

    assert!(usage.memory_total_bytes > 0);
    assert!(usage.memory_used_bytes <= usage.memory_total_bytes);
    assert!(usage.disk_total_bytes > 0);
    assert!(usage.disk_used_bytes <= usage.disk_total_bytes);
    assert!(usage.memory_percent <= 100);
    assert!(usage.disk_percent <= 100);
    assert!(!usage.disk_mount_point.is_empty());
}
