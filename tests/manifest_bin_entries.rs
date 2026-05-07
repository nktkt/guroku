use guroku::manifest::Manifest;

fn parse(s: &str) -> Manifest {
    serde_json::from_str(s).expect("parse manifest")
}

#[test]
fn string_form_unscoped() {
    let m = parse(r#"{"name":"foo","bin":"./cli.js"}"#);
    let entries = m.bin_entries();
    assert_eq!(entries, vec![("foo".to_string(), "./cli.js".to_string())]);
}

#[test]
fn string_form_scoped_uses_unscoped_name() {
    let m = parse(r#"{"name":"@types/foo","bin":"./cli.js"}"#);
    let entries = m.bin_entries();
    assert_eq!(entries, vec![("foo".to_string(), "./cli.js".to_string())]);
}

#[test]
fn object_form_two_entries() {
    let m = parse(r#"{"bin":{"a":"./a.js","b":"./b.js"}}"#);
    let entries = m.bin_entries();
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().any(|(k, v)| k == "a" && v == "./a.js"));
    assert!(entries.iter().any(|(k, v)| k == "b" && v == "./b.js"));
}

#[test]
fn object_form_non_string_values_dropped() {
    let m = parse(r#"{"bin":{"a":"./a.js","b":42}}"#);
    let entries = m.bin_entries();
    assert_eq!(entries, vec![("a".to_string(), "./a.js".to_string())]);
}

#[test]
fn bin_field_absent_returns_empty() {
    let m = parse(r#"{"name":"foo"}"#);
    assert!(m.bin_entries().is_empty());
}

#[test]
fn string_form_without_name_returns_empty() {
    let m = parse(r#"{"bin":"./cli.js"}"#);
    assert!(m.bin_entries().is_empty());
}

#[test]
fn bin_field_array_returns_empty() {
    let m = parse(r#"{"bin":["a","b"]}"#);
    assert!(m.bin_entries().is_empty());
}
