use std::fs;

use guroku::error::GurokuError;
use guroku::lockfile::{Lockfile, LOCKFILE_VERSION};
use tempfile::TempDir;

fn write_lockfile_with_version(path: &std::path::Path, version: u32) {
    let body = format!(
        "{{\n  \"lockfileVersion\": {version},\n  \"generatedBy\": \"guroku test\",\n  \"packages\": {{}}\n}}\n"
    );
    fs::write(path, body).unwrap();
}

#[test]
fn rejects_higher_version() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("guroku.lock.json");
    write_lockfile_with_version(&path, 2);

    let err = Lockfile::read_from(&path).unwrap_err();
    assert!(matches!(
        err,
        GurokuError::LockfileVersionMismatch {
            found: 2,
            expected: 1,
        }
    ));
}

#[test]
fn rejects_lower_version() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("guroku.lock.json");
    write_lockfile_with_version(&path, 0);

    let err = Lockfile::read_from(&path).unwrap_err();
    assert!(matches!(
        err,
        GurokuError::LockfileVersionMismatch {
            found: 0,
            expected: 1,
        }
    ));
}

#[test]
fn error_display_contains_useful_info() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("guroku.lock.json");
    write_lockfile_with_version(&path, 7);

    let err = Lockfile::read_from(&path).unwrap_err();
    let (found, expected) = match err {
        GurokuError::LockfileVersionMismatch { found, expected } => (found, expected),
        other => panic!("expected LockfileVersionMismatch, got {other:?}"),
    };
    let msg = GurokuError::LockfileVersionMismatch { found, expected }.to_string();
    assert!(msg.contains(&found.to_string()), "missing found in {msg:?}");
    assert!(
        msg.contains(&expected.to_string()),
        "missing expected in {msg:?}"
    );
    assert!(
        msg.to_lowercase().contains("lockfile"),
        "missing 'lockfile' in {msg:?}"
    );
}

#[test]
fn accepts_current_version() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("guroku.lock.json");
    write_lockfile_with_version(&path, LOCKFILE_VERSION);

    let result = Lockfile::read_from(&path);
    assert!(result.is_ok(), "expected Ok, got {result:?}");
}

#[test]
fn accepts_constant_value() {
    assert_eq!(LOCKFILE_VERSION, 1);
}
