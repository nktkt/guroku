#![deny(unreachable_patterns)]

use guroku::specs::{classify, DepSpec, GitRef};

#[test]
fn dep_spec_still_non_exhaustive() {
    let s = DepSpec::Range("1.0.0".into());
    match s {
        DepSpec::Range(_) => {}
        DepSpec::File(_) => {}
        DepSpec::Git(_) => {}
        DepSpec::Alias { .. } => {}
        _ => {}
    }
}

#[test]
fn dep_spec_range_constructor() {
    let _ = DepSpec::Range("1.2.3".into());
}

#[test]
fn dep_spec_file_constructor() {
    let _ = DepSpec::File("./local".into());
}

#[test]
fn dep_spec_git_constructor() {
    let g = GitRef {
        url: "https://x/y.git".into(),
        revision: None,
    };
    let _ = DepSpec::Git(g);
}

#[test]
fn dep_spec_alias_constructor() {
    let _ = DepSpec::Alias {
        real_name: "real".into(),
        inner: Box::new(DepSpec::Range("^1".into())),
    };
}

#[test]
fn classify_function_signature_unchanged() {
    let _: fn(&str) -> DepSpec = guroku::specs::classify;
}

#[test]
fn unparse_function_round_trips_v1_specs() {
    let parsed = classify("^1.2.3");
    let out = guroku::specs::unparse(&parsed);
    assert_eq!(out, "^1.2.3");
}
