use guroku::error::GurokuError;

fn make(requested_by: &str) -> GurokuError {
    GurokuError::ResolutionConflict {
        name: "left-pad".to_string(),
        chosen: "1.2.3".to_string(),
        requested: "^2.0.0".to_string(),
        requested_by: requested_by.to_string(),
    }
}

#[test]
fn display_includes_path_separator() {
    let err = make("a > b > c");
    let s = format!("{}", err);
    assert!(s.contains("a > b > c"), "missing path in: {}", s);
}

#[test]
fn display_includes_all_other_fields() {
    let err = make("a > b > c");
    let s = format!("{}", err);
    assert!(s.contains("left-pad"), "missing package name in: {}", s);
    assert!(s.contains("1.2.3"), "missing chosen version in: {}", s);
    assert!(s.contains("^2.0.0"), "missing requested range in: {}", s);
}

#[test]
fn path_with_root_marker() {
    let err = make("<root>");
    let s = format!("{}", err);
    assert!(s.contains("<root>"), "missing root marker in: {}", s);
}

#[test]
fn path_format_uses_arrow_separator() {
    let err = make("a > b");
    let s = format!("{}", err);
    assert!(s.contains("a > b"), "expected '>' separator in: {}", s);
    assert!(!s.contains("a/b"), "should not use '/' separator in: {}", s);
    assert!(
        !s.contains("a -> b"),
        "should not use '->' separator in: {}",
        s
    );
}

#[test]
fn error_kind_classification_works_with_path() {
    fn pass_through(e: GurokuError) -> guroku::Result<()> {
        Err(e)
    }
    let err = make("root > app > dep");
    let result = pass_through(err);
    match result {
        Err(GurokuError::ResolutionConflict { requested_by, .. }) => {
            assert_eq!(requested_by, "root > app > dep");
        }
        _ => panic!("expected ResolutionConflict variant"),
    }
}
