use guroku::error::GurokuError;
use guroku::lockfile::Lockfile;
use tempfile::TempDir;

#[test]
fn reads_well_formed_lockfile() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("guroku.lock.json");
    let json = r#"{
        "lockfileVersion": 1,
        "generatedBy": "guroku 0.2.0",
        "packages": {
            "lodash@4.17.21": {
                "resolved": "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz",
                "integrity": "sha512-abc",
                "dependencies": {}
            }
        }
    }"#;
    std::fs::write(&path, json).unwrap();

    let lockfile = Lockfile::read_from(&path).expect("read lockfile");
    assert_eq!(lockfile.lockfile_version, 1);
    let pkg = lockfile
        .packages
        .get("lodash@4.17.21")
        .expect("lodash entry present");
    assert_eq!(
        pkg.resolved,
        "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz"
    );
}

#[test]
fn handles_missing_integrity_field() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("guroku.lock.json");
    let json = r#"{
        "lockfileVersion": 1,
        "generatedBy": "guroku 0.2.0",
        "packages": {
            "left-pad@1.3.0": {
                "resolved": "https://registry.npmjs.org/left-pad/-/left-pad-1.3.0.tgz",
                "dependencies": {}
            }
        }
    }"#;
    std::fs::write(&path, json).unwrap();

    let lockfile = Lockfile::read_from(&path).expect("read lockfile");
    let pkg = lockfile
        .packages
        .get("left-pad@1.3.0")
        .expect("left-pad entry present");
    assert!(pkg.integrity.is_none());
}

#[test]
fn handles_empty_packages() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("guroku.lock.json");
    let json = r#"{"lockfileVersion": 1, "generatedBy": "guroku 0.2.0"}"#;
    std::fs::write(&path, json).unwrap();

    let lockfile = Lockfile::read_from(&path).expect("read lockfile");
    assert!(lockfile.packages.is_empty());
}

#[test]
fn rejects_unsupported_version() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("guroku.lock.json");
    let json = r#"{
        "lockfileVersion": 999,
        "generatedBy": "guroku 0.2.0",
        "packages": {}
    }"#;
    std::fs::write(&path, json).unwrap();

    let err = Lockfile::read_from(&path).expect_err("should reject version");
    assert!(matches!(
        err,
        GurokuError::LockfileVersionMismatch {
            found: 999,
            expected: 1
        }
    ));
}

#[test]
fn rejects_malformed_json() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("guroku.lock.json");
    std::fs::write(&path, "not { valid json at all ::::").unwrap();

    let result = Lockfile::read_from(&path);
    assert!(result.is_err());
}
