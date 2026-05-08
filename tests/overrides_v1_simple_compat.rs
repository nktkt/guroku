//! v1.0 compatibility checks for `guroku::overrides`.
//!
//! v1.1 introduced path-keyed (`"a > b > c"`) and glob (`"**/foo"`) entries.
//! These tests pin down that the v1.0 entry points — flat-name `lookup` and
//! `merged` — keep behaving the way pre-1.1 callers expect.

use guroku::manifest::Manifest;
use guroku::overrides::{self, lookup};

fn parse(json: &str) -> Manifest {
    serde_json::from_str(json).expect("valid manifest json")
}

#[test]
fn lookup_finds_flat_overrides() {
    let m = parse(r#"{"overrides":{"foo":"1.0.0"}}"#);
    assert_eq!(lookup(&m, "foo"), Some("1.0.0".into()));
}

#[test]
fn lookup_finds_flat_resolutions() {
    let m = parse(r#"{"resolutions":{"foo":"1.0.0"}}"#);
    assert_eq!(lookup(&m, "foo"), Some("1.0.0".into()));
}

#[test]
fn lookup_overrides_beat_resolutions() {
    let m = parse(r#"{"overrides":{"foo":"9.9.9"},"resolutions":{"foo":"1.0.0"}}"#);
    assert_eq!(lookup(&m, "foo"), Some("9.9.9".into()));
}

#[test]
fn lookup_unknown_returns_none() {
    let m = parse("{}");
    assert_eq!(lookup(&m, "foo"), None);
}

#[test]
fn lookup_does_not_match_path_keyed() {
    // Path-keyed entries are intentionally invisible to the flat lookup —
    // callers that need them must use `lookup_with_path`.
    let m = parse(r#"{"overrides":{"a > b > c":"1.0.0"}}"#);
    assert_eq!(lookup(&m, "c"), None);
}

#[test]
fn lookup_surfaces_glob_via_v1_1_shim() {
    // v1.0 had no glob support at all, so manifests in the wild never
    // carried `**/foo` keys. The v1.1 `lookup` shim is `lookup_with_path`
    // with `&[name]`, and the precedence ladder ends with a glob fallback
    // — so a v1.1 manifest that does declare `**/foo` will surface
    // through the v1.0-style flat-name caller too. This is strictly more
    // permissive than the v1.0 behaviour (which couldn't see globs at
    // all), not a behavioural regression.
    let m = parse(r#"{"resolutions":{"**/foo":"1.0.0"}}"#);
    assert_eq!(lookup(&m, "foo"), Some("1.0.0".into()));
}

#[test]
fn merged_includes_all_kinds() {
    let m = parse(
        r#"{
            "overrides":   {"flat":"1","a > b":"2"},
            "resolutions": {"**/foo":"3"}
        }"#,
    );
    let merged = overrides::merged(&m);
    assert_eq!(merged.get("flat").map(String::as_str), Some("1"));
    assert_eq!(merged.get("a > b").map(String::as_str), Some("2"));
    assert_eq!(merged.get("**/foo").map(String::as_str), Some("3"));
    assert_eq!(merged.len(), 3);
}

#[test]
fn merged_overrides_win_on_key_collision() {
    let m = parse(r#"{"overrides":{"foo":"override"},"resolutions":{"foo":"resolution"}}"#);
    let merged = overrides::merged(&m);
    assert_eq!(merged.get("foo").map(String::as_str), Some("override"));
}
