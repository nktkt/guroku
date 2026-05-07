//! Round-trip tests for `guroku::lockfile::Lockfile` through disk.

use std::collections::BTreeMap;

use guroku::lockfile::{Lockfile, PackageLock};
use tempfile::TempDir;

fn lock_path(dir: &TempDir) -> std::path::PathBuf {
    dir.path().join("guroku.lock")
}

#[test]
fn roundtrip_two_packages() {
    let tmp = TempDir::new().expect("create tempdir");
    let path = lock_path(&tmp);

    let mut lock = Lockfile::new();
    lock.insert(
        "lodash",
        "4.17.21",
        PackageLock {
            resolved: "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz".into(),
            integrity: Some("sha512-abc".into()),
            dependencies: BTreeMap::new(),
        },
    );
    lock.insert(
        "leftpad",
        "1.0.0",
        PackageLock {
            resolved: "https://registry.npmjs.org/leftpad/-/leftpad-1.0.0.tgz".into(),
            integrity: None,
            dependencies: BTreeMap::new(),
        },
    );

    lock.write_to(&path).expect("write lockfile");
    let read_back = Lockfile::read_from(&path).expect("read lockfile");

    let orig_keys: Vec<&String> = lock.packages.keys().collect();
    let read_keys: Vec<&String> = read_back.packages.keys().collect();
    assert_eq!(orig_keys, read_keys, "keys differ");

    for (key, orig) in &lock.packages {
        let got = read_back.packages.get(key).expect("missing key after read");
        assert_eq!(got.resolved, orig.resolved);
        assert_eq!(got.integrity, orig.integrity);
        assert_eq!(got.dependencies, orig.dependencies);
    }
}

#[test]
fn roundtrip_with_transitive_deps() {
    let tmp = TempDir::new().expect("create tempdir");
    let path = lock_path(&tmp);

    let mut deps = BTreeMap::new();
    deps.insert("react".to_string(), "18.3.1".to_string());
    deps.insert("scheduler".to_string(), "0.23.0".to_string());

    let mut lock = Lockfile::new();
    lock.insert(
        "react-dom",
        "18.3.1",
        PackageLock {
            resolved: "https://registry.npmjs.org/react-dom/-/react-dom-18.3.1.tgz".into(),
            integrity: Some("sha512-deadbeef".into()),
            dependencies: deps.clone(),
        },
    );

    lock.write_to(&path).expect("write lockfile");
    let read_back = Lockfile::read_from(&path).expect("read lockfile");

    let entry = read_back
        .packages
        .get("react-dom@18.3.1")
        .expect("entry present");
    assert_eq!(entry.dependencies, deps);
    assert_eq!(
        entry.dependencies.get("react").map(String::as_str),
        Some("18.3.1")
    );
    assert_eq!(
        entry.dependencies.get("scheduler").map(String::as_str),
        Some("0.23.0")
    );
}

#[test]
fn roundtrip_preserves_lockfile_version() {
    let tmp = TempDir::new().expect("create tempdir");
    let path = lock_path(&tmp);

    let lock = Lockfile::new();
    let original_version = lock.lockfile_version;

    lock.write_to(&path).expect("write lockfile");
    let read_back = Lockfile::read_from(&path).expect("read lockfile");

    assert_eq!(read_back.lockfile_version, original_version);
}

#[test]
fn roundtrip_preserves_generated_by() {
    let tmp = TempDir::new().expect("create tempdir");
    let path = lock_path(&tmp);

    let mut lock = Lockfile::new();
    lock.generated_by = "guroku-test/9.9.9".to_string();

    lock.write_to(&path).expect("write lockfile");
    let read_back = Lockfile::read_from(&path).expect("read lockfile");

    assert_eq!(read_back.generated_by, "guroku-test/9.9.9");
}

#[test]
fn roundtrip_empty_packages() {
    let tmp = TempDir::new().expect("create tempdir");
    let path = lock_path(&tmp);

    let lock = Lockfile::new();
    assert!(
        lock.packages.is_empty(),
        "precondition: new lockfile has no packages"
    );

    lock.write_to(&path).expect("write lockfile");
    let read_back = Lockfile::read_from(&path).expect("read lockfile");

    assert!(
        read_back.packages.is_empty(),
        "packages should remain empty"
    );
    let _: &BTreeMap<String, PackageLock> = &read_back.packages;
}
