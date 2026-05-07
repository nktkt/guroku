use guroku::manifest::Manifest;

#[test]
fn add_inserts_into_dependencies() {
    let mut m = Manifest::default();
    m.add_dependency("lodash", "^4.17.21");
    assert_eq!(
        m.dependencies.get("lodash").map(String::as_str),
        Some("^4.17.21")
    );
}

#[test]
fn add_overwrites_existing_spec() {
    let mut m = Manifest::default();
    m.add_dependency("lodash", "^4.17.20");
    m.add_dependency("lodash", "^4.17.21");
    assert_eq!(
        m.dependencies.get("lodash").map(String::as_str),
        Some("^4.17.21")
    );
}

#[test]
fn remove_returns_true_when_present() {
    let mut m = Manifest::default();
    m.add_dependency("lodash", "^4.17.21");
    assert!(m.remove_dependency("lodash"));
    assert!(!m.remove_dependency("lodash"));
}

#[test]
fn remove_searches_dev_dependencies_too() {
    let mut m = Manifest::default();
    m.dev_dependencies
        .insert("jest".to_string(), "^29.0.0".to_string());
    assert!(m.remove_dependency("jest"));
    assert!(!m.dev_dependencies.contains_key("jest"));
}

#[test]
fn remove_returns_false_for_unknown() {
    let mut m = Manifest::default();
    assert!(!m.remove_dependency("nope"));
}
