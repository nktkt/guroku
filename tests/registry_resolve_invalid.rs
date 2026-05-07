use guroku::GurokuError;

fn meta_one_zero_zero() -> guroku::registry::PackageMetadata {
    let json = r#"{
        "name": "demo",
        "versions": {
            "1.0.0": {"name":"demo","version":"1.0.0","dist":{"tarball":"https://example.com/demo-1.0.0.tgz"}}
        }
    }"#;
    serde_json::from_str(json).unwrap()
}

fn meta_one_x_only() -> guroku::registry::PackageMetadata {
    let json = r#"{
        "name": "demo",
        "versions": {
            "1.0.0": {"name":"demo","version":"1.0.0","dist":{"tarball":"https://example.com/demo-1.0.0.tgz"}},
            "1.2.3": {"name":"demo","version":"1.2.3","dist":{"tarball":"https://example.com/demo-1.2.3.tgz"}},
            "1.9.9": {"name":"demo","version":"1.9.9","dist":{"tarball":"https://example.com/demo-1.9.9.tgz"}}
        }
    }"#;
    serde_json::from_str(json).unwrap()
}

fn meta_empty() -> guroku::registry::PackageMetadata {
    let json = r#"{
        "name": "demo",
        "versions": {}
    }"#;
    serde_json::from_str(json).unwrap()
}

#[test]
fn garbage_spec_returns_err() {
    let m = meta_one_zero_zero();
    let err = m.resolve("not a version!!!").unwrap_err();
    assert!(matches!(err, GurokuError::NoMatchingVersion { .. }));
}

#[test]
fn non_existent_exact_version_returns_err() {
    let m = meta_one_zero_zero();
    let err = m.resolve("9.9.9").unwrap_err();
    assert!(matches!(err, GurokuError::NoMatchingVersion { .. }));
}

#[test]
fn out_of_range_caret_returns_err() {
    let m = meta_one_x_only();
    let err = m.resolve("^99").unwrap_err();
    assert!(matches!(err, GurokuError::NoMatchingVersion { .. }));
}

#[test]
fn error_carries_package_name_and_spec() {
    let m = meta_one_zero_zero();
    let input = "9.9.9";
    let err = m.resolve(input).unwrap_err();
    match err {
        GurokuError::NoMatchingVersion { name, spec } => {
            assert_eq!(name, "demo");
            assert!(
                spec.contains(input),
                "spec `{spec}` should contain original input `{input}`"
            );
        }
        other => panic!("expected NoMatchingVersion, got {other:?}"),
    }
}

#[test]
fn empty_versions_returns_err_for_anything_real() {
    let m = meta_empty();
    let err = m.resolve("^1").unwrap_err();
    assert!(matches!(err, GurokuError::NoMatchingVersion { .. }));
}
