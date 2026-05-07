fn meta() -> guroku::registry::PackageMetadata {
    let json = r#"{
        "name": "demo",
        "dist-tags": {
            "latest": "1.5.0",
            "next": "2.0.0-beta.1",
            "lts": "1.0.0"
        },
        "versions": {
            "1.0.0":{"name":"demo","version":"1.0.0","dist":{"tarball":"https://e/1.0.0.tgz"}},
            "1.5.0":{"name":"demo","version":"1.5.0","dist":{"tarball":"https://e/1.5.0.tgz"}},
            "2.0.0-beta.1":{"name":"demo","version":"2.0.0-beta.1","dist":{"tarball":"https://e/2.0.0-beta.1.tgz"}}
        }
    }"#;
    serde_json::from_str(json).unwrap()
}

#[test]
fn latest_resolves_to_dist_tag_target() {
    let m = meta();
    let v = m.resolve("latest").expect("latest should resolve");
    assert_eq!(v.version, "1.5.0");
}

#[test]
fn next_resolves_to_pre_release() {
    let m = meta();
    let v = m.resolve("next").expect("next should resolve");
    assert_eq!(v.version, "2.0.0-beta.1");
}

#[test]
fn lts_resolves_to_one_zero_zero() {
    let m = meta();
    let v = m.resolve("lts").expect("lts should resolve");
    assert_eq!(v.version, "1.0.0");
}

#[test]
fn unknown_tag_falls_through_to_semver() {
    let m = meta();
    assert!(
        m.resolve("alpha").is_err(),
        "unknown tag 'alpha' should not resolve"
    );
}

#[test]
fn tag_takes_precedence_over_version_string_clash() {
    let m = meta();
    let v = m.resolve("1.5.0").expect("exact version should resolve");
    assert_eq!(v.version, "1.5.0");
}
