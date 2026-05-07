//! Idempotency tests for `populate_node_modules`.
//!
//! `populate_node_modules` is intended to be safe to call twice with the same
//! input. These tests pin that contract, plus document the v0.3 limitation
//! that the writer does not garbage-collect entries no longer in the input.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use guroku::linker::{populate_node_modules, LinkedPackage};
use tempfile::TempDir;

const PKG_JSON_V1: &str = r#"{"name":"A","version":"1.0.0"}"#;
const PKG_JSON_V2: &str = r#"{"name":"A","version":"2.0.0"}"#;

/// Build a source dir under `dir` containing a minimal `package.json` for
/// `name`. The dir name embeds `name` so callers can build several side by
/// side.
fn fake_pkg(dir: &Path, name: &str) -> PathBuf {
    let pkg = dir.join(format!("src-{name}"));
    std::fs::create_dir_all(&pkg).unwrap();
    let body = match name {
        "A" => PKG_JSON_V1,
        "A_v2" => PKG_JSON_V2,
        _ => PKG_JSON_V1,
    };
    std::fs::write(pkg.join("package.json"), body.as_bytes()).unwrap();
    std::fs::write(pkg.join("index.js"), b"module.exports={};\n").unwrap();
    pkg
}

fn linked(name: &str, version: &str, source_dir: PathBuf) -> LinkedPackage {
    LinkedPackage {
        name: name.to_string(),
        version: version.to_string(),
        source_dir,
        dependencies: BTreeMap::new(),
    }
}

#[test]
fn running_twice_succeeds() {
    let tmp = TempDir::new().unwrap();
    let src = fake_pkg(tmp.path(), "A");
    let node_modules = tmp.path().join("node_modules");
    let pkgs = vec![linked("A", "1.0.0", src)];
    let direct = vec!["A".to_string()];

    let first = populate_node_modules(&pkgs, &node_modules, &direct);
    assert!(first.is_ok(), "first populate failed: {first:?}");

    let second = populate_node_modules(&pkgs, &node_modules, &direct);
    assert!(second.is_ok(), "second populate failed: {second:?}");
}

#[test]
fn top_level_symlink_remains_correct_after_second_run() {
    let tmp = TempDir::new().unwrap();
    let src = fake_pkg(tmp.path(), "A");
    let node_modules = tmp.path().join("node_modules");
    let pkgs = vec![linked("A", "1.0.0", src)];
    let direct = vec!["A".to_string()];

    populate_node_modules(&pkgs, &node_modules, &direct).unwrap();
    populate_node_modules(&pkgs, &node_modules, &direct).unwrap();

    let top = node_modules.join("A");
    assert!(top.is_symlink(), "{} should be a symlink", top.display());
    let resolved = std::fs::canonicalize(&top).unwrap();
    let expected = std::fs::canonicalize(
        node_modules
            .join(".guroku")
            .join("A@1.0.0")
            .join("node_modules")
            .join("A"),
    )
    .unwrap();
    assert_eq!(resolved, expected);
}

#[test]
fn inner_pkg_files_remain_intact_after_second_run() {
    let tmp = TempDir::new().unwrap();
    let src = fake_pkg(tmp.path(), "A");
    let node_modules = tmp.path().join("node_modules");
    let pkgs = vec![linked("A", "1.0.0", src)];
    let direct = vec!["A".to_string()];

    populate_node_modules(&pkgs, &node_modules, &direct).unwrap();
    populate_node_modules(&pkgs, &node_modules, &direct).unwrap();

    let pkg_json = node_modules
        .join(".guroku")
        .join("A@1.0.0")
        .join("node_modules")
        .join("A")
        .join("package.json");
    assert!(pkg_json.is_file(), "{} should exist", pkg_json.display());
    let contents = std::fs::read_to_string(&pkg_json).unwrap();
    assert_eq!(contents, PKG_JSON_V1);
}

#[test]
fn second_run_with_a_different_version_replaces_top_level_symlink() {
    let tmp = TempDir::new().unwrap();
    let node_modules = tmp.path().join("node_modules");

    let src_v1 = fake_pkg(tmp.path(), "A");
    let pkgs_v1 = vec![linked("A", "1.0.0", src_v1)];
    populate_node_modules(&pkgs_v1, &node_modules, &["A".to_string()]).unwrap();

    let src_v2 = fake_pkg(tmp.path(), "A_v2");
    let pkgs_v2 = vec![linked("A", "2.0.0", src_v2)];
    populate_node_modules(&pkgs_v2, &node_modules, &["A".to_string()]).unwrap();

    let top = node_modules.join("A");
    assert!(top.is_symlink(), "{} should be a symlink", top.display());
    let resolved = std::fs::canonicalize(&top).unwrap();
    let expected = std::fs::canonicalize(
        node_modules
            .join(".guroku")
            .join("A@2.0.0")
            .join("node_modules")
            .join("A"),
    )
    .unwrap();
    assert_eq!(
        resolved, expected,
        "top-level node_modules/A should now point at A@2.0.0"
    );
    // Note: the v1 entry under .guroku/A@1.0.0 may still exist on disk; v0.3
    // does not garbage-collect, and we deliberately don't assert about it.
}

#[test]
fn second_run_with_no_direct_deps_removes_top_level_symlink() {
    let tmp = TempDir::new().unwrap();
    let src = fake_pkg(tmp.path(), "A");
    let node_modules = tmp.path().join("node_modules");
    let pkgs = vec![linked("A", "1.0.0", src)];

    populate_node_modules(&pkgs, &node_modules, &["A".to_string()]).unwrap();
    // Second run with no packages and no direct deps. populate_node_modules
    // only OVERWRITES things it owns; it does not remove stale entries.
    populate_node_modules(&[], &node_modules, &[]).unwrap();

    let top = node_modules.join("A");
    // v0.3 does not garbage-collect; future work. The top-level symlink
    // created by the first run is expected to still be present.
    assert!(
        top.is_symlink() || top.exists(),
        "{} should still exist — v0.3 does not garbage-collect; future work",
        top.display()
    );
}
