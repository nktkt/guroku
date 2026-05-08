use guroku::manifest::Manifest;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

#[test]
fn unknown_top_level_string_lands_in_other() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("package.json");
    fs::write(&path, r#"{"name":"x","version":"1","weirdField":"value"}"#).unwrap();

    let manifest = Manifest::read_from(&path).expect("read manifest");
    assert!(manifest.other.contains_key("weirdField"));
    assert_eq!(
        manifest.other.get("weirdField"),
        Some(&Value::String("value".to_string()))
    );
}

#[test]
fn unknown_object_field_preserved_through_round_trip() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("package.json");
    fs::write(
        &path,
        r#"{"name":"x","version":"1","customMetadata":{"a":1}}"#,
    )
    .unwrap();

    let manifest = Manifest::read_from(&path).expect("read manifest");
    manifest.write_to(&path).unwrap();
    let reloaded = Manifest::read_from(&path).expect("re-read manifest");

    let cm = reloaded
        .other
        .get("customMetadata")
        .expect("customMetadata preserved");
    assert!(cm.is_object(), "expected object, got {cm:?}");
    assert_eq!(cm.get("a"), Some(&serde_json::json!(1)));
}

#[test]
fn unknown_array_field_round_trips() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("package.json");
    fs::write(&path, r#"{"name":"x","version":"1","keywords":["a","b"]}"#).unwrap();

    let manifest = Manifest::read_from(&path).expect("read manifest");
    manifest.write_to(&path).unwrap();
    let reloaded = Manifest::read_from(&path).expect("re-read manifest");

    let kw = reloaded.other.get("keywords").expect("keywords preserved");
    assert!(kw.is_array(), "expected array, got {kw:?}");
    assert_eq!(
        kw,
        &serde_json::json!(["a", "b"]),
        "array contents preserved"
    );
}

#[test]
fn boolean_unknown_field_round_trips() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("package.json");
    fs::write(&path, r#"{"name":"x","version":"1","private":true}"#).unwrap();

    let manifest = Manifest::read_from(&path).expect("read manifest");
    manifest.write_to(&path).unwrap();
    let reloaded = Manifest::read_from(&path).expect("re-read manifest");

    assert_eq!(reloaded.other.get("private"), Some(&Value::Bool(true)));
}

#[test]
fn multiple_unknown_fields_all_preserved() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("package.json");
    fs::write(
        &path,
        r#"{
            "name":"x",
            "version":"1",
            "weirdField":"value",
            "customMetadata":{"a":1},
            "keywords":["a","b"]
        }"#,
    )
    .unwrap();

    let manifest = Manifest::read_from(&path).expect("read manifest");
    assert!(manifest.other.contains_key("weirdField"));
    assert!(manifest.other.contains_key("customMetadata"));
    assert!(manifest.other.contains_key("keywords"));
}
