#![cfg(unix)]

use guroku::error::GurokuError;
use guroku::scripts::run_in;
use tempfile::TempDir;

#[test]
fn nonzero_exit_returns_script_failed() {
    let tmp = TempDir::new().unwrap();
    let err = run_in(tmp.path(), "build", "exit 7", &[]).unwrap_err();
    assert!(matches!(err, GurokuError::ScriptFailed { status: 7, .. }));
}

#[test]
fn false_returns_script_failed_status_one() {
    let tmp = TempDir::new().unwrap();
    let err = run_in(tmp.path(), "check", "false", &[]).unwrap_err();
    assert!(matches!(err, GurokuError::ScriptFailed { status: 1, .. }));
    if let GurokuError::ScriptFailed { status, .. } = err {
        assert_eq!(status, 1);
    } else {
        panic!("expected ScriptFailed");
    }
}

#[test]
fn script_name_propagates_in_error() {
    let tmp = TempDir::new().unwrap();
    let err = run_in(tmp.path(), "my-build", "exit 1", &[]).unwrap_err();
    assert!(matches!(err, GurokuError::ScriptFailed { .. }));
    if let GurokuError::ScriptFailed { script, status } = err {
        assert_eq!(script, "my-build");
        assert_eq!(status, 1);
    } else {
        panic!("expected ScriptFailed");
    }
}

#[test]
fn display_string_mentions_script_and_status() {
    let tmp = TempDir::new().unwrap();
    let err = run_in(tmp.path(), "my-build", "exit 7", &[]).unwrap_err();
    let msg = format!("{}", err);
    assert!(
        msg.contains("my-build"),
        "expected display to contain script name, got: {msg}"
    );
    assert!(
        msg.contains("7"),
        "expected display to contain status code, got: {msg}"
    );
}
