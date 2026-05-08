use guroku::specs::{classify, DepSpec, GitRef};

#[test]
fn empty_string_is_range() {
    assert!(matches!(classify(""), DepSpec::Range(r) if r.is_empty()));
}

#[test]
fn dist_tag_strings_are_range() {
    assert!(matches!(classify("latest"), DepSpec::Range(_)));
    assert!(matches!(classify("next"), DepSpec::Range(_)));
}

#[test]
fn pre_release_range_is_range() {
    assert!(matches!(classify("^1.0.0-beta.1"), DepSpec::Range(r) if r == "^1.0.0-beta.1"));
}

#[test]
fn range_with_or_is_range() {
    assert!(matches!(classify("^1 || ^2"), DepSpec::Range(r) if r == "^1 || ^2"));
}

#[test]
fn git_url_with_dot_git_suffix() {
    match classify("git+https://github.com/u/r.git") {
        DepSpec::Git(GitRef { url, revision }) => {
            assert_eq!(url, "https://github.com/u/r.git");
            assert!(revision.is_none());
        }
        other => panic!("expected Git, got {:?}", other),
    }
}

#[test]
fn git_url_without_dot_git_suffix() {
    match classify("git+https://github.com/u/r") {
        DepSpec::Git(GitRef { url, revision }) => {
            assert_eq!(url, "https://github.com/u/r");
            assert!(revision.is_none());
        }
        other => panic!("expected Git, got {:?}", other),
    }
}

#[test]
fn github_shorthand_with_user_dot_git_repo() {
    match classify("github:u/r.js") {
        DepSpec::Git(GitRef { url, revision }) => {
            assert_eq!(url, "https://github.com/u/r.js");
            assert!(revision.is_none());
        }
        other => panic!("expected Git, got {:?}", other),
    }
}

#[test]
fn git_at_host_form() {
    match classify("git@github.com:u/r.git") {
        DepSpec::Git(GitRef { url, .. }) => {
            assert_eq!(url, "git@github.com:u/r.git");
        }
        other => panic!("expected Git, got {:?}", other),
    }
}

#[test]
fn file_with_trailing_slash() {
    assert!(matches!(classify("file:./pkg/"), DepSpec::File(p) if p == "./pkg/"));
}

#[test]
fn file_absolute_path() {
    assert!(matches!(classify("file:/tmp/pkg"), DepSpec::File(p) if p == "/tmp/pkg"));
}

#[test]
fn range_starting_with_v_is_range() {
    assert!(matches!(classify("v1.2.3"), DepSpec::Range(r) if r == "v1.2.3"));
}
