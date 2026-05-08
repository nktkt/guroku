use guroku::specs::{classify, unparse, DepSpec, GitRef};

#[test]
fn unparse_range() {
    assert_eq!(unparse(&DepSpec::Range("^1.2.3".into())), "^1.2.3");
}

#[test]
fn unparse_file() {
    assert_eq!(unparse(&DepSpec::File("./pkg".into())), "file:./pkg");
}

#[test]
fn unparse_git_no_revision() {
    let spec = DepSpec::Git(GitRef {
        url: "https://x/r.git".into(),
        revision: None,
    });
    assert_eq!(unparse(&spec), "git+https://x/r.git");
}

#[test]
fn unparse_git_with_revision() {
    let spec = DepSpec::Git(GitRef {
        url: "https://x/r.git".into(),
        revision: Some("v1".into()),
    });
    assert_eq!(unparse(&spec), "git+https://x/r.git#v1");
}

#[test]
fn range_round_trips() {
    let original = DepSpec::Range("^2.0.0".into());
    assert_eq!(classify(&unparse(&original)), original);
}

#[test]
fn file_round_trips() {
    let original = DepSpec::File("../local".into());
    assert_eq!(classify(&unparse(&original)), original);
}

#[test]
fn git_round_trips() {
    let original = DepSpec::Git(GitRef {
        url: "https://x/r.git".into(),
        revision: Some("main".into()),
    });
    assert_eq!(classify(&unparse(&original)), original);
}
