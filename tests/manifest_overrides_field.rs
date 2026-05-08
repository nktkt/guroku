use guroku::manifest::Manifest;
use std::fs;
use tempfile::TempDir;

#[test]
fn parses_overrides_field() {
    let json = r#"{"overrides":{"foo":"1.2.3","bar":"2"}}"#;
    let manifest: Manifest = serde_json::from_str(json).unwrap();
    assert_eq!(
        manifest.overrides.get("foo").map(String::as_str),
        Some("1.2.3")
    );
    assert_eq!(manifest.overrides.get("bar").map(String::as_str), Some("2"));
}

#[test]
fn parses_resolutions_field() {
    let json = r#"{"resolutions":{"foo":"3.0.0"}}"#;
    let manifest: Manifest = serde_json::from_str(json).unwrap();
    assert_eq!(
        manifest.resolutions.get("foo").map(String::as_str),
        Some("3.0.0")
    );
}

#[test]
fn default_overrides_is_empty() {
    let json = r#"{"name":"x"}"#;
    let manifest: Manifest = serde_json::from_str(json).unwrap();
    assert!(manifest.overrides.is_empty());
}

#[test]
fn default_resolutions_is_empty() {
    let json = r#"{"name":"x"}"#;
    let manifest: Manifest = serde_json::from_str(json).unwrap();
    assert!(manifest.resolutions.is_empty());
}

#[test]
fn overrides_round_trip() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("package.json");

    let original = r#"{
  "name": "rt",
  "version": "0.1.0",
  "overrides": { "foo": "1.2.3", "bar": "2" }
}
"#;
    fs::write(&path, original).unwrap();

    let manifest = Manifest::read_from(&path).unwrap();
    manifest.write_to(&path).unwrap();
    let reloaded = Manifest::read_from(&path).unwrap();

    assert_eq!(reloaded.overrides, manifest.overrides);
    assert_eq!(
        reloaded.overrides.get("foo").map(String::as_str),
        Some("1.2.3")
    );
    assert_eq!(reloaded.overrides.get("bar").map(String::as_str), Some("2"));
}

#[test]
fn written_json_uses_overrides_key() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("package.json");

    let mut manifest = Manifest {
        name: Some("w".to_string()),
        version: Some("0.0.1".to_string()),
        ..Default::default()
    };
    manifest.overrides.insert("a".to_string(), "1".to_string());

    manifest.write_to(&path).unwrap();
    let contents = fs::read_to_string(&path).unwrap();
    assert!(
        contents.contains("\"overrides\""),
        "expected `\"overrides\"` key in serialized JSON, got: {contents}"
    );
}

#[test]
fn overrides_does_not_pollute_other() {
    let json = r#"{"overrides":{"foo":"1"}}"#;
    let manifest: Manifest = serde_json::from_str(json).unwrap();
    assert!(!manifest.other.contains_key("overrides"));
}
