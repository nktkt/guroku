use guroku::error::GurokuError;
use guroku::version::{parse_range, parse_version};

#[test]
fn parse_range_caret() {
    assert!(parse_range("^1.2.3").is_ok());
}

#[test]
fn parse_range_tilde() {
    assert!(parse_range("~1.2.3").is_ok());
}

#[test]
fn parse_range_inclusive() {
    assert!(parse_range(">=1.0 <2.0").is_ok());
}

#[test]
fn parse_range_or() {
    assert!(parse_range("^1 || ^2").is_ok());
}

#[test]
fn parse_range_x_wildcard() {
    assert!(parse_range("1.2.x").is_ok());
}

#[test]
fn parse_range_star_returns_ok() {
    assert!(parse_range("*").is_ok());
}

#[test]
fn parse_range_empty_treated_as_star() {
    assert!(parse_range("").is_ok());
}

#[test]
fn parse_range_whitespace_trimmed() {
    assert!(parse_range("  ^1.2.3  ").is_ok());
}

#[test]
fn parse_range_garbage_fails() {
    let err = parse_range("not a range !!!").unwrap_err();
    assert!(matches!(err, GurokuError::InvalidVersionSpec { .. }));
}

#[test]
fn parse_version_ok() {
    assert!(parse_version("1.2.3").is_ok());
}

#[test]
fn parse_version_with_pre_release() {
    assert!(parse_version("1.2.3-beta.1").is_ok());
}

#[test]
fn parse_version_garbage_fails() {
    assert!(parse_version("not.a.version").is_err());
}
