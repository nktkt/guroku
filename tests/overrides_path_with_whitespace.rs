//! Whitespace tolerance for path-keyed overrides (v1.1).
//!
//! The path-key parser splits on `>` and trims each segment, so
//! `"a > b > c"`, `"a>b>c"`, `"a >b> c"`, and tab-padded variants all
//! denote the same path through the dependency graph.

use guroku::manifest::Manifest;
use guroku::overrides::lookup_with_path;

fn manifest_with_overrides(entries: &[(&str, &str)]) -> Manifest {
    let mut m = Manifest::default();
    for (k, v) in entries {
        m.overrides.insert((*k).to_string(), (*v).to_string());
    }
    m
}

#[test]
fn path_no_spaces() {
    let m = manifest_with_overrides(&[("a>b>c", "9.9.9")]);
    assert_eq!(
        lookup_with_path(&m, &["a", "b", "c"]),
        Some("9.9.9".to_string())
    );
}

#[test]
fn path_spaces_around_arrows() {
    let m = manifest_with_overrides(&[("a > b > c", "9.9.9")]);
    assert_eq!(
        lookup_with_path(&m, &["a", "b", "c"]),
        Some("9.9.9".to_string())
    );
}

#[test]
fn path_mixed_whitespace() {
    let m = manifest_with_overrides(&[("a >b> c", "9.9.9")]);
    assert_eq!(
        lookup_with_path(&m, &["a", "b", "c"]),
        Some("9.9.9".to_string())
    );
}

#[test]
fn path_tabs() {
    let m = manifest_with_overrides(&[("a\t>\tb\t>\tc", "9.9.9")]);
    assert_eq!(
        lookup_with_path(&m, &["a", "b", "c"]),
        Some("9.9.9".to_string())
    );
}

#[test]
fn path_does_not_match_subset() {
    // Path key requires `a` at the front; looking up just ["b", "c"]
    // must not match because the manifest entry demands the full chain.
    let m = manifest_with_overrides(&[("a > b > c", "9.9.9")]);
    assert_eq!(lookup_with_path(&m, &["b", "c"]), None);
}

#[test]
fn path_matches_as_suffix() {
    // Path-keyed overrides match as a contiguous suffix of the
    // resolution path, so `"b > c"` matches a lookup for ["a","b","c"].
    let m = manifest_with_overrides(&[("b > c", "9.9.9")]);
    assert_eq!(
        lookup_with_path(&m, &["a", "b", "c"]),
        Some("9.9.9".to_string())
    );
}

#[test]
fn flat_name_alone_matches_anywhere() {
    // A bare name (no `>`) is a flat override and matches any path
    // whose leaf is that name.
    let m = manifest_with_overrides(&[("c", "9.9.9")]);
    assert_eq!(
        lookup_with_path(&m, &["a", "b", "c"]),
        Some("9.9.9".to_string())
    );
}
