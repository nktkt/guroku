use guroku::manifest::Manifest;
use guroku::overrides::lookup_with_path;

fn parse(json: &str) -> Manifest {
    serde_json::from_str(json).expect("valid manifest json")
}

#[test]
fn path_match_exact() {
    let m = parse(r#"{"overrides":{"a > b > c":"1.0.0"}}"#);
    assert_eq!(lookup_with_path(&m, &["a", "b", "c"]), Some("1.0.0".into()));
}

#[test]
fn path_match_as_suffix() {
    let m = parse(r#"{"overrides":{"a > b > c":"1.0.0"}}"#);
    assert_eq!(
        lookup_with_path(&m, &["root", "a", "b", "c"]),
        Some("1.0.0".into())
    );
}

#[test]
fn path_no_match_when_intermediate_changes() {
    let m = parse(r#"{"overrides":{"a > b > c":"1.0.0"}}"#);
    assert_eq!(lookup_with_path(&m, &["a", "x", "c"]), None);
}

#[test]
fn path_match_with_whitespace() {
    let m = parse(r#"{"overrides":{"a > b > c":"1.0.0"}}"#);
    assert_eq!(lookup_with_path(&m, &["a", "b", "c"]), Some("1.0.0".into()));
}

#[test]
fn path_match_no_whitespace() {
    let m = parse(r#"{"overrides":{"a>b>c":"1.0.0"}}"#);
    assert_eq!(lookup_with_path(&m, &["a", "b", "c"]), Some("1.0.0".into()));
}

#[test]
fn path_overrides_beat_flat() {
    let m = parse(r#"{"overrides":{"c":"2.0.0","a > b > c":"1.0.0"}}"#);
    assert_eq!(lookup_with_path(&m, &["a", "b", "c"]), Some("1.0.0".into()));
}

#[test]
fn flat_falls_through_when_path_missing() {
    let m = parse(r#"{"overrides":{"c":"2.0.0","a > x > c":"1.0.0"}}"#);
    assert_eq!(lookup_with_path(&m, &["a", "b", "c"]), Some("2.0.0".into()));
}

#[test]
fn path_in_resolutions_loses_to_flat_in_overrides() {
    let m = parse(r#"{"overrides":{"c":"2.0.0"},"resolutions":{"a > b > c":"1.0.0"}}"#);
    assert_eq!(lookup_with_path(&m, &["a", "b", "c"]), Some("2.0.0".into()));
}
