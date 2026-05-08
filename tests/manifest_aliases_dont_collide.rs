//! v1.1: aliased deps must not collide with regular deps that share the
//! aliased package's registry name. A manifest can hold both
//! `"react-old": "npm:react@^16"` and `"react": "^19"` as two distinct
//! entries — they end up as two independent node_modules / lockfile rows.
//! These tests pin the manifest-level invariants needed for that.

use std::fs;

use guroku::manifest::Manifest;
use guroku::specs::{classify, DepSpec};
use tempfile::TempDir;

const FIXTURE: &str = r#"{
    "name":"app","version":"1.0.0",
    "dependencies":{
        "react-old":"npm:react@^16",
        "react":"^19"
    }
}"#;

fn read_fixture() -> Manifest {
    let tmp = TempDir::new().expect("tempdir");
    let p = tmp.path().join("package.json");
    fs::write(&p, FIXTURE).expect("seed");
    Manifest::read_from(&p).expect("read")
}

#[test]
fn manifest_can_hold_alias_and_real() {
    let m = read_fixture();
    // Both keys present.
    assert!(
        m.dependencies.contains_key("react-old"),
        "missing alias key 'react-old' in {:?}",
        m.dependencies
    );
    assert!(
        m.dependencies.contains_key("react"),
        "missing real key 'react' in {:?}",
        m.dependencies
    );
    // And the values are distinct strings — one is an npm:-prefixed alias,
    // the other is a plain semver range.
    let alias_val = m.dependencies.get("react-old").expect("alias value");
    let real_val = m.dependencies.get("react").expect("real value");
    assert_ne!(
        alias_val, real_val,
        "alias and real spec values must differ"
    );
    assert_eq!(alias_val, "npm:react@^16");
    assert_eq!(real_val, "^19");
}

#[test]
fn classify_distinguishes_alias_and_range() {
    match classify("npm:react@^16") {
        DepSpec::Alias { real_name, inner } => {
            assert_eq!(real_name, "react");
            match *inner {
                DepSpec::Range(r) => assert_eq!(r, "^16"),
                other => panic!("alias inner should be Range, got {other:?}"),
            }
        }
        other => panic!("expected Alias for 'npm:react@^16', got {other:?}"),
    }

    match classify("^19") {
        DepSpec::Range(r) => assert_eq!(r, "^19"),
        other => panic!("expected Range for '^19', got {other:?}"),
    }
}

#[test]
fn merge_helpers_keep_both() {
    let m = read_fixture();
    let collected: Vec<(&String, &String)> = m.all_dependencies().collect();

    let count_alias = collected
        .iter()
        .filter(|(k, _)| k.as_str() == "react-old")
        .count();
    let count_real = collected
        .iter()
        .filter(|(k, _)| k.as_str() == "react")
        .count();

    assert_eq!(
        count_alias, 1,
        "alias key 'react-old' should appear exactly once, got {count_alias} in {collected:?}"
    );
    assert_eq!(
        count_real, 1,
        "real key 'react' should appear exactly once, got {count_real} in {collected:?}"
    );
}

#[test]
fn local_and_real_names_are_separate() {
    // String-level invariant: the local names differ.
    assert_ne!("react-old", "react");

    // The alias's real_name (extracted by classify) is the registry name.
    let real_name_of_alias = match classify("npm:react@^16") {
        DepSpec::Alias { real_name, .. } => real_name,
        other => panic!("expected Alias, got {other:?}"),
    };
    assert_eq!(real_name_of_alias, "react");

    // The plain `"^19"` spec is a Range — it is NOT an alias, so its
    // local-name equals itself ('react' maps to 'react').
    let plain = classify("^19");
    assert!(
        matches!(plain, DepSpec::Range(_)),
        "'^19' should classify as Range, got {plain:?}"
    );

    // Therefore: local-name 'react-old' resolves to registry name 'react',
    // AND local-name 'react' also resolves to registry name 'react' — but
    // they remain two separate manifest entries with two separate specs.
    let m = read_fixture();
    assert_eq!(
        m.dependencies.get("react-old"),
        Some(&"npm:react@^16".to_string())
    );
    assert_eq!(m.dependencies.get("react"), Some(&"^19".to_string()));
}
