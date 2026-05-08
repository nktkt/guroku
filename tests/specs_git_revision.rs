use guroku::specs::{classify, DepSpec, GitRef};

fn expect_git(spec: DepSpec) -> GitRef {
    match spec {
        DepSpec::Git(g) => g,
        other => panic!("expected Git, got {other:?}"),
    }
}

#[test]
fn revision_extracted_from_hash_suffix() {
    let g = expect_git(classify("git+https://github.com/u/r.git#v1.2.3"));
    assert_eq!(g.url, "https://github.com/u/r.git");
    assert_eq!(g.revision, Some("v1.2.3".to_string()));
}

#[test]
fn github_shorthand_with_branch() {
    let g = expect_git(classify("github:u/r#main"));
    assert_eq!(g.url, "https://github.com/u/r");
    assert_eq!(g.revision, Some("main".to_string()));
}

#[test]
fn no_hash_no_revision() {
    let g = expect_git(classify("git+https://github.com/u/r.git"));
    assert!(g.revision.is_none());
}

#[test]
fn slash_after_hash_means_no_revision() {
    let g = expect_git(classify("git+https://example.com/p?foo=bar/baz"));
    assert!(g.revision.is_none());
}

#[test]
fn commit_sha_revision() {
    let g = expect_git(classify("git+https://github.com/u/r.git#abc1234"));
    assert_eq!(g.revision, Some("abc1234".to_string()));
}

#[test]
fn empty_hash_falls_back_to_no_revision() {
    let g = expect_git(classify("git+https://github.com/u/r.git#"));
    assert!(g.revision.is_none());
}
