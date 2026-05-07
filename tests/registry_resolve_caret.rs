fn meta() -> guroku::registry::PackageMetadata {
    let json = r#"{
        "name": "demo",
        "dist-tags": {"latest": "2.1.0"},
        "versions": {
            "1.0.0": {"name":"demo","version":"1.0.0","dist":{"tarball":"https://example.com/demo-1.0.0.tgz"}},
            "1.2.3": {"name":"demo","version":"1.2.3","dist":{"tarball":"https://example.com/demo-1.2.3.tgz"}},
            "1.5.0": {"name":"demo","version":"1.5.0","dist":{"tarball":"https://example.com/demo-1.5.0.tgz"}},
            "2.0.0": {"name":"demo","version":"2.0.0","dist":{"tarball":"https://example.com/demo-2.0.0.tgz"}},
            "2.1.0": {"name":"demo","version":"2.1.0","dist":{"tarball":"https://example.com/demo-2.1.0.tgz"}}
        }
    }"#;
    serde_json::from_str(json).unwrap()
}

#[test]
fn caret_one_picks_highest_one_x() {
    let m = meta();
    let v = m.resolve("^1").expect("should resolve ^1");
    assert_eq!(v.version, "1.5.0");
}

#[test]
fn caret_one_two_three_picks_highest_one_x() {
    let m = meta();
    let v = m.resolve("^1.2.3").expect("should resolve ^1.2.3");
    assert_eq!(v.version, "1.5.0");
}

#[test]
fn caret_one_two_three_does_not_cross_major() {
    let m = meta();
    let v = m.resolve("^1.2.3").expect("should resolve ^1.2.3");
    assert_ne!(v.version, "2.0.0");
    assert_ne!(v.version, "2.1.0");
}

#[test]
fn caret_two_picks_highest_two_x() {
    let m = meta();
    let v = m.resolve("^2").expect("should resolve ^2");
    assert_eq!(v.version, "2.1.0");
}

#[test]
fn caret_with_no_match_returns_err() {
    let m = meta();
    assert!(m.resolve("^3").is_err());
}
