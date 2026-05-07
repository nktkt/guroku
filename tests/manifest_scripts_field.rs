use guroku::manifest::Manifest;
use tempfile::TempDir;

#[test]
fn parses_scripts_object() {
    let json = r#"{"scripts":{"build":"tsc","test":"vitest"}}"#;
    let manifest: Manifest = serde_json::from_str(json).expect("parse manifest");
    assert_eq!(
        manifest.scripts.get("build").map(String::as_str),
        Some("tsc")
    );
    assert_eq!(
        manifest.scripts.get("test").map(String::as_str),
        Some("vitest")
    );
}

#[test]
fn scripts_field_uses_serde_default_on_absence() {
    let json = r#"{"name":"x"}"#;
    let manifest: Manifest = serde_json::from_str(json).expect("parse manifest");
    assert!(manifest.scripts.is_empty());
}

#[test]
fn scripts_not_in_other_after_v04() {
    let json = r#"{"scripts":{"build":"tsc","test":"vitest"}}"#;
    let manifest: Manifest = serde_json::from_str(json).expect("parse manifest");
    assert!(!manifest.other.contains_key("scripts"));
}

#[test]
fn roundtrip_preserves_scripts() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("in.json");
    let dst = dir.path().join("out.json");
    std::fs::write(&src, r#"{"scripts":{"build":"tsc","test":"vitest"}}"#).unwrap();

    let manifest = Manifest::read_from(&src).expect("read manifest");
    manifest.write_to(&dst).expect("write manifest");
    let reread = Manifest::read_from(&dst).expect("reread manifest");

    assert_eq!(reread.scripts, manifest.scripts);
    assert_eq!(reread.scripts.get("build").map(String::as_str), Some("tsc"));
    assert_eq!(
        reread.scripts.get("test").map(String::as_str),
        Some("vitest")
    );
}

#[test]
fn written_json_uses_scripts_key() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("package.json");
    let mut manifest = Manifest::default();
    manifest
        .scripts
        .insert("build".to_string(), "tsc".to_string());
    manifest.write_to(&path).expect("write manifest");

    let written = std::fs::read_to_string(&path).expect("read back");
    assert!(
        written.contains("\"scripts\""),
        "missing scripts key: {written}"
    );
    assert!(
        written.contains("\"build\""),
        "missing build key: {written}"
    );
}

#[test]
fn empty_scripts_round_trips_clean() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("in.json");
    let dst = dir.path().join("out.json");
    std::fs::write(&src, r#"{"name":"empty"}"#).unwrap();

    let manifest = Manifest::read_from(&src).expect("read manifest");
    assert!(manifest.scripts.is_empty());
    manifest.write_to(&dst).expect("write manifest");
    let reread = Manifest::read_from(&dst).expect("reread manifest");
    assert!(reread.scripts.is_empty());
}
