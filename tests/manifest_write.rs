use guroku::manifest::Manifest;
use std::fs;
use tempfile::TempDir;

#[test]
fn roundtrip_preserves_deps_and_dev_deps() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("package.json");

    let mut manifest = Manifest {
        name: Some("my-pkg".to_string()),
        version: Some("1.2.3".to_string()),
        ..Default::default()
    };
    manifest.add_dependency("left-pad", "^1.3.0");
    manifest.add_dependency("lodash", "^4.17.21");
    manifest
        .dev_dependencies
        .insert("jest".to_string(), "^29.0.0".to_string());
    manifest
        .dev_dependencies
        .insert("typescript".to_string(), "^5.0.0".to_string());

    manifest.write_to(&path).unwrap();
    let loaded = Manifest::read_from(&path).unwrap();

    assert_eq!(loaded.name.as_deref(), Some("my-pkg"));
    assert_eq!(loaded.version.as_deref(), Some("1.2.3"));
    assert_eq!(loaded.dependencies, manifest.dependencies);
    assert_eq!(loaded.dev_dependencies, manifest.dev_dependencies);
}

#[test]
fn output_ends_with_newline() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("package.json");

    let manifest = Manifest {
        name: Some("nl".to_string()),
        version: Some("0.0.1".to_string()),
        ..Default::default()
    };
    manifest.write_to(&path).unwrap();

    let contents = fs::read_to_string(&path).unwrap();
    assert!(
        contents.ends_with('\n'),
        "expected trailing newline, got {contents:?}"
    );
}

#[test]
fn roundtrip_preserves_unknown_fields() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("package.json");

    let original = r#"{
  "name": "with-extras",
  "version": "0.1.0",
  "private": true,
  "scripts": { "test": "jest", "build": "tsc" }
}
"#;
    fs::write(&path, original).unwrap();

    let manifest = Manifest::read_from(&path).unwrap();
    manifest.write_to(&path).unwrap();
    let reloaded = Manifest::read_from(&path).unwrap();

    assert_eq!(
        reloaded.other.get("private"),
        Some(&serde_json::json!(true))
    );
    // v0.4 promoted `scripts` into a typed Manifest field; assert via that.
    assert_eq!(
        reloaded.scripts.get("test").map(String::as_str),
        Some("jest")
    );
    assert_eq!(
        reloaded.scripts.get("build").map(String::as_str),
        Some("tsc")
    );
}
