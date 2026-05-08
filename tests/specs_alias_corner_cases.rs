//! Corner cases for `specs::classify` around alias-vs-not-alias inputs.
//!
//! These nail down the v1.1 contract: `npm:` is the alias prefix, anything
//! else falls through to the existing File / Git / Range classification.

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
fn bare_npm_prefix_with_no_at_is_alias_with_default_spec() {
    // `npm:react` (no `@version`) means "alias to react, any version".
    let (name, inner) = unwrap_alias(classify("npm:react"));
    assert_eq!(name, "react");
    assert_eq!(unwrap_range(inner), "*");
}

#[test]
fn npm_prefix_with_scoped_real_name() {
    // The split must be on the LAST `@` so the leading scope `@` survives.
    let (name, inner) = unwrap_alias(classify("npm:@types/node@^20"));
    assert_eq!(name, "@types/node");
    assert_eq!(unwrap_range(inner), "^20");
}

#[test]
fn bare_word_is_range_not_alias() {
    // A bare identifier with no prefix is a dist-tag (e.g. `latest`),
    // which the resolver handles as a Range string.
    assert_eq!(unwrap_range(classify("react")), "react");
}

#[test]
fn version_only_is_range() {
    assert_eq!(unwrap_range(classify("1.2.3")), "1.2.3");
}

#[test]
fn caret_only_is_range() {
    assert_eq!(unwrap_range(classify("^1.2.3")), "^1.2.3");
}

#[test]
fn file_url_is_file_not_range() {
    match classify("file:./local") {
        DepSpec::File(p) => assert_eq!(p, "./local"),
        other => panic!("expected File, got {other:?}"),
    }
}

#[test]
fn git_url_is_git_not_range() {
    match classify("git+ssh://git@github.com/x/y.git#main") {
        DepSpec::Git(g) => {
            assert_eq!(g.url, "ssh://git@github.com/x/y.git");
            assert_eq!(g.revision.as_deref(), Some("main"));
        }
        other => panic!("expected Git, got {other:?}"),
    }
}

#[test]
fn npm_prefix_with_git_inner_is_alias_of_range_string() {
    // The OUTER classify only knows about the `npm:` prefix; once it has
    // pulled off `<real-name>@`, the remainder is stuffed verbatim into a
    // Range string. We deliberately use a git URL without an embedded `@`
    // so the rsplit_once on '@' picks the alias separator and not the
    // `git@host` userinfo.
    //
    // NOTE (v1.1): the inner Range may itself round-trip through classify
    // in a future v1.x release (so the inner becomes a real Git variant);
    // for v1.1 we keep the inner as a Range string and assert that here.
    let (name, inner) = unwrap_alias(classify("npm:foo@git+ssh://example.com/x/y.git"));
    assert_eq!(name, "foo");
    assert_eq!(unwrap_range(inner), "git+ssh://example.com/x/y.git");
}
