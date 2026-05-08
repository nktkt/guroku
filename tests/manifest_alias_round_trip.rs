//! v1.1 manifest alias round-trip: aliases like `"my-lodash":"npm:lodash@^4.17"`
//! are stored as plain (key, spec) pairs. The manifest layer is unaware of the
//! alias semantics — `classify(spec)` is what later recognises them. These
//! tests pin that the read/write pipeline preserves the spec string verbatim.

use std::fs;

use guroku::manifest::Manifest;
use tempfile::TempDir;

const ALIAS_FIXTURE: &str =
    r#"{"name":"x","version":"1","dependencies":{"my-lodash":"npm:lodash@^4.17"}}"#;

#[test]
fn parses_alias_dependency() {
    let tmp = TempDir::new().expect("tempdir");
    let p = tmp.path().join("package.json");
    fs::write(&p, ALIAS_FIXTURE).expect("seed");

    let m = Manifest::read_from(&p).expect("read");
    assert_eq!(
        m.dependencies.get("my-lodash"),
        Some(&"npm:lodash@^4.17".to_string())
    );
}

#[test]
fn alias_persists_through_round_trip() {
    let tmp = TempDir::new().expect("tempdir");
    let p1 = tmp.path().join("package.json");
    let p2 = tmp.path().join("package.out.json");
    fs::write(&p1, ALIAS_FIXTURE).expect("seed");

    let first = Manifest::read_from(&p1).expect("read");
    first.write_to(&p2).expect("write");
    let second = Manifest::read_from(&p2).expect("re-read");

    assert_eq!(
        second.dependencies.get("my-lodash"),
        Some(&"npm:lodash@^4.17".to_string())
    );
    assert_eq!(first.dependencies, second.dependencies);
}

#[test]
fn multiple_aliases_round_trip() {
    let tmp = TempDir::new().expect("tempdir");
    let p1 = tmp.path().join("package.json");
    let p2 = tmp.path().join("package.out.json");
    fs::write(
        &p1,
        r#"{
            "name":"x","version":"1",
            "dependencies":{
                "my-lodash":"npm:lodash@^4.17",
                "my-react":"npm:react@^18.2.0",
                "express":"^4.18.0"
            }
        }"#,
    )
    .expect("seed");

    let first = Manifest::read_from(&p1).expect("read");
    assert_eq!(
        first.dependencies.get("my-lodash"),
        Some(&"npm:lodash@^4.17".to_string())
    );
    assert_eq!(
        first.dependencies.get("my-react"),
        Some(&"npm:react@^18.2.0".to_string())
    );
    assert_eq!(
        first.dependencies.get("express"),
        Some(&"^4.18.0".to_string())
    );

    first.write_to(&p2).expect("write");
    let second = Manifest::read_from(&p2).expect("re-read");

    assert_eq!(first.dependencies, second.dependencies);
    assert_eq!(
        second.dependencies.get("my-lodash"),
        Some(&"npm:lodash@^4.17".to_string())
    );
    assert_eq!(
        second.dependencies.get("my-react"),
        Some(&"npm:react@^18.2.0".to_string())
    );
    assert_eq!(
        second.dependencies.get("express"),
        Some(&"^4.18.0".to_string())
    );
}

#[test]
fn mixing_alias_with_regular_deps() {
    let tmp = TempDir::new().expect("tempdir");
    let p1 = tmp.path().join("package.json");
    let p2 = tmp.path().join("package.out.json");
    fs::write(
        &p1,
        r#"{
            "name":"x","version":"1",
            "dependencies":{
                "lodash":"^4",
                "my-fork":"npm:lodash@4.17.21"
            }
        }"#,
    )
    .expect("seed");

    let first = Manifest::read_from(&p1).expect("read");
    assert_eq!(first.dependencies.get("lodash"), Some(&"^4".to_string()));
    assert_eq!(
        first.dependencies.get("my-fork"),
        Some(&"npm:lodash@4.17.21".to_string())
    );

    first.write_to(&p2).expect("write");
    let second = Manifest::read_from(&p2).expect("re-read");

    assert_eq!(second.dependencies.get("lodash"), Some(&"^4".to_string()));
    assert_eq!(
        second.dependencies.get("my-fork"),
        Some(&"npm:lodash@4.17.21".to_string())
    );
    assert_eq!(first.dependencies, second.dependencies);
}

#[test]
fn serialised_json_uses_npm_prefix() {
    let tmp = TempDir::new().expect("tempdir");
    let out = tmp.path().join("package.json");

    let mut m = Manifest {
        name: Some("x".into()),
        version: Some("1".into()),
        ..Manifest::default()
    };
    m.add_dependency("my-lodash", "npm:lodash@^4.17");

    m.write_to(&out).expect("write");
    let raw = fs::read_to_string(&out).expect("read string");

    assert!(
        raw.contains(r#""npm:lodash@^4""#) || raw.contains(r#""npm:lodash@^4.17""#),
        "serialised JSON should contain the npm: prefix verbatim, got:\n{raw}"
    );
    assert!(raw.contains("npm:lodash@^4"), "got:\n{raw}");
}
