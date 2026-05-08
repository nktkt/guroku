use guroku::manifest::Manifest;
use guroku::overrides::lookup_with_path;

fn parse(json: &str) -> Manifest {
    serde_json::from_str(json).expect("valid manifest json")
}

#[test]
fn glob_matches_leaf_name() {
    let m = parse(r#"{"resolutions":{"**/foo":"1.0.0"}}"#);
    assert_eq!(
        lookup_with_path(&m, &["root", "bar", "foo"]),
        Some("1.0.0".into())
    );
}

#[test]
fn glob_matches_top_level_name() {
    let m = parse(r#"{"resolutions":{"**/foo":"1.0.0"}}"#);
    assert_eq!(lookup_with_path(&m, &["foo"]), Some("1.0.0".into()));
}

#[test]
fn glob_does_not_match_prefix() {
    let m = parse(r#"{"resolutions":{"**/foo":"1.0.0"}}"#);
    assert_eq!(lookup_with_path(&m, &["foobar"]), None);
}

#[test]
fn glob_loses_to_flat_overrides() {
    let m = parse(r#"{"overrides":{"foo":"2.0.0"},"resolutions":{"**/foo":"1.0.0"}}"#);
    assert_eq!(lookup_with_path(&m, &["root", "foo"]), Some("2.0.0".into()));
}

#[test]
fn glob_loses_to_flat_resolutions() {
    let m = parse(r#"{"resolutions":{"foo":"2.0.0","**/foo":"1.0.0"}}"#);
    assert_eq!(lookup_with_path(&m, &["root", "foo"]), Some("2.0.0".into()));
}

#[test]
fn glob_with_partial_path_pattern_does_not_match() {
    let m = parse(r#"{"resolutions":{"pkg/**/foo":"1.0.0"}}"#);
    assert_eq!(lookup_with_path(&m, &["pkg", "x", "foo"]), None);
}

#[test]
fn scoped_name_glob_works() {
    let m = parse(r#"{"resolutions":{"**/@types/node":"20.0.0"}}"#);
    assert_eq!(
        lookup_with_path(&m, &["@types/node"]),
        Some("20.0.0".into())
    );
}

#[test]
fn unrelated_name_no_match() {
    let m = parse(r#"{"resolutions":{"**/foo":"1.0.0"}}"#);
    assert_eq!(lookup_with_path(&m, &["bar"]), None);
}
