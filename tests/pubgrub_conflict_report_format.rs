//! Contract tests for the format of `GurokuError::ResolutionConflict`.
//!
//! In v1.2 the `requested_by` field carries two different shapes:
//!   * BFS-resolver conflicts: an `"a > b > c"`-style dependency path.
//!   * pubgrub-produced conflicts: pubgrub's `DefaultStringReporter::report`
//!     output, which is a multi-line ASCII derivation tree.
//!
//! These tests pin down the externally-visible contract so both shapes keep
//! round-tripping through `Display`.

#[test]
fn bfs_path_format_unchanged() {
    use guroku::error::GurokuError;
    let err = GurokuError::ResolutionConflict {
        name: "is-number".into(),
        chosen: "5.0.0".into(),
        requested: "^6".into(),
        requested_by: "is-odd > is-number".into(),
    };
    let s = format!("{err}");
    assert!(s.contains("is-number"));
    assert!(s.contains("5.0.0"));
    assert!(s.contains("^6"));
    assert!(s.contains("is-odd > is-number"));
}

#[test]
fn pubgrub_multiline_report_round_trips() {
    use guroku::error::GurokuError;
    let report = "Because lib-a@1 depends on core@>=2.5,
and lib-b@1 depends on core@<2.5,
lib-a@1 and lib-b@1 are incompatible.";
    let err = GurokuError::ResolutionConflict {
        name: "<resolver>".into(),
        chosen: "<unsolvable>".into(),
        requested: "<see report>".into(),
        requested_by: report.into(),
    };
    let s = format!("{err}");
    assert!(s.contains("Because lib-a@1"));
    assert!(s.contains("incompatible"));
    assert!(s.contains("<resolver>"));
}

#[test]
fn requested_by_no_unprintable_chars() {
    let s = "is-odd > is-number";
    for ch in s.chars() {
        assert!(
            ch.is_ascii_graphic() || ch == ' ',
            "non-printable in path: {ch:?}"
        );
    }
}

#[test]
fn resolution_conflict_implements_std_error() {
    use guroku::error::GurokuError;
    let error = GurokuError::ResolutionConflict {
        name: "x".into(),
        chosen: "1.0.0".into(),
        requested: "^2".into(),
        requested_by: "root > x".into(),
    };
    let _: &dyn std::error::Error = &error;
}

#[test]
fn pubgrub_report_can_contain_arrows() {
    // pubgrub's `DefaultStringReporter::report` output uses only ASCII
    // characters (letters, digits, punctuation, spaces, newlines). We don't
    // need to escape anything special when stuffing it into `requested_by`.
    let report = "root 1.0.0 -> a ^1 -> b ^1 -> core <2.5\n\
                  root 1.0.0 -> c ^1 -> core >=2.5\n\
                  => no version of core satisfies both ranges.";
    for ch in report.chars() {
        assert!(
            ch.is_ascii(),
            "pubgrub report unexpectedly contains non-ASCII char: {ch:?}"
        );
        assert!(
            ch.is_ascii_graphic() || ch == ' ' || ch == '\n',
            "pubgrub report contains unprintable ASCII: {ch:?}"
        );
    }

    // And it survives the Display round-trip unchanged in substance.
    use guroku::error::GurokuError;
    let err = GurokuError::ResolutionConflict {
        name: "<resolver>".into(),
        chosen: "<unsolvable>".into(),
        requested: "<see report>".into(),
        requested_by: report.into(),
    };
    let s = format!("{err}");
    assert!(s.contains("-> core <2.5"));
    assert!(s.contains("=> no version of core"));
}
