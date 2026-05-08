use guroku::pubgrub_resolver::NpmVersion;
use guroku::version::parse_version;
use pubgrub::version::Version as PubGrubVersion;

fn npm(s: &str) -> NpmVersion {
    NpmVersion(parse_version(s).unwrap())
}

#[test]
fn lowest_is_zero() {
    assert_eq!(NpmVersion::lowest().to_string(), "0.0.0");
}

#[test]
fn bump_increments_patch() {
    assert_eq!(npm("1.2.3").bump().to_string(), "1.2.4");
}

#[test]
fn bump_strips_prerelease() {
    let bumped = npm("1.2.3-rc.1").bump();
    assert_eq!(bumped.to_string(), "1.2.4");
    assert_ne!(bumped.to_string(), "1.2.4-rc.1");
}

#[test]
fn bump_strips_build_metadata() {
    assert_eq!(npm("1.2.3+build.42").bump().to_string(), "1.2.4");
}

#[test]
fn ord_matches_semver_ord() {
    assert!(npm("1.2.3") < npm("1.2.4"));
    assert!(npm("1.2.4") < npm("1.3.0"));
    assert!(npm("1.3.0") < npm("2.0.0"));
    assert!(npm("1.2.3-rc.1") < npm("1.2.3"));
}

#[test]
fn display_round_trips() {
    assert_eq!(
        format!("{}", NpmVersion(parse_version("1.2.3-rc.1").unwrap())),
        "1.2.3-rc.1"
    );
}
