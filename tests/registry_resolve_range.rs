fn meta() -> guroku::registry::PackageMetadata {
    let json = r#"{
        "name": "demo",
        "versions": {
            "0.9.0": {"name":"demo","version":"0.9.0","dist":{"tarball":"https://e/0.9.0.tgz"}},
            "1.0.0": {"name":"demo","version":"1.0.0","dist":{"tarball":"https://e/1.0.0.tgz"}},
            "1.5.0": {"name":"demo","version":"1.5.0","dist":{"tarball":"https://e/1.5.0.tgz"}},
            "2.0.0": {"name":"demo","version":"2.0.0","dist":{"tarball":"https://e/2.0.0.tgz"}},
            "2.5.0": {"name":"demo","version":"2.5.0","dist":{"tarball":"https://e/2.5.0.tgz"}},
            "3.0.0": {"name":"demo","version":"3.0.0","dist":{"tarball":"https://e/3.0.0.tgz"}}
        }
    }"#;
    serde_json::from_str(json).unwrap()
}

#[test]
fn gte_lt_picks_highest_in_range() {
    let m = meta();
    let v = m.resolve(">=1.0 <2.0").expect("should resolve");
    assert_eq!(v.version, "1.5.0");
}

#[test]
fn gt_picks_highest_above() {
    let m = meta();
    let v = m.resolve(">1.0.0").expect("should resolve");
    assert_eq!(v.version, "3.0.0");
}

#[test]
fn lte_picks_highest_at_or_below() {
    let m = meta();
    let v = m.resolve("<=1.5.0").expect("should resolve");
    assert_eq!(v.version, "1.5.0");
}

#[test]
fn lt_excludes_endpoint() {
    let m = meta();
    let v = m.resolve("<2.0.0").expect("should resolve");
    assert_eq!(v.version, "1.5.0");
}

#[test]
fn gte_includes_endpoint() {
    let m = meta();
    let v = m.resolve(">=2.0.0").expect("should resolve");
    assert_eq!(v.version, "3.0.0");
}

#[test]
fn combined_no_match() {
    let m = meta();
    assert!(m.resolve(">=10 <20").is_err());
}
