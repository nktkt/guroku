#![cfg(unix)]

use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn write_bin(dir: &Path, name: &str, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(dir).unwrap();
    let p = dir.join(name);
    std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
    let mut perms = std::fs::metadata(&p).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&p, perms).unwrap();
    p
}

#[test]
fn inherits_arbitrary_env_var() {
    // Use a unique var name to avoid collisions with parallel tests.
    let var = "GUROKU_TEST_VAR_INHERITS_ARBITRARY_8F3A";
    // SAFETY: mutates process-global env; name is unique to this test.
    std::env::set_var(var, "abc");

    let tmp = TempDir::new().unwrap();
    let cwd = tmp.path();
    let body = format!(r#"printf "%s" "${var}" > out.txt"#);
    guroku::scripts::run_in(cwd, "go", &body, &[]).unwrap();

    let out = std::fs::read_to_string(cwd.join("out.txt")).unwrap();
    assert_eq!(out, "abc");

    std::env::remove_var(var);
}

#[test]
fn path_extra_overrides_inherited_path_for_lookup() {
    let tmp = TempDir::new().unwrap();
    let cwd = tmp.path();
    let bin_dir = cwd.join("bin");
    write_bin(&bin_dir, "widget", "echo SHIM-OK");

    guroku::scripts::run_in(cwd, "go", "widget > out.txt", &[bin_dir.as_path()]).unwrap();

    let out = std::fs::read_to_string(cwd.join("out.txt")).unwrap();
    assert_eq!(out, "SHIM-OK\n");
}

#[test]
fn cwd_works_when_path_inherited_from_parent() {
    let tmp = TempDir::new().unwrap();
    let cwd = tmp.path();

    guroku::scripts::run_in(cwd, "go", "pwd > out.txt", &[]).unwrap();

    let out = std::fs::read_to_string(cwd.join("out.txt")).unwrap();
    let out_trim = out.trim();
    let out_canon = std::fs::canonicalize(Path::new(out_trim)).unwrap();
    let cwd_canon = std::fs::canonicalize(cwd).unwrap();
    assert_eq!(out_canon, cwd_canon);
}

#[test]
fn running_with_empty_env_path_extra_still_inherits_path() {
    let tmp = TempDir::new().unwrap();
    let cwd = tmp.path();

    guroku::scripts::run_in(cwd, "go", "command -v sh > out.txt", &[]).unwrap();

    let out = std::fs::read_to_string(cwd.join("out.txt")).unwrap();
    let out_trim = out.trim();
    assert!(
        !out_trim.is_empty(),
        "expected `command -v sh` to find sh via inherited PATH, got empty"
    );
    // Sanity: the resolved path should exist as a file.
    assert!(
        Path::new(out_trim).exists(),
        "resolved sh path does not exist: {out_trim:?}"
    );
}
