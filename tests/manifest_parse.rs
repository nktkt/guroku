use guroku::manifest::Manifest;
use tempfile::TempDir;

#[test]
fn parses_minimal_manifest() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("package.json");
    std::fs::write(&path, r#"{"name":"foo","version":"1.0.0"}"#).unwrap();

    let manifest = Manifest::read_from(&path).expect("read manifest");
    assert_eq!(manifest.name.as_deref(), Some("foo"));
    assert_eq!(manifest.version.as_deref(), Some("1.0.0"));
}

#[test]
fn parses_dependencies_and_dev_dependencies() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("package.json");
    let body = r#"{
        "name":"app",
        "version":"0.1.0",
        "dependencies":{"left-pad":"^1.0.0","lodash":"4.17.21"},
        "devDependencies":{"jest":"^29.0.0"}
    }"#;
    std::fs::write(&path, body).unwrap();

    let manifest = Manifest::read_from(&path).expect("read manifest");
    assert_eq!(
        manifest.dependencies.get("left-pad").map(String::as_str),
        Some("^1.0.0")
    );
    assert_eq!(
        manifest.dependencies.get("lodash").map(String::as_str),
        Some("4.17.21")
    );
    assert_eq!(
        manifest.dev_dependencies.get("jest").map(String::as_str),
        Some("^29.0.0")
    );
    assert!(!manifest.dev_dependencies.contains_key("left-pad"));
}

#[test]
fn unknown_fields_land_in_other() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("package.json");
    let body = r#"{
        "name":"app",
        "version":"0.0.1",
        "scripts":{"test":"echo"},
        "private":true
    }"#;
    std::fs::write(&path, body).unwrap();

    let manifest = Manifest::read_from(&path).expect("read manifest");
    assert_eq!(
        manifest.other.get("private"),
        Some(&serde_json::json!(true))
    );
    // v0.4 promoted `scripts` into a real Manifest field, so it no longer
    // lands in `other`. Confirm the new shape.
    assert_eq!(
        manifest.scripts.get("test").map(String::as_str),
        Some("echo")
    );
    assert!(!manifest.other.contains_key("scripts"));
}
