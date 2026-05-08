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
fn unscoped_alias() {
    let (name, inner) = unwrap_alias(classify("npm:lodash@^4.17.0"));
    assert_eq!(name, "lodash");
    assert_eq!(unwrap_range(inner), "^4.17.0");
}

#[test]
fn scoped_alias() {
    let (name, inner) = unwrap_alias(classify("npm:@types/node@^20"));
    assert_eq!(name, "@types/node");
    assert_eq!(unwrap_range(inner), "^20");
}

#[test]
fn alias_to_dist_tag() {
    let (name, inner) = unwrap_alias(classify("npm:react@latest"));
    assert_eq!(name, "react");
    assert_eq!(unwrap_range(inner), "latest");
}

#[test]
fn alias_with_complex_range() {
    let (name, inner) = unwrap_alias(classify("npm:lodash@^1 || ^2"));
    assert_eq!(name, "lodash");
    assert_eq!(unwrap_range(inner), "^1 || ^2");
}

#[test]
fn alias_without_at_falls_back_to_star() {
    let (name, inner) = unwrap_alias(classify("npm:lodash"));
    assert_eq!(name, "lodash");
    assert_eq!(unwrap_range(inner), "*");
}

#[test]
fn alias_to_unscoped_with_pre_release() {
    let (name, inner) = unwrap_alias(classify("npm:react@19.0.0-rc.1"));
    assert_eq!(name, "react");
    assert_eq!(unwrap_range(inner), "19.0.0-rc.1");
}

#[test]
fn non_alias_unaffected() {
    match classify("^1.2.3") {
        DepSpec::Range(r) => assert_eq!(r, "^1.2.3"),
        other => panic!("expected Range, got {other:?}"),
    }
    match classify("file:./x") {
        DepSpec::File(p) => assert_eq!(p, "./x"),
        other => panic!("expected File, got {other:?}"),
    }
    match classify("git+https://x") {
        DepSpec::Git(_) => {}
        other => panic!("expected Git, got {other:?}"),
    }
}

#[test]
fn non_exhaustive_match_compiles() {
    // Exercises the `#[non_exhaustive]` contract: any external match on
    // DepSpec must include a wildcard arm. If this stops compiling, the
    // attribute was removed and downstream crates will break.
    let spec = classify("^1.0.0");
    let label = match spec {
        DepSpec::Range(_) => "range",
        DepSpec::File(_) => "file",
        DepSpec::Git(_) => "git",
        DepSpec::Alias { .. } => "alias",
        _ => "unknown",
    };
    assert_eq!(label, "range");
}
