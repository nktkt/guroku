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
fn top_level_keys_are_camel_case() {
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
    lock.write_to(&path).unwrap();

    let text = fs::read_to_string(&path).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    let obj = v.as_object().expect("top-level is an object");

    assert!(obj.contains_key("lockfileVersion"));
    assert!(obj.contains_key("generatedBy"));
    assert!(obj.contains_key("packages"));
}

#[test]
fn package_keys_use_at_separator() {
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
    lock.write_to(&path).unwrap();

    let text = fs::read_to_string(&path).unwrap();
    assert!(
        text.contains("\"lodash@4.17.21\""),
        "expected literal key \"lodash@4.17.21\" in lockfile, got:\n{text}"
    );
}

#[test]
fn pretty_printed_json() {
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
    let newlines = text.matches('\n').count();
    assert!(
        newlines >= 5,
        "expected pretty-printed JSON with >= 5 newlines, got {newlines}:\n{text}"
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
    assert!(text.ends_with('\n'), "lockfile must end with newline");
}

#[test]
fn integrity_omitted_when_none_in_serialized_form() {
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
    assert!(
        !text.contains("\"integrity\""),
        "integrity key must be omitted when None, got:\n{text}"
    );
}

#[test]
fn integrity_present_when_some() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("guroku.lock.json");

    let mut lock = Lockfile::new();
    lock.insert(
        "lodash",
        "4.17.21",
        sample_pkg(
            "https://registry.example.com/lodash/-/lodash-4.17.21.tgz",
            Some("sha512-deadbeef"),
        ),
    );
    lock.write_to(&path).unwrap();

    let text = fs::read_to_string(&path).unwrap();
    assert!(
        text.contains("\"integrity\""),
        "integrity key must be present when Some, got:\n{text}"
    );
}

#[test]
fn packages_section_keys_sorted_alphabetically() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("guroku.lock.json");

    let mut lock = Lockfile::new();
    lock.insert(
        "zebra",
        "1.0.0",
        sample_pkg("https://registry.example.com/zebra/-/zebra-1.0.0.tgz", None),
    );
    lock.insert(
        "apple",
        "1.0.0",
        sample_pkg("https://registry.example.com/apple/-/apple-1.0.0.tgz", None),
    );
    lock.insert(
        "moose",
        "1.0.0",
        sample_pkg("https://registry.example.com/moose/-/moose-1.0.0.tgz", None),
    );
    lock.write_to(&path).unwrap();

    let text = fs::read_to_string(&path).unwrap();
    let apple = text.find("apple@").expect("apple@ present");
    let moose = text.find("moose@").expect("moose@ present");
    let zebra = text.find("zebra@").expect("zebra@ present");

    assert!(
        apple < moose && moose < zebra,
        "expected packages sorted alphabetically (apple < moose < zebra), \
         got offsets apple={apple}, moose={moose}, zebra={zebra}"
    );
}
