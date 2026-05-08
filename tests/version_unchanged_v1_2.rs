//! v1.2 stability checks for `guroku::version`.
//!
//! Locks down the public surface of the version module: re-exported types,
//! function signatures, and basic semantic behaviour.

#[test]
fn range_type_re_exported() {
    let _: Option<guroku::version::Range> = None;
}

#[test]
fn version_type_re_exported() {
    let _: Option<guroku::version::Version> = None;
}

#[test]
fn parse_range_signature() {
    let r = guroku::version::parse_range("^1").unwrap();
    let _: guroku::version::Range = r;
}

#[test]
fn parse_version_signature() {
    let v = guroku::version::parse_version("1.2.3").unwrap();
    let _: guroku::version::Version = v;
}

#[test]
fn max_satisfying_signature() {
    let v = guroku::version::max_satisfying(
        ["1.0.0", "2.0.0"],
        &guroku::version::parse_range("^1").unwrap(),
    );
    assert_eq!(v.unwrap().to_string(), "1.0.0");
}

#[test]
fn parse_range_empty_is_star() {
    let r = guroku::version::parse_range("").expect("empty range parses");
    let v = guroku::version::parse_version("0.0.1").unwrap();
    assert!(r.satisfies(&v), "empty range should behave like *");
    let v2 = guroku::version::parse_version("9.9.9").unwrap();
    assert!(r.satisfies(&v2), "empty range should match 9.9.9");
}

#[test]
fn range_satisfies_basic_caret() {
    let r = guroku::version::parse_range("^1.2.3").unwrap();
    let v = guroku::version::parse_version("1.5.0").unwrap();
    assert!(r.satisfies(&v), "^1.2.3 should match 1.5.0");
}

#[test]
fn version_ord_matches_semver() {
    let a = guroku::version::parse_version("1.2.3").unwrap();
    let b = guroku::version::parse_version("1.2.4").unwrap();
    assert!(a < b);
}
