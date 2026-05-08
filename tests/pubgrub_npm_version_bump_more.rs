use guroku::pubgrub_resolver::NpmVersion;
use guroku::version::parse_version;
use pubgrub::version::Version as PubGrubVersion;

fn npm(s: &str) -> NpmVersion {
    NpmVersion(parse_version(s).unwrap())
}

#[test]
fn bump_zero_version() {
    assert_eq!(npm("0.0.0").bump().to_string(), "0.0.1");
}

#[test]
fn bump_zero_dot_zero_dot_one() {
    assert_eq!(npm("0.0.1").bump().to_string(), "0.0.2");
}

#[test]
fn bump_high_patch() {
    assert_eq!(npm("1.2.99").bump().to_string(), "1.2.100");
}

#[test]
fn bump_does_not_change_major() {
    let bumped = npm("2.0.0").bump();
    assert_eq!(bumped.to_string(), "2.0.1");
    assert_ne!(bumped.to_string(), "3.0.0");
    assert_ne!(bumped.to_string(), "2.1.0");
}

#[test]
fn bump_does_not_change_minor() {
    assert_eq!(npm("1.5.0").bump().to_string(), "1.5.1");
}

#[test]
fn bump_idempotent_under_repeated_bumps() {
    let v1 = npm("1.2.3").bump();
    assert_eq!(v1.to_string(), "1.2.4");
    let v2 = v1.bump();
    assert_eq!(v2.to_string(), "1.2.5");
    let v3 = v2.bump();
    assert_eq!(v3.to_string(), "1.2.6");
}
