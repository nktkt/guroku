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
fn bin_dir_lands_first_on_path() {
    let tmp = TempDir::new().unwrap();
    let cwd = tmp.path();
    let bin_dir = cwd.join("bin");
    write_bin(&bin_dir, "widget", "echo HELLO");

    guroku::scripts::run_in(cwd, "go", "widget > out.txt", &[bin_dir.as_path()]).unwrap();

    let out = std::fs::read_to_string(cwd.join("out.txt")).unwrap();
    assert_eq!(out, "HELLO\n");
}

#[test]
fn existing_path_still_present() {
    let tmp = TempDir::new().unwrap();
    let cwd = tmp.path();
    let bin_dir = cwd.join("bin");
    let widget_path = write_bin(&bin_dir, "widget", "echo HELLO");

    let body = r#"which widget > w.txt; echo "---"; ls / > /dev/null; pwd > p.txt"#;
    guroku::scripts::run_in(cwd, "go", body, &[bin_dir.as_path()]).unwrap();

    let w = std::fs::read_to_string(cwd.join("w.txt")).unwrap();
    let w_trim = w.trim();
    let expected = widget_path.to_string_lossy().to_string();
    // On macOS, /tmp may resolve via /private/tmp; compare canonicalized forms.
    let w_canon = std::fs::canonicalize(Path::new(w_trim)).unwrap();
    let expected_canon = std::fs::canonicalize(&widget_path).unwrap();
    assert_eq!(
        w_canon, expected_canon,
        "which widget returned {w_trim:?}, expected {expected:?}"
    );
    assert!(cwd.join("p.txt").exists());
}

#[test]
fn prepended_not_appended() {
    let tmp = TempDir::new().unwrap();
    let cwd = tmp.path();
    let bin_dir = cwd.join("bin");
    let name = "guroku-test-widget-xyz";
    write_bin(&bin_dir, name, "echo FROM-BIN");

    let body = format!("{name} > out.txt");
    guroku::scripts::run_in(cwd, "go", &body, &[bin_dir.as_path()]).unwrap();

    let out = std::fs::read_to_string(cwd.join("out.txt")).unwrap();
    assert_eq!(out, "FROM-BIN\n");
}

#[test]
fn multiple_extras_prepended_in_order() {
    let tmp = TempDir::new().unwrap();
    let cwd = tmp.path();
    let bin1 = cwd.join("bin1");
    let bin2 = cwd.join("bin2");
    let foo1 = write_bin(&bin1, "foo", "echo ONE");
    let _foo2 = write_bin(&bin2, "foo", "echo TWO");

    let body = "foo > out.txt; which foo > w.txt";
    guroku::scripts::run_in(cwd, "go", body, &[bin1.as_path(), bin2.as_path()]).unwrap();

    let out = std::fs::read_to_string(cwd.join("out.txt")).unwrap();
    assert_eq!(out, "ONE\n");

    let w = std::fs::read_to_string(cwd.join("w.txt")).unwrap();
    let w_trim = w.trim();
    let w_canon = std::fs::canonicalize(Path::new(w_trim)).unwrap();
    let foo1_canon = std::fs::canonicalize(&foo1).unwrap();
    assert_eq!(w_canon, foo1_canon, "first extras entry should win");
}
