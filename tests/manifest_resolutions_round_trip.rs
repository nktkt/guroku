use guroku::manifest::Manifest;
use std::fs;
use tempfile::TempDir;

#[test]
fn parses_resolutions_object() {
    let json = r#"{"resolutions":{"foo":"1","bar":"2"}}"#;
    let manifest: Manifest = serde_json::from_str(json).unwrap();
    assert_eq!(
        manifest.resolutions.get("foo").map(String::as_str),
        Some("1")
    );
    assert_eq!(
        manifest.resolutions.get("bar").map(String::as_str),
        Some("2")
    );
}

#[test]
fn default_is_empty() {
    let json = r#"{"name":"x"}"#;
    let manifest: Manifest = serde_json::from_str(json).unwrap();
    assert!(manifest.resolutions.is_empty());
}

#[test]
fn round_trip_via_disk() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("package.json");

    let original = r#"{
  "name": "rt",
  "version": "0.1.0",
  "resolutions": { "foo": "1.0.0" }
}
"#;
    fs::write(&path, original).unwrap();

    let manifest = Manifest::read_from(&path).unwrap();
    assert_eq!(
        manifest.resolutions.get("foo").map(String::as_str),
        Some("1.0.0")
    );

    manifest.write_to(&path).unwrap();
    let reloaded = Manifest::read_from(&path).unwrap();

    assert_eq!(reloaded.resolutions, manifest.resolutions);
    assert_eq!(
        reloaded.resolutions.get("foo").map(String::as_str),
        Some("1.0.0")
    );
}

#[test]
fn serialised_json_uses_resolutions_key() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("package.json");

    let mut manifest = Manifest {
        name: Some("w".to_string()),
        version: Some("0.0.1".to_string()),
        ..Default::default()
    };
    manifest
        .resolutions
        .insert("a".to_string(), "1".to_string());

    manifest.write_to(&path).unwrap();
    let contents = fs::read_to_string(&path).unwrap();
    assert!(
        contents.contains("\"resolutions\""),
        "expected `\"resolutions\"` key in serialized JSON, got: {contents}"
    );
}

#[test]
fn resolutions_does_not_pollute_other() {
    let json = r#"{"resolutions":{"foo":"1"}}"#;
    let manifest: Manifest = serde_json::from_str(json).unwrap();
    assert!(!manifest.other.contains_key("resolutions"));
}

#[test]
fn resolutions_and_overrides_can_coexist_on_disk() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("package.json");

    let original = r#"{
  "name": "both",
  "version": "0.1.0",
  "overrides": { "foo": "1.0.0", "shared": "9.9.9" },
  "resolutions": { "bar": "2.0.0", "shared": "9.9.9" }
}
"#;
    fs::write(&path, original).unwrap();

    let manifest = Manifest::read_from(&path).unwrap();
    assert!(!manifest.overrides.is_empty());
    assert!(!manifest.resolutions.is_empty());
    assert_eq!(
        manifest.overrides.get("foo").map(String::as_str),
        Some("1.0.0")
    );
    assert_eq!(
        manifest.resolutions.get("bar").map(String::as_str),
        Some("2.0.0")
    );
}
