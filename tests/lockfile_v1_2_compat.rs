//! v1.2 lockfile compatibility tests.
//!
//! `lockfileVersion: 1` was frozen in guroku 1.0 and is unchanged in 1.1 and
//! 1.2. v1.2's pubgrub-based default resolver MUST still produce lockfiles
//! that are bit-compatible with v1.0/v1.1 readers, and v1.2 itself MUST still
//! read the v1.0 baseline fixtures without modification.

use guroku::lockfile::Lockfile;
use std::path::PathBuf;

fn fixture(name: &str) -> Lockfile {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/fixtures");
    p.push(name);
    Lockfile::read_from(&p).unwrap_or_else(|e| panic!("read {name}: {e}"))
}

#[test]
fn v1_minimal_lockfile_still_reads() {
    let lock = fixture("v1_minimal_lockfile.json");
    assert_eq!(lock.lockfile_version, 1);
}

#[test]
fn v1_realistic_lockfile_still_reads() {
    let lock = fixture("v1_realistic_lockfile.json");
    assert_eq!(lock.lockfile_version, 1);
    let mut count = 0;
    for _ in lock.packages.iter() {
        count += 1;
    }
    assert!(
        count >= 1,
        "realistic fixture must have at least one package entry"
    );
}

#[test]
fn lockfile_version_constant_unchanged() {
    assert_eq!(guroku::lockfile::LOCKFILE_VERSION, 1);
}

#[test]
fn lockfile_name_constant_unchanged() {
    assert_eq!(guroku::lockfile::LOCKFILE_NAME, "guroku.lock");
}

#[test]
fn round_trip_preserves_version_one() {
    use guroku::lockfile::Lockfile;
    use std::path::PathBuf;
    use tempfile::TempDir;
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/fixtures/v1_minimal_lockfile.json");
    let lock = Lockfile::read_from(&p).expect("read minimal");
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("guroku.lock");
    lock.write_to(&out).expect("write");
    let round = Lockfile::read_from(&out).expect("re-read");
    assert_eq!(round.lockfile_version, 1);
}
