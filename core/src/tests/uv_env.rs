use super::temp_base;
use crate::uv_env::UvEnv;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn write_fake_uv_binary(dir: &Path, script: &str) -> PathBuf {
    let path = if cfg!(windows) {
        dir.join("uv.cmd")
    } else {
        dir.join("uv")
    };

    fs::write(&path, script).unwrap();

    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o700);
        fs::set_permissions(&path, perms).unwrap();
    }

    path
}

#[test]
fn uv_version_uses_global_flag_instead_of_project_subcommand() {
    let tmp = temp_base();
    let workspace = tmp.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();

    let script = if cfg!(windows) {
        "@echo off\r\nif \"%~1\"==\"--version\" (\r\n  echo uv 9.9.9-test\r\n  exit /b 0\r\n)\r\nif \"%~1\"==\"version\" (\r\n  exit /b 1\r\n)\r\nexit /b 1\r\n"
    } else {
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  printf 'uv 9.9.9-test\\n'\n  exit 0\nfi\nif [ \"$1\" = \"version\" ]; then\n  exit 1\nfi\nexit 1\n"
    };

    let uv_bin = write_fake_uv_binary(tmp.path(), script);
    let uv = UvEnv::new(&uv_bin, &workspace, "3");

    assert_eq!(uv.uv_version().as_deref(), Some("uv 9.9.9-test"));
}

#[test]
fn uv_version_rejects_empty_output() {
    let tmp = temp_base();
    let workspace = tmp.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();

    let script = if cfg!(windows) {
        "@echo off\r\nif \"%~1\"==\"--version\" exit /b 0\r\nexit /b 1\r\n"
    } else {
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  exit 0\nfi\nexit 1\n"
    };

    let uv_bin = write_fake_uv_binary(tmp.path(), script);
    let uv = UvEnv::new(&uv_bin, &workspace, "3");

    assert_eq!(uv.uv_version(), None);
}
