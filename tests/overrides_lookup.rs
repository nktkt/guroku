use guroku::manifest::Manifest;
use guroku::overrides::lookup;

fn parse(json: &str) -> Manifest {
    serde_json::from_str(json).expect("valid manifest json")
}

#[test]
fn unknown_name_returns_none() {
    let m = parse("{}");
    assert_eq!(lookup(&m, "foo"), None);
}

#[test]
fn overrides_only_returns_value() {
    let m = parse(r#"{"overrides":{"foo":"1.2.3"}}"#);
    assert_eq!(lookup(&m, "foo"), Some("1.2.3".into()));
}

#[test]
fn resolutions_only_returns_value() {
    let m = parse(r#"{"resolutions":{"foo":"1.2.3"}}"#);
    assert_eq!(lookup(&m, "foo"), Some("1.2.3".into()));
}

#[test]
fn overrides_wins_over_resolutions() {
    let m = parse(r#"{"overrides":{"foo":"9.9.9"},"resolutions":{"foo":"1.2.3"}}"#);
    assert_eq!(lookup(&m, "foo"), Some("9.9.9".into()));
}

#[test]
fn lookup_only_finds_exact_name() {
    let m = parse(r#"{"overrides":{"foo":"1"}}"#);
    assert_eq!(lookup(&m, "bar"), None);
}

#[test]
fn empty_string_value_still_returned() {
    let m = parse(r#"{"overrides":{"foo":""}}"#);
    assert_eq!(lookup(&m, "foo"), Some("".into()));
}

#[test]
fn case_sensitive() {
    let m = parse(r#"{"overrides":{"Foo":"1"}}"#);
    assert_eq!(lookup(&m, "foo"), None);
}
