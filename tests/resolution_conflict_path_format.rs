//! Display tests for `GurokuError::ResolutionConflict` with v1.1 path-style
//! `requested_by` (e.g. "a > b > c").

use guroku::error::GurokuError;

fn make(name: &str, chosen: &str, requested: &str, requested_by: &str) -> GurokuError {
    GurokuError::ResolutionConflict {
        name: name.to_string(),
        chosen: chosen.to_string(),
        requested: requested.to_string(),
        requested_by: requested_by.to_string(),
    }
}

#[test]
fn display_with_path_chain() {
    let err = make("left-pad", "1.0.0", "^2.0.0", "a > b > c");
    let s = format!("{}", err);
    assert!(
        s.ends_with("but `a > b > c` requires `^2.0.0`"),
        "unexpected trailing portion: {}",
        s
    );
}

#[test]
fn display_with_root_marker() {
    let err = make("foo", "1.2.3", "^2.0.0", "<root>");
    let s = format!("{}", err);
    assert!(s.contains("<root>"), "expected <root> marker in: {}", s);
}

#[test]
fn display_path_segments_separated_by_arrow() {
    let err = make("acorn", "8.0.0", "^7.0.0", "webpack > terser");
    let s = format!("{}", err);
    assert!(
        s.contains("webpack > terser"),
        "expected verbatim path in: {}",
        s
    );
}

#[test]
fn equality_of_resolution_conflicts() {
    let a = make("foo", "1.0.0", "^2.0.0", "x > y");
    let b = make("foo", "1.0.0", "^2.0.0", "x > y");
    assert_eq!(format!("{}", a), format!("{}", b));
}

#[test]
fn path_with_unicode_safe() {
    let err = make("baz", "0.1.0", "^0.2.0", "@scope/x > foo > bar");
    let s = format!("{}", err);
    assert!(
        s.contains("@scope/"),
        "expected @scope/ to be preserved in: {}",
        s
    );
}
