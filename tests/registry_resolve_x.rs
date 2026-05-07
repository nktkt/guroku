fn meta() -> guroku::registry::PackageMetadata {
    let json = r#"{
        "name": "demo",
        "dist-tags": {"latest":"2.0.0"},
        "versions": {
            "1.0.0":{"name":"demo","version":"1.0.0","dist":{"tarball":"https://e/1.0.0.tgz"}},
            "1.2.0":{"name":"demo","version":"1.2.0","dist":{"tarball":"https://e/1.2.0.tgz"}},
            "1.2.5":{"name":"demo","version":"1.2.5","dist":{"tarball":"https://e/1.2.5.tgz"}},
            "1.5.0":{"name":"demo","version":"1.5.0","dist":{"tarball":"https://e/1.5.0.tgz"}},
            "2.0.0":{"name":"demo","version":"2.0.0","dist":{"tarball":"https://e/2.0.0.tgz"}}
        }
    }"#;
    serde_json::from_str(json).unwrap()
}

#[test]
fn one_two_x_locks_to_one_two_y() {
    let m = meta();
    let v = m.resolve("1.2.x").expect("1.2.x should resolve");
    assert_eq!(v.version, "1.2.5");
}

#[test]
fn one_x_locks_to_one_y_z() {
    let m = meta();
    let v = m.resolve("1.x").expect("1.x should resolve");
    assert_eq!(v.version, "1.5.0");
}

#[test]
fn star_picks_highest_overall() {
    let m = meta();
    let v = m.resolve("*").expect("* should resolve");
    assert_eq!(v.version, "2.0.0");
}

#[test]
fn cap_x_works_too() {
    let m = meta();
    let v = m.resolve("1.X").expect("1.X should resolve");
    assert_eq!(v.version, "1.5.0");
}

#[test]
fn x_no_match_returns_err() {
    let m = meta();
    assert!(m.resolve("9.x").is_err());
}
