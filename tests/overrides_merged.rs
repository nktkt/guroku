use guroku::manifest::Manifest;
use guroku::overrides;

fn parse(body: &str) -> Manifest {
    serde_json::from_str(body).expect("parse manifest json")
}

#[test]
fn empty_manifest_returns_empty_map() {
    let manifest = Manifest::default();
    assert!(overrides::merged(&manifest).is_empty());
}

#[test]
fn only_overrides_propagate() {
    let manifest = parse(r#"{"overrides":{"a":"1","b":"2"}}"#);
    let merged = overrides::merged(&manifest);
    assert_eq!(merged.get("a").map(String::as_str), Some("1"));
    assert_eq!(merged.get("b").map(String::as_str), Some("2"));
    assert_eq!(merged.len(), 2);
}

#[test]
fn only_resolutions_propagate() {
    let manifest = parse(r#"{"resolutions":{"a":"3"}}"#);
    let merged = overrides::merged(&manifest);
    assert_eq!(merged.get("a").map(String::as_str), Some("3"));
    assert_eq!(merged.len(), 1);
}

#[test]
fn overrides_take_precedence() {
    let manifest =
        parse(r#"{"overrides":{"a":"override-value"},"resolutions":{"a":"resolution-value"}}"#);
    let merged = overrides::merged(&manifest);
    assert_eq!(merged.get("a").map(String::as_str), Some("override-value"));
}

#[test]
fn non_conflicting_keys_unioned() {
    let manifest = parse(r#"{"overrides":{"a":"1"},"resolutions":{"b":"2"}}"#);
    let merged = overrides::merged(&manifest);
    assert_eq!(merged.get("a").map(String::as_str), Some("1"));
    assert_eq!(merged.get("b").map(String::as_str), Some("2"));
    assert_eq!(merged.len(), 2);
}

#[test]
fn merged_returns_a_clone() {
    let manifest = parse(r#"{"overrides":{"a":"1"},"resolutions":{"b":"2"}}"#);
    let mut merged = overrides::merged(&manifest);
    merged.insert("a".to_string(), "mutated".to_string());
    merged.insert("c".to_string(), "added".to_string());
    merged.remove("b");

    // Original manifest maps must be untouched.
    assert_eq!(manifest.overrides.get("a").map(String::as_str), Some("1"));
    assert!(!manifest.overrides.contains_key("c"));
    assert_eq!(manifest.resolutions.get("b").map(String::as_str), Some("2"));
}
