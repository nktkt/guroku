use guroku::registry::PackageMetadata;

fn metadata_with_tags() -> PackageMetadata {
    let json = r#"{
        "name": "demo",
        "dist-tags": { "latest": "1.1.0" },
        "versions": {
            "1.0.0": {
                "name": "demo",
                "version": "1.0.0",
                "dist": { "tarball": "https://example.com/demo-1.0.0.tgz" }
            },
            "1.1.0": {
                "name": "demo",
                "version": "1.1.0",
                "dist": { "tarball": "https://example.com/demo-1.1.0.tgz" }
            }
        }
    }"#;
    serde_json::from_str(json).expect("parse metadata")
}

#[test]
fn resolve_exact_version() {
    let meta = metadata_with_tags();
    let v = meta.resolve("1.0.0").expect("resolve 1.0.0");
    assert_eq!(v.version, "1.0.0");
    assert_eq!(v.name, "demo");
    assert_eq!(
        v.dist.tarball.as_str(),
        "https://example.com/demo-1.0.0.tgz"
    );
}

#[test]
fn resolve_latest_dist_tag() {
    let meta = metadata_with_tags();
    let latest = meta.resolve("latest").expect("resolve latest");
    assert_eq!(latest.version, "1.1.0");
    let star = meta.resolve("*").expect("resolve *");
    assert_eq!(star.version, "1.1.0");
}

#[test]
fn resolve_unknown_falls_back_to_latest() {
    let meta = metadata_with_tags();
    let v = meta.resolve("^2").expect("falls back to latest");
    assert_eq!(v.version, "1.1.0");
}

#[test]
fn resolve_no_match_when_no_latest() {
    let json = r#"{
        "name": "demo",
        "versions": {
            "1.0.0": {
                "name": "demo",
                "version": "1.0.0",
                "dist": { "tarball": "https://example.com/demo-1.0.0.tgz" }
            }
        }
    }"#;
    let meta: PackageMetadata = serde_json::from_str(json).expect("parse metadata");
    assert!(meta.dist_tags.is_empty());
    let err = meta.resolve("^2");
    assert!(err.is_err(), "expected Err for bogus spec without latest");
}
