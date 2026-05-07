fn meta() -> guroku::registry::PackageMetadata {
    let json = r#"{
        "name": "demo",
        "versions": {
            "0.9.0":{"name":"demo","version":"0.9.0","dist":{"tarball":"https://e/0.9.0.tgz"}},
            "1.0.0":{"name":"demo","version":"1.0.0","dist":{"tarball":"https://e/1.0.0.tgz"}},
            "1.5.0":{"name":"demo","version":"1.5.0","dist":{"tarball":"https://e/1.5.0.tgz"}},
            "2.0.0":{"name":"demo","version":"2.0.0","dist":{"tarball":"https://e/2.0.0.tgz"}},
            "2.5.0":{"name":"demo","version":"2.5.0","dist":{"tarball":"https://e/2.5.0.tgz"}},
            "3.0.0":{"name":"demo","version":"3.0.0","dist":{"tarball":"https://e/3.0.0.tgz"}}
        }
    }"#;
    serde_json::from_str(json).unwrap()
}

#[test]
fn caret_one_or_caret_two_picks_highest_two_x() {
    let m = meta();
    let v = m.resolve("^1 || ^2").unwrap();
    assert_eq!(v.version, "2.5.0");
}

#[test]
fn caret_zero_or_caret_one_picks_highest_one_x() {
    let m = meta();
    let v = m.resolve("^0 || ^1").unwrap();
    assert_eq!(v.version, "1.5.0");
}

#[test]
fn caret_one_or_caret_three_skips_two() {
    let m = meta();
    let v = m.resolve("^1 || ^3").unwrap();
    assert_eq!(v.version, "3.0.0");
    assert_ne!(v.version, "2.0.0");
    assert_ne!(v.version, "2.5.0");
}

#[test]
fn or_with_no_match_returns_err() {
    let m = meta();
    assert!(m.resolve("^7 || ^8").is_err());
}

#[test]
fn or_with_one_branch_matching_picks_that_branch() {
    let m = meta();
    let v = m.resolve("^7 || ^1").unwrap();
    assert_eq!(v.version, "1.5.0");
}
