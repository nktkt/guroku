#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use guroku::linker::{populate_bin_dir, LinkedPackage};
use tempfile::TempDir;

/// Pre-create the inner pkg dir
/// `<node_modules>/.guroku/<name>@<version>/node_modules/<name>/<rel>`
/// and touch the file so the symlink target lands somewhere meaningful.
fn touch_inner_bin(node_modules: &Path, name: &str, version: &str, rel: &str) -> PathBuf {
    let rel = rel.trim_start_matches("./").trim_start_matches('/');
    let pkg_full = node_modules
        .join(".guroku")
        .join(format!("{name}@{version}"))
        .join("node_modules")
        .join(name)
        .join(rel);
    if let Some(parent) = pkg_full.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&pkg_full, b"#!/usr/bin/env node\n").unwrap();
    pkg_full
}

fn pkg(name: &str, version: &str, bin_entries: Vec<(&str, &str)>) -> LinkedPackage {
    LinkedPackage {
        name: name.to_string(),
        version: version.to_string(),
        source_dir: PathBuf::from("/unused-source"),
        dependencies: BTreeMap::new(),
        bin_entries: bin_entries
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    }
}

#[test]
fn creates_bin_symlink_for_direct_dep_with_string_bin() {
    let tmp = TempDir::new().unwrap();
    let node_modules = tmp.path().join("node_modules");
    touch_inner_bin(&node_modules, "widget", "1.0.0", "./bin/widget.js");

    let packages = vec![pkg("widget", "1.0.0", vec![("widget", "./bin/widget.js")])];
    populate_bin_dir(&packages, &["widget".to_string()], &node_modules).unwrap();

    let link = node_modules.join(".bin").join("widget");
    assert!(link.is_symlink(), "{} should be a symlink", link.display());
}

#[test]
fn does_not_create_bin_dir_when_no_direct_dep_has_bin() {
    let tmp = TempDir::new().unwrap();
    let node_modules = tmp.path().join("node_modules");
    std::fs::create_dir_all(&node_modules).unwrap();

    let packages = vec![pkg("widget", "1.0.0", vec![])];
    populate_bin_dir(&packages, &["widget".to_string()], &node_modules).unwrap();

    let bin = node_modules.join(".bin");
    assert!(
        !bin.exists(),
        "{} should not exist when no direct dep has bins",
        bin.display()
    );
}

#[test]
fn transitive_deps_with_bin_are_not_shimmed() {
    let tmp = TempDir::new().unwrap();
    let node_modules = tmp.path().join("node_modules");
    touch_inner_bin(&node_modules, "widget", "1.0.0", "./w.js");

    let packages = vec![pkg("widget", "1.0.0", vec![("widget", "./w.js")])];
    // direct_deps is empty: widget is purely transitive.
    populate_bin_dir(&packages, &[], &node_modules).unwrap();

    let link = node_modules.join(".bin").join("widget");
    assert!(
        !link.exists() && !link.is_symlink(),
        "{} should not exist for transitive deps",
        link.display()
    );
}

#[test]
fn bin_symlink_target_is_relative() {
    let tmp = TempDir::new().unwrap();
    let node_modules = tmp.path().join("node_modules");
    touch_inner_bin(&node_modules, "widget", "1.0.0", "./bin/widget.js");

    let packages = vec![pkg("widget", "1.0.0", vec![("widget", "./bin/widget.js")])];
    populate_bin_dir(&packages, &["widget".to_string()], &node_modules).unwrap();

    let link = node_modules.join(".bin").join("widget");
    let target = std::fs::read_link(&link).unwrap();
    let target_str = target.to_string_lossy();
    assert!(
        !target_str.starts_with('/'),
        "expected relative target, got {target_str}"
    );
    assert!(
        target_str.starts_with("..") || !target_str.contains('/') || target_str.starts_with('.'),
        "expected relative-looking target, got {target_str}"
    );
}

#[test]
fn multiple_bins_per_package_all_shimmed() {
    let tmp = TempDir::new().unwrap();
    let node_modules = tmp.path().join("node_modules");
    touch_inner_bin(&node_modules, "widget", "1.0.0", "./a.js");
    touch_inner_bin(&node_modules, "widget", "1.0.0", "./b.js");

    let packages = vec![pkg(
        "widget",
        "1.0.0",
        vec![("a", "./a.js"), ("b", "./b.js")],
    )];
    populate_bin_dir(&packages, &["widget".to_string()], &node_modules).unwrap();

    let a = node_modules.join(".bin").join("a");
    let b = node_modules.join(".bin").join("b");
    assert!(a.is_symlink(), "{} should be a symlink", a.display());
    assert!(b.is_symlink(), "{} should be a symlink", b.display());
}
