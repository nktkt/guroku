use guroku::specs::{classify, DepSpec, GitRef};

#[test]
fn range_is_default() {
    assert!(matches!(classify("^1.2.3"), DepSpec::Range(r) if r == "^1.2.3"));
}

#[test]
fn range_keeps_dist_tag() {
    assert!(matches!(classify("latest"), DepSpec::Range(r) if r == "latest"));
}

#[test]
fn file_strips_prefix() {
    assert!(matches!(classify("file:./pkg"), DepSpec::File(p) if p == "./pkg"));
}

#[test]
fn file_with_relative_parent() {
    assert!(matches!(classify("file:../pkg"), DepSpec::File(p) if p == "../pkg"));
}

#[test]
fn git_https_strips_git_plus() {
    match classify("git+https://github.com/u/r.git") {
        DepSpec::Git(GitRef { url, revision }) => {
            assert_eq!(url, "https://github.com/u/r.git");
            assert!(revision.is_none());
        }
        other => panic!("expected Git, got {:?}", other),
    }
}

#[test]
fn git_ssh_strips_git_plus() {
    match classify("git+ssh://git@host/r.git") {
        DepSpec::Git(GitRef { url, revision }) => {
            assert_eq!(url, "ssh://git@host/r.git");
            assert!(revision.is_none());
        }
        other => panic!("expected Git, got {:?}", other),
    }
}

#[test]
fn github_shorthand_expands_to_https() {
    match classify("github:user/repo") {
        DepSpec::Git(GitRef { url, revision }) => {
            assert_eq!(url, "https://github.com/user/repo");
            assert!(revision.is_none());
        }
        other => panic!("expected Git, got {:?}", other),
    }
}

#[test]
fn bare_git_url_recognised() {
    assert!(matches!(
        classify("git://example.com/u/r.git"),
        DepSpec::Git(_)
    ));
}

#[test]
fn whitespace_trimmed() {
    assert!(matches!(classify("  ^1.0.0  "), DepSpec::Range(r) if r == "^1.0.0"));
}
