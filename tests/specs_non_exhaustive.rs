//! Compile-time guard: `guroku::specs::DepSpec` must remain `#[non_exhaustive]`
//! in v1.1. If a future PR removes the attribute, the wildcard arm below will
//! trigger an `unreachable_patterns` warning (denied here), failing the build.

#![deny(unreachable_patterns)]

use guroku::specs::{DepSpec, GitRef};

fn kind(s: &DepSpec) -> &'static str {
    match s {
        DepSpec::Range(_) => "range",
        DepSpec::File(_) => "file",
        DepSpec::Git(_) => "git",
        DepSpec::Alias { .. } => "alias",
        _ => "unknown",
    }
}

#[test]
fn match_with_wildcard_compiles() {
    // The whole point: this file compiling proves the wildcard arm is allowed
    // (i.e. `DepSpec` is still `#[non_exhaustive]`).
    let _ = kind as fn(&DepSpec) -> &'static str;
}

#[test]
fn range_kind() {
    assert_eq!(kind(&DepSpec::Range("^1".into())), "range");
}

#[test]
fn file_kind() {
    assert_eq!(kind(&DepSpec::File("./x".into())), "file");
}

#[test]
fn git_kind() {
    let g = GitRef {
        url: "https://example.com/repo.git".into(),
        revision: None,
    };
    assert_eq!(kind(&DepSpec::Git(g)), "git");
}

#[test]
fn alias_kind() {
    let s = DepSpec::Alias {
        real_name: "real-pkg".into(),
        inner: Box::new(DepSpec::Range("^1".into())),
    };
    assert_eq!(kind(&s), "alias");
}
