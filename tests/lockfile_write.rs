use std::collections::BTreeMap;
use std::fs;

use guroku::lockfile::{Lockfile, PackageLock};
use tempfile::TempDir;

fn sample_pkg(resolved: &str, integrity: Option<&str>) -> PackageLock {
    PackageLock {
        resolved: resolved.to_string(),
        integrity: integrity.map(|s| s.to_string()),
        dependencies: BTreeMap::new(),
    }
}

#[test]
fn writes_top_level_keys() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("guroku.lock.json");

    let mut lock = Lockfile::new();
    lock.insert(
        "lodash",
        "4.17.21",
        sample_pkg(
            "https://registry.example.com/lodash/-/lodash-4.17.21.tgz",
            Some("sha512-aaa"),
        ),
    );
    lock.insert(
        "react",
        "18.3.1",
        sample_pkg(
            "https://registry.example.com/react/-/react-18.3.1.tgz",
            Some("sha512-bbb"),
        ),
    );

    lock.write_to(&path).unwrap();

    let text = fs::read_to_string(&path).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(v["lockfileVersion"], 1);
    let gen = v["generatedBy"].as_str().expect("generatedBy is a string");
    assert!(
        gen.starts_with("guroku "),
        "generatedBy should start with 'guroku ', got {gen:?}"
    );

    let packages = v["packages"].as_object().expect("packages is an object");
    assert!(packages.contains_key("lodash@4.17.21"));
    assert!(packages.contains_key("react@18.3.1"));
}

#[test]
fn writes_packages_with_resolved_and_integrity() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("guroku.lock.json");

    let url = "https://registry.example.com/lodash/-/lodash-4.17.21.tgz";
    let integrity = "sha512-deadbeef";

    let mut lock = Lockfile::new();
    lock.insert("lodash", "4.17.21", sample_pkg(url, Some(integrity)));
    lock.write_to(&path).unwrap();

    let text = fs::read_to_string(&path).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();

    let entry = &v["packages"]["lodash@4.17.21"];
    assert_eq!(entry["resolved"], url);
    assert_eq!(entry["integrity"], integrity);
}

#[test]
fn omits_integrity_when_none() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("guroku.lock.json");

    let mut lock = Lockfile::new();
    lock.insert(
        "left-pad",
        "1.3.0",
        sample_pkg(
            "https://registry.example.com/left-pad/-/left-pad-1.3.0.tgz",
            None,
        ),
    );
    lock.write_to(&path).unwrap();

    let text = fs::read_to_string(&path).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();

    let entry = v["packages"]["left-pad@1.3.0"]
        .as_object()
        .expect("package entry is an object");
    assert!(
        !entry.contains_key("integrity"),
        "integrity should be omitted when None, got: {entry:?}"
    );
}

#[test]
fn output_ends_with_newline() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("guroku.lock.json");

    let mut lock = Lockfile::new();
    lock.insert(
        "react",
        "18.3.1",
        sample_pkg(
            "https://registry.example.com/react/-/react-18.3.1.tgz",
            None,
        ),
    );
    lock.write_to(&path).unwrap();

    let text = fs::read_to_string(&path).unwrap();
    assert!(
        text.ends_with('\n'),
        "lockfile output must end with newline"
    );
}

#[test]
fn dependencies_field_round_trip() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("guroku.lock.json");

    let mut deps = BTreeMap::new();
    deps.insert("a".to_string(), "1.0.0".to_string());
    deps.insert("b".to_string(), "2.0.0".to_string());

    let pkg = PackageLock {
        resolved: "https://registry.example.com/pkg/-/pkg-1.0.0.tgz".to_string(),
        integrity: None,
        dependencies: deps,
    };

    let mut lock = Lockfile::new();
    lock.insert("pkg", "1.0.0", pkg);
    lock.write_to(&path).unwrap();

    let text = fs::read_to_string(&path).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();

    let deps_out = v["packages"]["pkg@1.0.0"]["dependencies"]
        .as_object()
        .expect("dependencies is an object");
    assert!(deps_out.contains_key("a"));
    assert!(deps_out.contains_key("b"));
    assert_eq!(deps_out["a"], "1.0.0");
    assert_eq!(deps_out["b"], "2.0.0");
}
