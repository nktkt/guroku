//! v1.0 manifest schema pin: a single comprehensive round-trip exercising
//! every typed field plus arbitrary `other` payload, alongside three smaller
//! shape-specific round-trips (string-form `bin`, object-form `workspaces`,
//! and a numeric unknown field).

use std::fs;

use guroku::manifest::Manifest;
use tempfile::TempDir;

const FULL_FIXTURE: &str = r#"{
  "name": "stable-roundtrip",
  "version": "1.0.0",
  "description": "everything-in-one fixture",
  "license": "MIT",
  "dependencies": { "lodash": "^4.17.0" },
  "devDependencies": { "vitest": "^1.6.0" },
  "peerDependencies": { "react": "^18" },
  "optionalDependencies": { "fsevents": "^2.3.3" },
  "scripts": { "build": "tsc", "test": "vitest" },
  "bin": { "my-cli": "./bin/cli.js" },
  "workspaces": ["packages/*"],
  "overrides": { "ms": "2.1.3" },
  "resolutions": { "left-pad": "1.3.0" },
  "private": true,
  "keywords": ["a","b"]
}"#;

fn assert_full_manifest_shape(m: &Manifest) {
    assert_eq!(m.name.as_deref(), Some("stable-roundtrip"));
    assert_eq!(m.version.as_deref(), Some("1.0.0"));

    assert_eq!(
        m.dependencies.get("lodash").map(String::as_str),
        Some("^4.17.0")
    );
    assert_eq!(
        m.dev_dependencies.get("vitest").map(String::as_str),
        Some("^1.6.0")
    );
    assert_eq!(
        m.peer_dependencies.get("react").map(String::as_str),
        Some("^18")
    );
    assert_eq!(
        m.optional_dependencies.get("fsevents").map(String::as_str),
        Some("^2.3.3"),
    );

    assert_eq!(m.scripts.get("build").map(String::as_str), Some("tsc"));
    assert_eq!(m.scripts.get("test").map(String::as_str), Some("vitest"));

    assert_eq!(
        m.bin_entries(),
        vec![("my-cli".to_string(), "./bin/cli.js".to_string())]
    );
    assert_eq!(m.workspace_globs(), vec!["packages/*".to_string()]);

    assert_eq!(m.overrides.get("ms").map(String::as_str), Some("2.1.3"));
    assert_eq!(
        m.resolutions.get("left-pad").map(String::as_str),
        Some("1.3.0")
    );

    // Unknown-but-preserved fields land in `other`.
    assert_eq!(m.other.get("private"), Some(&serde_json::json!(true)));
    assert_eq!(
        m.other.get("description"),
        Some(&serde_json::json!("everything-in-one fixture"))
    );
    assert_eq!(m.other.get("license"), Some(&serde_json::json!("MIT")));
    assert_eq!(
        m.other.get("keywords"),
        Some(&serde_json::json!(["a", "b"]))
    );
}

#[test]
fn full_manifest_round_trip() {
    let tmp = TempDir::new().expect("tempdir");
    let p1 = tmp.path().join("package.json");
    let p2 = tmp.path().join("package.out.json");

    fs::write(&p1, FULL_FIXTURE).expect("seed fixture");

    let first = Manifest::read_from(&p1).expect("read fixture");
    assert_full_manifest_shape(&first);

    first.write_to(&p2).expect("write");

    let second = Manifest::read_from(&p2).expect("re-read");
    assert_full_manifest_shape(&second);

    // Cross-check structural equality of the typed fields after round-trip.
    assert_eq!(first.name, second.name);
    assert_eq!(first.version, second.version);
    assert_eq!(first.dependencies, second.dependencies);
    assert_eq!(first.dev_dependencies, second.dev_dependencies);
    assert_eq!(first.peer_dependencies, second.peer_dependencies);
    assert_eq!(first.optional_dependencies, second.optional_dependencies);
    assert_eq!(first.scripts, second.scripts);
    assert_eq!(first.bin, second.bin);
    assert_eq!(first.workspaces, second.workspaces);
    assert_eq!(first.overrides, second.overrides);
    assert_eq!(first.resolutions, second.resolutions);
    assert_eq!(first.other, second.other);
}

#[test]
fn bin_string_form_round_trips() {
    let tmp = TempDir::new().expect("tempdir");
    let p1 = tmp.path().join("package.json");
    let p2 = tmp.path().join("package.out.json");

    fs::write(&p1, r#"{"name":"x","bin":"./cli.js"}"#).expect("seed");

    let first = Manifest::read_from(&p1).expect("read");
    assert_eq!(
        first.bin_entries(),
        vec![("x".to_string(), "./cli.js".to_string())]
    );

    first.write_to(&p2).expect("write");
    let second = Manifest::read_from(&p2).expect("re-read");

    assert_eq!(second.name.as_deref(), Some("x"));
    assert_eq!(second.bin, Some(serde_json::json!("./cli.js")));
    assert_eq!(
        second.bin_entries(),
        vec![("x".to_string(), "./cli.js".to_string())]
    );
}

#[test]
fn workspaces_object_form_round_trips() {
    let tmp = TempDir::new().expect("tempdir");
    let p1 = tmp.path().join("package.json");
    let p2 = tmp.path().join("package.out.json");

    fs::write(&p1, r#"{"workspaces":{"packages":["a/*"]}}"#).expect("seed");

    let first = Manifest::read_from(&p1).expect("read");
    assert_eq!(first.workspace_globs(), vec!["a/*".to_string()]);

    first.write_to(&p2).expect("write");
    let second = Manifest::read_from(&p2).expect("re-read");

    assert_eq!(
        second.workspaces,
        Some(serde_json::json!({"packages": ["a/*"]}))
    );
    assert_eq!(second.workspace_globs(), vec!["a/*".to_string()]);
}

#[test]
fn numeric_unknown_field_round_trips() {
    let tmp = TempDir::new().expect("tempdir");
    let p1 = tmp.path().join("package.json");
    let p2 = tmp.path().join("package.out.json");

    fs::write(&p1, r#"{"keywords":[1,2,3]}"#).expect("seed");

    let first = Manifest::read_from(&p1).expect("read");
    assert_eq!(
        first.other.get("keywords"),
        Some(&serde_json::json!([1, 2, 3]))
    );

    first.write_to(&p2).expect("write");
    let second = Manifest::read_from(&p2).expect("re-read");

    assert_eq!(
        second.other.get("keywords"),
        Some(&serde_json::json!([1, 2, 3]))
    );
    assert_eq!(first.other, second.other);
}
