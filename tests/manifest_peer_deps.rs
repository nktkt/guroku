use guroku::manifest::Manifest;
use std::fs;
use tempfile::TempDir;

fn write_manifest(dir: &TempDir, contents: &str) -> std::path::PathBuf {
    let path = dir.path().join("package.json");
    fs::write(&path, contents).unwrap();
    path
}

#[test]
fn parses_peer_dependencies() {
    let dir = TempDir::new().unwrap();
    let path = write_manifest(
        &dir,
        r#"{
            "name": "demo",
            "version": "0.1.0",
            "peerDependencies": {
                "react": "^18",
                "react-dom": "^18"
            }
        }"#,
    );
    let manifest = Manifest::read_from(&path).unwrap();
    assert_eq!(manifest.peer_dependencies.len(), 2);
    assert_eq!(
        manifest.peer_dependencies.get("react").map(String::as_str),
        Some("^18")
    );
    assert_eq!(
        manifest
            .peer_dependencies
            .get("react-dom")
            .map(String::as_str),
        Some("^18")
    );
}

#[test]
fn peer_deps_excluded_from_all_dependencies() {
    let dir = TempDir::new().unwrap();
    let path = write_manifest(
        &dir,
        r#"{
            "name": "demo",
            "version": "0.1.0",
            "dependencies": { "lodash": "^4" },
            "devDependencies": { "jest": "^29" },
            "peerDependencies": { "react": "^18" }
        }"#,
    );
    let manifest = Manifest::read_from(&path).unwrap();
    let all: Vec<String> = manifest
        .all_dependencies()
        .map(|(name, _)| name.to_string())
        .collect();
    assert!(all.iter().any(|n| n == "lodash"));
    assert!(all.iter().any(|n| n == "jest"));
    assert!(
        !all.iter().any(|n| n == "react"),
        "peer deps must not appear in all_dependencies"
    );
}

#[test]
fn roundtrip_preserves_peer_deps() {
    let dir = TempDir::new().unwrap();
    let src = write_manifest(
        &dir,
        r#"{
            "name": "demo",
            "version": "0.1.0",
            "peerDependencies": {
                "react": "^18",
                "vue": "^3"
            }
        }"#,
    );
    let original = Manifest::read_from(&src).unwrap();
    let dst = dir.path().join("out.json");
    original.write_to(&dst).unwrap();
    let reloaded = Manifest::read_from(&dst).unwrap();
    assert_eq!(original.peer_dependencies, reloaded.peer_dependencies);
}

#[test]
fn peer_deps_field_uses_camelcase_in_json() {
    let dir = TempDir::new().unwrap();
    let src = write_manifest(
        &dir,
        r#"{
            "name": "demo",
            "version": "0.1.0",
            "peerDependencies": { "react": "^18" }
        }"#,
    );
    let manifest = Manifest::read_from(&src).unwrap();
    let dst = dir.path().join("out.json");
    manifest.write_to(&dst).unwrap();
    let written = fs::read_to_string(&dst).unwrap();
    assert!(
        written.contains("\"peerDependencies\""),
        "expected camelCase peerDependencies key in output, got: {written}"
    );
}

#[test]
fn default_peer_deps_is_empty() {
    let manifest = Manifest::default();
    assert!(manifest.peer_dependencies.is_empty());
}
