//! v1.0 lockfile schema compatibility tests.
//!
//! `lockfileVersion: 1` is SemVer-stable as of guroku 1.0. Any future change
//! that breaks parsing of these byte-for-byte literals MUST fail this suite
//! loudly so it cannot ship in a minor release.

use guroku::lockfile::{Lockfile, LOCKFILE_VERSION};
use std::fs;
use tempfile::TempDir;

fn write_lock(dir: &TempDir, json: &str) -> std::path::PathBuf {
    let path = dir.path().join("guroku.lock");
    fs::write(&path, json).unwrap();
    path
}

#[test]
fn parses_minimal_v1_lockfile() {
    let dir = TempDir::new().unwrap();
    let json = r#"{"lockfileVersion":1,"generatedBy":"guroku 1.0.0","packages":{}}"#;
    let path = write_lock(&dir, json);
    let lock = Lockfile::read_from(&path).expect("minimal v1 lockfile must parse");
    assert_eq!(lock.lockfile_version, 1);
    assert!(lock.packages.is_empty());
}

#[test]
fn parses_lockfile_with_one_package() {
    let dir = TempDir::new().unwrap();
    let json = r#"{
        "lockfileVersion": 1,
        "generatedBy": "guroku 1.0.0",
        "packages": {
            "lodash@4.17.21": {
                "resolved": "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz",
                "integrity": "sha512-deadbeef",
                "dependencies": {}
            }
        }
    }"#;
    let path = write_lock(&dir, json);
    let lock = Lockfile::read_from(&path).expect("single-package v1 lockfile must parse");
    assert!(lock.packages.contains_key("lodash@4.17.21"));
}

#[test]
fn parses_lockfile_with_transitive_chain() {
    let dir = TempDir::new().unwrap();
    let json = r#"{
        "lockfileVersion": 1,
        "generatedBy": "guroku 1.0.0",
        "packages": {
            "a@1.0.0": {
                "resolved": "https://example.invalid/a-1.0.0.tgz",
                "integrity": "sha512-aaa",
                "dependencies": { "b": "1.0.0" }
            },
            "b@1.0.0": {
                "resolved": "https://example.invalid/b-1.0.0.tgz",
                "integrity": "sha512-bbb",
                "dependencies": { "c": "1.0.0" }
            },
            "c@1.0.0": {
                "resolved": "https://example.invalid/c-1.0.0.tgz",
                "integrity": "sha512-ccc",
                "dependencies": {}
            }
        }
    }"#;
    let path = write_lock(&dir, json);
    let lock = Lockfile::read_from(&path).expect("transitive-chain v1 lockfile must parse");
    assert!(lock.packages.contains_key("a@1.0.0"));
    assert!(lock.packages.contains_key("b@1.0.0"));
    assert!(lock.packages.contains_key("c@1.0.0"));
}

#[test]
fn parses_lockfile_with_integrity_omitted() {
    let dir = TempDir::new().unwrap();
    let json = r#"{
        "lockfileVersion": 1,
        "generatedBy": "guroku 1.0.0",
        "packages": {
            "no-integrity@0.0.1": {
                "resolved": "https://example.invalid/no-integrity-0.0.1.tgz",
                "dependencies": {}
            }
        }
    }"#;
    let path = write_lock(&dir, json);
    let lock = Lockfile::read_from(&path).expect("entry without integrity must parse");
    let entry = lock
        .packages
        .get("no-integrity@0.0.1")
        .expect("entry should be present");
    assert!(entry.integrity.is_none());
}

#[test]
fn parses_lockfile_with_dependencies_omitted() {
    let dir = TempDir::new().unwrap();
    let json = r#"{
        "lockfileVersion": 1,
        "generatedBy": "guroku 1.0.0",
        "packages": {
            "leaf@2.0.0": {
                "resolved": "https://example.invalid/leaf-2.0.0.tgz",
                "integrity": "sha512-leaf"
            }
        }
    }"#;
    let path = write_lock(&dir, json);
    let lock = Lockfile::read_from(&path).expect("entry without dependencies must parse");
    let entry = lock
        .packages
        .get("leaf@2.0.0")
        .expect("entry should be present");
    assert!(entry.dependencies.is_empty());
}

#[test]
fn lockfile_version_is_one_in_freshly_constructed() {
    assert_eq!(Lockfile::new().lockfile_version, 1);
    assert_eq!(LOCKFILE_VERSION, 1);
}

#[test]
fn lockfile_version_is_one_after_round_trip() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("guroku.lock");
    let original = Lockfile::new();
    original.write_to(&path).expect("write should succeed");
    let parsed = Lockfile::read_from(&path).expect("round-trip read should succeed");
    assert_eq!(parsed.lockfile_version, 1);
}
