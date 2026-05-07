fn meta() -> guroku::registry::PackageMetadata {
    let json = r#"{
        "name": "demo",
        "dist-tags": {"latest":"2.0.0"},
        "versions": {
            "1.2.0": {"name":"demo","version":"1.2.0","dist":{"tarball":"https://e/1.2.0.tgz"}},
            "1.2.5": {"name":"demo","version":"1.2.5","dist":{"tarball":"https://e/1.2.5.tgz"}},
            "1.2.9": {"name":"demo","version":"1.2.9","dist":{"tarball":"https://e/1.2.9.tgz"}},
            "1.3.0": {"name":"demo","version":"1.3.0","dist":{"tarball":"https://e/1.3.0.tgz"}},
            "1.3.5": {"name":"demo","version":"1.3.5","dist":{"tarball":"https://e/1.3.5.tgz"}},
            "2.0.0": {"name":"demo","version":"2.0.0","dist":{"tarball":"https://e/2.0.0.tgz"}}
        }
    }"#;
    serde_json::from_str(json).unwrap()
}

#[test]
fn tilde_one_two_three_locks_to_one_two_x() {
    let m = meta();
    let v = m.resolve("~1.2.3").expect("should resolve");
    assert_eq!(v.version, "1.2.9");
}

#[test]
fn tilde_one_two_does_not_cross_minor() {
    let m = meta();
    let v = m.resolve("~1.2.0").expect("should resolve");
    assert_ne!(v.version, "1.3.0");
    assert_ne!(v.version, "1.3.5");
    assert_ne!(v.version, "2.0.0");
    assert!(v.version.starts_with("1.2."));
}

#[test]
fn tilde_one_three_picks_one_three_five() {
    let m = meta();
    let v = m.resolve("~1.3.0").expect("should resolve");
    assert_eq!(v.version, "1.3.5");
}

#[test]
fn tilde_one_does_not_cross_major() {
    let m = meta();
    let v = m.resolve("~1").expect("should resolve");
    assert_eq!(v.version, "1.3.5");
}

#[test]
fn tilde_with_no_match_returns_err() {
    let m = meta();
    assert!(m.resolve("~5").is_err());
}
