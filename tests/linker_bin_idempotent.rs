//! Idempotency / replacement tests for `populate_bin_dir`.
//!
//! `populate_bin_dir` writes `node_modules/.bin/<name>` symlinks. It must be
//! safe to call repeatedly and must replace whatever already lives at the
//! symlink path (regular file, dir, stale symlink) with a fresh symlink.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use guroku::linker::{populate_bin_dir, LinkedPackage};
use tempfile::TempDir;

/// Build a fake CAS source dir with a `package.json` and a bin script `w.js`.
fn fake_widget_src(root: &Path, tag: &str) -> PathBuf {
    let p = root.join(format!("src-{tag}"));
    std::fs::create_dir_all(&p).unwrap();
    std::fs::write(p.join("package.json"), br#"{"name":"widget"}"#).unwrap();
    std::fs::write(p.join("w.js"), b"#!/usr/bin/env node\nconsole.log('w');\n").unwrap();
    p
}

fn widget_pkg(version: &str, source_dir: PathBuf) -> LinkedPackage {
    LinkedPackage {
        name: "widget".to_string(),
        version: version.to_string(),
        source_dir,
        dependencies: BTreeMap::new(),
        bin_entries: vec![("widget".to_string(), "./w.js".to_string())],
    }
}

/// Pre-create the file `populate_bin_dir`'s symlink will end up pointing at,
/// so the symlink resolves successfully.
fn precreate_target(node_modules: &Path, version: &str) {
    let dir = node_modules
        .join(".guroku")
        .join(format!("widget@{version}"))
        .join("node_modules")
        .join("widget");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("w.js"), b"#!/usr/bin/env node\n").unwrap();
}

fn bin_path(node_modules: &Path) -> PathBuf {
    node_modules.join(".bin").join("widget")
}

fn expected_target(node_modules: &Path, version: &str) -> PathBuf {
    std::fs::canonicalize(
        node_modules
            .join(".guroku")
            .join(format!("widget@{version}"))
            .join("node_modules")
            .join("widget")
            .join("w.js"),
    )
    .unwrap()
}

#[test]
fn running_twice_succeeds() {
    let tmp = TempDir::new().unwrap();
    let node_modules = tmp.path().join("node_modules");
    let pkgs = vec![widget_pkg("1.0.0", fake_widget_src(tmp.path(), "v1"))];
    let direct = vec!["widget".to_string()];
    precreate_target(&node_modules, "1.0.0");

    populate_bin_dir(&pkgs, &direct, &node_modules).expect("first call");
    populate_bin_dir(&pkgs, &direct, &node_modules).expect("second call");

    let link = bin_path(&node_modules);
    assert!(link.is_symlink(), "{} should be a symlink", link.display());
    assert_eq!(
        std::fs::canonicalize(&link).unwrap(),
        expected_target(&node_modules, "1.0.0"),
    );
}

#[test]
fn replaces_existing_dir_at_bin_path() {
    let tmp = TempDir::new().unwrap();
    let node_modules = tmp.path().join("node_modules");
    let pkgs = vec![widget_pkg("1.0.0", fake_widget_src(tmp.path(), "v1"))];
    let direct = vec!["widget".to_string()];
    precreate_target(&node_modules, "1.0.0");

    let link = bin_path(&node_modules);
    std::fs::create_dir_all(&link).unwrap();
    std::fs::write(link.join("stray.txt"), b"junk").unwrap();
    assert!(link.is_dir() && !link.is_symlink());

    populate_bin_dir(&pkgs, &direct, &node_modules).expect("populate");

    assert!(link.is_symlink(), "{} should be a symlink", link.display());
    assert!(!link.is_dir() || std::fs::read_link(&link).is_ok());
}

#[test]
fn replaces_existing_file_at_bin_path() {
    let tmp = TempDir::new().unwrap();
    let node_modules = tmp.path().join("node_modules");
    let pkgs = vec![widget_pkg("1.0.0", fake_widget_src(tmp.path(), "v1"))];
    let direct = vec!["widget".to_string()];
    precreate_target(&node_modules, "1.0.0");

    let link = bin_path(&node_modules);
    std::fs::create_dir_all(link.parent().unwrap()).unwrap();
    std::fs::write(&link, b"stale shim").unwrap();
    assert!(link.is_file() && !link.is_symlink());

    populate_bin_dir(&pkgs, &direct, &node_modules).expect("populate");

    assert!(link.is_symlink(), "{} should be a symlink", link.display());
}

#[test]
fn replaces_old_symlink_at_bin_path() {
    let tmp = TempDir::new().unwrap();
    let node_modules = tmp.path().join("node_modules");
    let pkgs = vec![widget_pkg("1.0.0", fake_widget_src(tmp.path(), "v1"))];
    let direct = vec!["widget".to_string()];
    precreate_target(&node_modules, "1.0.0");

    let irrelevant = tmp.path().join("elsewhere.js");
    std::fs::write(&irrelevant, b"unrelated").unwrap();
    let link = bin_path(&node_modules);
    std::fs::create_dir_all(link.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&irrelevant, &link).unwrap();
    assert!(link.is_symlink());

    populate_bin_dir(&pkgs, &direct, &node_modules).expect("populate");

    assert!(link.is_symlink(), "{} should be a symlink", link.display());
    assert_eq!(
        std::fs::canonicalize(&link).unwrap(),
        expected_target(&node_modules, "1.0.0"),
    );
}

#[test]
fn pkg_renamed_replaces_old_bin_target() {
    let tmp = TempDir::new().unwrap();
    let node_modules = tmp.path().join("node_modules");
    let direct = vec!["widget".to_string()];

    let pkgs_v1 = vec![widget_pkg("1.0.0", fake_widget_src(tmp.path(), "v1"))];
    precreate_target(&node_modules, "1.0.0");
    populate_bin_dir(&pkgs_v1, &direct, &node_modules).expect("v1 populate");

    let pkgs_v2 = vec![widget_pkg("2.0.0", fake_widget_src(tmp.path(), "v2"))];
    precreate_target(&node_modules, "2.0.0");
    populate_bin_dir(&pkgs_v2, &direct, &node_modules).expect("v2 populate");

    let link = bin_path(&node_modules);
    assert!(link.is_symlink(), "{} should be a symlink", link.display());
    let target = std::fs::read_link(&link).unwrap();
    let target_str = target.to_string_lossy();
    assert!(
        target_str.contains("widget@2.0.0"),
        "symlink target {target_str:?} should mention widget@2.0.0",
    );
}
