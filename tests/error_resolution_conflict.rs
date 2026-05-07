use guroku::error::GurokuError;

#[test]
fn resolution_conflict_display_mentions_all_fields() {
    let err = GurokuError::ResolutionConflict {
        name: "left-pad".to_string(),
        chosen: "1.2.3".to_string(),
        requested: "2.0.0".to_string(),
        requested_by: "my-app".to_string(),
    };
    let s = format!("{}", err);
    assert!(s.contains("left-pad"), "missing package name in: {}", s);
    assert!(s.contains("1.2.3"), "missing chosen version in: {}", s);
    assert!(s.contains("2.0.0"), "missing requested version in: {}", s);
    assert!(s.contains("my-app"), "missing requester in: {}", s);
}

#[test]
fn resolution_conflict_display_format_is_useful() {
    let err = GurokuError::ResolutionConflict {
        name: "foo".to_string(),
        chosen: "1.0.0".to_string(),
        requested: "2.0.0".to_string(),
        requested_by: "bar".to_string(),
    };
    let s = format!("{}", err);
    assert!(
        s.contains("version conflict"),
        "missing 'version conflict' in: {}",
        s
    );
    assert!(
        s.contains("already chose"),
        "missing 'already chose' in: {}",
        s
    );
}

#[test]
fn lockfile_version_mismatch_display() {
    let err = GurokuError::LockfileVersionMismatch {
        found: 2,
        expected: 1,
    };
    let s = format!("{}", err);
    assert!(s.contains("v2"), "missing 'v2' in: {}", s);
    assert!(s.contains("v1"), "missing 'v1' in: {}", s);
    assert!(
        s.contains("lockfile version"),
        "missing 'lockfile version' in: {}",
        s
    );
}

#[test]
fn lockfile_out_of_date_display() {
    let err = GurokuError::LockfileOutOfDate;
    let s = format!("{}", err);
    assert!(s.contains("lockfile"), "missing 'lockfile' in: {}", s);
    assert!(s.contains("out of date"), "missing 'out of date' in: {}", s);
}
