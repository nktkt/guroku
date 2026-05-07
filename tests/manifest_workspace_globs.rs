use guroku::manifest::Manifest;

fn globs(s: &str) -> Vec<String> {
    let m: Manifest = serde_json::from_str(s).expect("parse manifest");
    m.workspace_globs()
}

#[test]
fn array_form() {
    assert_eq!(
        globs(r#"{"workspaces":["packages/*"]}"#),
        vec!["packages/*"]
    );
}

#[test]
fn array_form_multiple() {
    assert_eq!(globs(r#"{"workspaces":["a/*","b/*"]}"#), vec!["a/*", "b/*"]);
}

#[test]
fn object_form_packages_key() {
    assert_eq!(
        globs(r#"{"workspaces":{"packages":["packages/*"]}}"#),
        vec!["packages/*"]
    );
}

#[test]
fn object_form_unrecognised_key_returns_empty() {
    assert!(globs(r#"{"workspaces":{"foo":["x"]}}"#).is_empty());
}

#[test]
fn absent_returns_empty() {
    assert!(globs(r#"{"name":"x"}"#).is_empty());
}

#[test]
fn string_value_returns_empty() {
    assert!(globs(r#"{"workspaces":"packages/*"}"#).is_empty());
}

#[test]
fn array_skips_non_strings() {
    assert_eq!(
        globs(r#"{"workspaces":["a/*", 42, true, "b/*"]}"#),
        vec!["a/*", "b/*"]
    );
}

#[test]
fn empty_array_returns_empty_vec() {
    assert!(globs(r#"{"workspaces":[]}"#).is_empty());
}
