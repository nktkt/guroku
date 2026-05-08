use guroku::specs::{validate, DepSpec, GitRef};

#[test]
fn validate_range_passes() {
    assert!(validate(&DepSpec::Range("^1.2.3".into())).is_ok());
}

#[test]
fn validate_file_passes() {
    assert!(validate(&DepSpec::File("./pkg".into())).is_ok());
}

#[test]
fn validate_git_passes() {
    assert!(validate(&DepSpec::Git(GitRef {
        url: "https://x/r.git".into(),
        revision: None,
    }))
    .is_ok());
}

#[test]
fn validate_git_with_revision_passes() {
    assert!(validate(&DepSpec::Git(GitRef {
        url: "https://x/r.git".into(),
        revision: Some("v1".into()),
    }))
    .is_ok());
}

#[test]
fn validate_empty_range_passes() {
    assert!(validate(&DepSpec::Range("".into())).is_ok());
}
