//! Tests for the pnpm-style object form of `package.json#workspaces`.
//!
//! `Manifest::workspace_globs()` must accept both the array form and the
//! `{"packages": [...]}` object form, and `workspaces::discover` must honour
//! both via that helper.

use guroku::manifest::Manifest;
use guroku::workspaces;
use tempfile::TempDir;

#[test]
fn object_form_globs_extracted() {
    let body = r#"{
        "name": "root",
        "version": "0.0.0",
        "workspaces": { "packages": ["a/*", "b/*"] }
    }"#;
    let manifest: Manifest = serde_json::from_str(body).expect("parse manifest");
    assert_eq!(
        manifest.workspace_globs(),
        vec!["a/*".to_string(), "b/*".to_string()]
    );
}

#[test]
fn object_form_without_packages_key_returns_empty() {
    let body = r#"{
        "name": "root",
        "version": "0.0.0",
        "workspaces": { "otherKey": ["a/*", "b/*"] }
    }"#;
    let manifest: Manifest = serde_json::from_str(body).expect("parse manifest");
    assert!(
        manifest.workspace_globs().is_empty(),
        "expected empty globs when `packages` key is absent, got {:?}",
        manifest.workspace_globs()
    );
}

#[test]
fn discover_with_object_form() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    std::fs::write(
        root.join("package.json"),
        r#"{
            "name": "root",
            "version": "0.0.0",
            "workspaces": { "packages": ["pkgs/*"] }
        }"#,
    )
    .unwrap();

    let pkgs = root.join("pkgs");
    std::fs::create_dir_all(pkgs.join("x")).unwrap();
    std::fs::create_dir_all(pkgs.join("y")).unwrap();
    std::fs::write(
        pkgs.join("x").join("package.json"),
        r#"{"name":"x","version":"0.0.0"}"#,
    )
    .unwrap();
    std::fs::write(
        pkgs.join("y").join("package.json"),
        r#"{"name":"y","version":"0.0.0"}"#,
    )
    .unwrap();

    let found = workspaces::discover(root).expect("discover workspaces");
    assert_eq!(found.len(), 2, "expected 2 workspaces, got {found:?}");

    let names: Vec<_> = found.iter().filter_map(|w| w.name()).collect();
    assert!(names.contains(&"x"), "missing workspace `x` in {names:?}");
    assert!(names.contains(&"y"), "missing workspace `y` in {names:?}");
}

#[test]
fn array_form_still_works_alongside_object_test() {
    let body = r#"{
        "name": "root",
        "version": "0.0.0",
        "workspaces": ["a/*"]
    }"#;
    let manifest: Manifest = serde_json::from_str(body).expect("parse manifest");
    assert_eq!(manifest.workspace_globs(), vec!["a/*".to_string()]);
}

#[test]
fn array_form_skips_non_string_entries() {
    let body = r#"{
        "name": "root",
        "version": "0.0.0",
        "workspaces": ["a/*", 42, true, "b/*"]
    }"#;
    let manifest: Manifest = serde_json::from_str(body).expect("parse manifest");
    assert_eq!(
        manifest.workspace_globs(),
        vec!["a/*".to_string(), "b/*".to_string()]
    );
}
