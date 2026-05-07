use guroku::version::{parse_range, parse_version};

fn s(range: &str, version: &str) -> bool {
    parse_range(range)
        .unwrap()
        .satisfies(&parse_version(version).unwrap())
}

#[test]
fn caret_one_two_three() {
    assert!(s("^1.2.3", "1.2.3"));
    assert!(s("^1.2.3", "1.5.0"));
    assert!(s("^1.2.3", "1.99.99"));
    assert!(!s("^1.2.3", "1.2.2"));
    assert!(!s("^1.2.3", "2.0.0"));
    assert!(!s("^1.2.3", "0.9.0"));
}

#[test]
fn tilde_one_two() {
    assert!(s("~1.2.0", "1.2.0"));
    assert!(s("~1.2.0", "1.2.99"));
    assert!(!s("~1.2.0", "1.3.0"));
}

#[test]
fn caret_zero_x() {
    assert!(s("^0.2.3", "0.2.3"));
    assert!(s("^0.2.3", "0.2.99"));
    assert!(!s("^0.2.3", "0.3.0"));
}

#[test]
fn inclusive_range() {
    assert!(s(">=1.0 <2.0", "1.0.0"));
    assert!(s(">=1.0 <2.0", "1.5.5"));
    assert!(!s(">=1.0 <2.0", "0.9.9"));
    assert!(!s(">=1.0 <2.0", "2.0.0"));
}

#[test]
fn or_combinator() {
    assert!(s("^1 || ^2", "1.5.0"));
    assert!(s("^1 || ^2", "2.5.0"));
    assert!(!s("^1 || ^2", "0.9.0"));
    assert!(!s("^1 || ^2", "3.0.0"));
}

#[test]
fn wildcard() {
    assert!(s("*", "1.0.0"));
    assert!(s("*", "99.99.99"));
}

#[test]
fn pre_release_handling() {
    assert!(!s("^1.0.0", "1.2.3-beta.1"));
}

#[test]
fn exact_version() {
    assert!(s("1.2.3", "1.2.3"));
    assert!(!s("1.2.3", "1.2.4"));
    assert!(!s("1.2.3", "1.2.2"));
}
