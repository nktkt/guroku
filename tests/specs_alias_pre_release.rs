use guroku::specs::{classify, DepSpec};

fn unwrap_alias(spec: DepSpec) -> (String, DepSpec) {
    match spec {
        DepSpec::Alias { real_name, inner } => (real_name, *inner),
        other => panic!("expected Alias, got {other:?}"),
    }
}

fn unwrap_range(spec: DepSpec) -> String {
    match spec {
        DepSpec::Range(r) => r,
        other => panic!("expected Range, got {other:?}"),
    }
}

#[test]
fn alias_with_pre_release_exact() {
    let (name, inner) = unwrap_alias(classify("npm:react@19.0.0-rc.1"));
    assert_eq!(name, "react");
    assert_eq!(unwrap_range(inner), "19.0.0-rc.1");
}

#[test]
fn alias_with_pre_release_caret() {
    let (name, inner) = unwrap_alias(classify("npm:react@^19.0.0-rc.1"));
    assert_eq!(name, "react");
    assert_eq!(unwrap_range(inner), "^19.0.0-rc.1");
}

#[test]
fn alias_with_dist_tag_next() {
    let (name, inner) = unwrap_alias(classify("npm:react@next"));
    assert_eq!(name, "react");
    assert_eq!(unwrap_range(inner), "next");
}

#[test]
fn alias_with_star() {
    let (name, inner) = unwrap_alias(classify("npm:lodash@*"));
    assert_eq!(name, "lodash");
    assert_eq!(unwrap_range(inner), "*");
}

#[test]
fn alias_with_empty_spec() {
    let (name, inner) = unwrap_alias(classify("npm:lodash"));
    assert_eq!(name, "lodash");
    assert_eq!(unwrap_range(inner), "*");
}

#[test]
fn alias_with_complex_or_range() {
    let (name, inner) = unwrap_alias(classify("npm:lodash@^1 || ^2"));
    assert_eq!(name, "lodash");
    assert_eq!(unwrap_range(inner), "^1 || ^2");
}

#[test]
fn alias_to_pre_release_dist_tag() {
    let (name, inner) = unwrap_alias(classify("npm:react@beta"));
    assert_eq!(name, "react");
    assert_eq!(unwrap_range(inner), "beta");
}
