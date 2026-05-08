use guroku::lockfile::Lockfile;
use tempfile::TempDir;

#[test]
fn unknown_top_level_field_ignored() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("guroku.lock.json");
    let json = r#"{
        "lockfileVersion": 1,
        "generatedBy": "guroku 1.1.0",
        "packages": {
            "lodash@4.17.21": {
                "resolved": "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz",
                "integrity": "sha512-abc",
                "dependencies": {}
            }
        },
        "signature": "deadbeef-future-field"
    }"#;
    std::fs::write(&path, json).unwrap();

    let lockfile = Lockfile::read_from(&path).expect("unknown top-level field must be tolerated");
    assert_eq!(lockfile.lockfile_version, 1);
    assert!(lockfile.packages.contains_key("lodash@4.17.21"));
}

#[test]
fn unknown_field_inside_package_ignored() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("guroku.lock.json");
    let json = r#"{
        "lockfileVersion": 1,
        "generatedBy": "guroku 1.1.0",
        "packages": {
            "left-pad@1.3.0": {
                "resolved": "https://registry.npmjs.org/left-pad/-/left-pad-1.3.0.tgz",
                "integrity": "sha512-xyz",
                "dependencies": { "foo": "1.0.0" },
                "extraKey": { "anything": [1, 2, 3] }
            }
        }
    }"#;
    std::fs::write(&path, json).unwrap();

    let lockfile =
        Lockfile::read_from(&path).expect("unknown field inside package must be tolerated");
    let pkg = lockfile
        .packages
        .get("left-pad@1.3.0")
        .expect("left-pad entry present");
    assert_eq!(
        pkg.resolved,
        "https://registry.npmjs.org/left-pad/-/left-pad-1.3.0.tgz"
    );
    assert_eq!(pkg.integrity.as_deref(), Some("sha512-xyz"));
    assert_eq!(
        pkg.dependencies.get("foo").map(String::as_str),
        Some("1.0.0")
    );
}

#[test]
fn extra_array_top_level_field_ignored() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("guroku.lock.json");
    let json = r#"{
        "lockfileVersion": 1,
        "generatedBy": "guroku 1.1.0",
        "packages": {},
        "notes": ["future use"]
    }"#;
    std::fs::write(&path, json).unwrap();

    let lockfile =
        Lockfile::read_from(&path).expect("extra array top-level field must be tolerated");
    assert!(lockfile.packages.is_empty());
}

#[test]
fn multiple_unknown_fields_ignored() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("guroku.lock.json");
    let json = r#"{
        "lockfileVersion": 1,
        "generatedBy": "guroku 1.1.0",
        "packages": {},
        "signature": "abc",
        "integrity": "sha512-top",
        "metadata": { "tool": "v2" }
    }"#;
    std::fs::write(&path, json).unwrap();

    let lockfile =
        Lockfile::read_from(&path).expect("multiple unknown top-level fields must be tolerated");
    assert_eq!(lockfile.lockfile_version, 1);
    assert!(lockfile.packages.is_empty());
}
