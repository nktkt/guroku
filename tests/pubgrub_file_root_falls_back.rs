//! The pubgrub resolver delegates `file:` and `git:` roots to the legacy
//! resolver via `plan_roots`. That decision is made by classifying each
//! root spec with `guroku::specs::classify`. `plan_roots` itself is not
//! public API, so we test the underlying classifier here — if these
//! classifications change, the fallback in `pubgrub_resolver` will break.

use guroku::specs::{classify, DepSpec};

#[test]
fn file_url_classifies_as_file() {
    let s = classify("file:./node_modules/lib");
    match s {
        DepSpec::File(p) => assert_eq!(p, "./node_modules/lib"),
        other => panic!("expected DepSpec::File, got {other:?}"),
    }
}

#[test]
fn git_url_classifies_as_git() {
    let s = classify("git+https://github.com/x/y.git");
    assert!(
        matches!(s, DepSpec::Git(_)),
        "expected DepSpec::Git for git+https URL"
    );
}

#[test]
fn github_shorthand_classifies_as_git() {
    let s = classify("github:nktkt/guroku");
    assert!(
        matches!(s, DepSpec::Git(_)),
        "expected DepSpec::Git for github: shorthand"
    );
}

#[test]
fn range_does_not_classify_as_file_or_git() {
    // Defends against accidentally matching schemes too eagerly: a plain
    // semver range must remain a Range so pubgrub handles it directly.
    let s = classify("^1.2.3");
    assert!(
        !matches!(s, DepSpec::File(_) | DepSpec::Git(_)),
        "plain semver range should not classify as File or Git, got {s:?}"
    );
    assert!(
        matches!(s, DepSpec::Range(_)),
        "expected DepSpec::Range, got {s:?}"
    );
}

#[test]
fn npm_alias_of_range_does_not_trigger_fallback() {
    // Per `plan_roots` source: only `Alias { inner: not Range }` triggers
    // the fallback. A Range-of-alias root pubgrubs normally.
    let s = classify("npm:lodash@^4");
    match s {
        DepSpec::Alias { real_name, inner } => {
            assert_eq!(real_name, "lodash");
            match *inner {
                DepSpec::Range(r) => assert_eq!(r, "^4"),
                other => panic!("expected inner DepSpec::Range, got {other:?}"),
            }
        }
        other => panic!("expected DepSpec::Alias, got {other:?}"),
    }
}

#[test]
fn npm_alias_of_file_inner_is_range_string() {
    use guroku::specs::{classify, DepSpec};
    let s = classify("npm:foo@file:./local");
    match s {
        DepSpec::Alias { real_name, inner } => {
            assert_eq!(real_name, "foo");
            // Note: in v1.2 the inner classifier doesn't recurse, so
            // file: inside an alias is a Range string, not a File. The
            // pubgrub resolver treats this as a regular Range root and
            // will probably fail to find the version "file:./local" in
            // the registry. v1.x backlog: recursive classify.
            assert!(matches!(*inner, DepSpec::Range(_)));
        }
        other => panic!("expected Alias, got {other:?}"),
    }
}
