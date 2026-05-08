use guroku::registry::PackageMetadata;
use guroku::version::{max_satisfying, parse_range, parse_version};

fn meta() -> PackageMetadata {
    let json = r#"{
        "name": "demo",
        "dist-tags": {"latest": "2.0.0"},
        "versions": {
            "1.0.0": {"name":"demo","version":"1.0.0","dist":{"tarball":"https://example.com/demo-1.0.0.tgz"}},
            "1.5.0": {"name":"demo","version":"1.5.0","dist":{"tarball":"https://example.com/demo-1.5.0.tgz"}},
            "2.0.0": {"name":"demo","version":"2.0.0","dist":{"tarball":"https://example.com/demo-2.0.0.tgz"}}
        }
    }"#;
    serde_json::from_str(json).unwrap()
}

#[test]
fn package_metadata_resolve_picks_highest_in_caret_range() {
    let m = meta();
    let v = m.resolve("^1").expect("should resolve ^1");
    assert_eq!(v.version, "1.5.0");
}

#[test]
fn range_satisfies_after_metadata_pick() {
    let m = meta();
    let v = m.resolve("^1.0").expect("should resolve ^1.0");
    let range = parse_range("^1.0").expect("range parses");
    let picked = parse_version(&v.version).expect("picked version parses");
    assert!(
        range.satisfies(&picked),
        "picked {} should satisfy ^1.0",
        v.version
    );
}

#[test]
fn max_satisfying_picks_highest_when_multiple_match() {
    let range = parse_range("^1").expect("range parses");
    let candidates = ["1.0.0", "1.2.0", "1.5.3", "2.0.0"];
    let got = max_satisfying(candidates.iter().copied(), &range).map(|v| v.to_string());
    assert_eq!(got.as_deref(), Some("1.5.3"));
}

#[test]
fn compatible_ranges_dont_force_lower_version() {
    let range = parse_range("^1.0.0").expect("range parses");
    let candidates = ["1.0.0", "1.5.0", "2.0.0"];
    let got = max_satisfying(candidates.iter().copied(), &range).map(|v| v.to_string());
    assert_eq!(
        got.as_deref(),
        Some("1.5.0"),
        "v1.1 must not pessimistically pick a lower version when constraints are compatible"
    );
}
