use guroku::manifest::Manifest;
use tempfile::TempDir;

#[test]
fn parses_optional_dependencies() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("package.json");
    let body = r#"{
        "name":"app",
        "version":"0.1.0",
        "optionalDependencies":{"fsevents":"^2.3.0"}
    }"#;
    std::fs::write(&path, body).unwrap();

    let manifest = Manifest::read_from(&path).expect("read manifest");
    assert_eq!(
        manifest
            .optional_dependencies
            .get("fsevents")
            .map(String::as_str),
        Some("^2.3.0")
    );
}

#[test]
fn optional_deps_excluded_from_all_dependencies() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("package.json");
    let body = r#"{
        "name":"app",
        "version":"0.1.0",
        "dependencies":{"lodash":"4.17.21"},
        "devDependencies":{"jest":"^29.0.0"},
        "optionalDependencies":{"fsevents":"^2.3.0"}
    }"#;
    std::fs::write(&path, body).unwrap();

    let manifest = Manifest::read_from(&path).expect("read manifest");
    let names: Vec<&String> = manifest.all_dependencies().map(|(k, _)| k).collect();
    assert!(names.iter().any(|n| n.as_str() == "lodash"));
    assert!(names.iter().any(|n| n.as_str() == "jest"));
    assert!(!names.iter().any(|n| n.as_str() == "fsevents"));
}

#[test]
fn remove_finds_optional_dep() {
    let mut manifest = Manifest::default();
    manifest
        .optional_dependencies
        .insert("fsevents".to_string(), "^2.3.0".to_string());

    assert!(manifest.remove_dependency("fsevents"));
    assert!(!manifest.optional_dependencies.contains_key("fsevents"));
    assert!(!manifest.remove_dependency("fsevents"));
}

#[test]
fn roundtrip_preserves_optional_deps() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("package.json");
    let body = r#"{
        "name":"app",
        "version":"0.1.0",
        "optionalDependencies":{"fsevents":"^2.3.0"}
    }"#;
    std::fs::write(&path, body).unwrap();

    let manifest = Manifest::read_from(&path).expect("read manifest");
    let out = dir.path().join("package.out.json");
    manifest.write_to(&out).expect("write manifest");

    let reread = Manifest::read_from(&out).expect("reread manifest");
    assert_eq!(
        reread
            .optional_dependencies
            .get("fsevents")
            .map(String::as_str),
        Some("^2.3.0")
    );
}

#[test]
fn optional_deps_field_uses_camelcase_in_json() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("package.json");
    let mut manifest = Manifest {
        name: Some("app".to_string()),
        ..Manifest::default()
    };
    manifest
        .optional_dependencies
        .insert("fsevents".to_string(), "^2.3.0".to_string());
    manifest.write_to(&path).expect("write manifest");

    let written = std::fs::read_to_string(&path).unwrap();
    assert!(
        written.contains("\"optionalDependencies\""),
        "expected camelCase key in output, got: {written}"
    );
}

#[test]
fn default_optional_deps_is_empty() {
    let manifest = Manifest::default();
    assert!(manifest.optional_dependencies.is_empty());
}
