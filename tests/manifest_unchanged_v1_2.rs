use guroku::manifest::Manifest;
use std::path::PathBuf;

#[test]
fn manifest_default_construct() {
    let m: Manifest = Default::default();
    assert!(m.dependencies.is_empty());
    assert!(m.dev_dependencies.is_empty());
    assert!(m.peer_dependencies.is_empty());
    assert!(m.optional_dependencies.is_empty());
    assert!(m.scripts.is_empty());
    assert!(m.bin.is_none());
    assert!(m.overrides.is_empty());
    assert!(m.resolutions.is_empty());
    assert!(m.other.is_empty());
}

#[test]
fn manifest_round_trip_v1_full_fixture() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("v1_manifest_full.json");
    let original = Manifest::read_from(&fixture).expect("read fixture");

    let tmp = std::env::temp_dir().join("guroku_v1_2_manifest_round_trip.json");
    original.write_to(&tmp).expect("write manifest");

    let reloaded = Manifest::read_from(&tmp).expect("re-read written manifest");

    assert_eq!(original.name, reloaded.name);
    assert_eq!(original.version, reloaded.version);
    assert_eq!(original.dependencies, reloaded.dependencies);
    assert_eq!(original.dev_dependencies, reloaded.dev_dependencies);
    assert_eq!(original.peer_dependencies, reloaded.peer_dependencies);
    assert_eq!(
        original.optional_dependencies,
        reloaded.optional_dependencies
    );
    assert_eq!(original.scripts, reloaded.scripts);
    assert_eq!(original.bin, reloaded.bin);
    assert_eq!(original.workspaces, reloaded.workspaces);
    assert_eq!(original.overrides, reloaded.overrides);
    assert_eq!(original.resolutions, reloaded.resolutions);
    assert_eq!(original.other, reloaded.other);

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn manifest_all_dependencies_iterates() {
    let mut m: Manifest = Default::default();
    m.dependencies
        .insert("alpha".to_string(), "^1.0.0".to_string());
    m.dev_dependencies
        .insert("beta".to_string(), "^2.0.0".to_string());
    m.peer_dependencies
        .insert("gamma".to_string(), "^3.0.0".to_string());
    m.optional_dependencies
        .insert("delta".to_string(), "^4.0.0".to_string());

    let names: Vec<String> = m.all_dependencies().map(|(n, _)| n.to_string()).collect();

    assert!(
        names.iter().any(|n| n == "alpha"),
        "all_dependencies() must include `dependencies` entries; got {:?}",
        names
    );
    assert!(
        names.iter().any(|n| n == "beta"),
        "all_dependencies() must include `dev_dependencies` entries; got {:?}",
        names
    );
}

#[test]
fn manifest_add_dependency_lands_in_dependencies() {
    let mut m: Manifest = Default::default();
    m.add_dependency("foo", "^1.0.0");
    assert_eq!(m.dependencies.get("foo"), Some(&"^1.0.0".to_string()));
}

#[test]
fn manifest_overrides_field_is_btreemap_string_string() {
    fn _shape(m: &Manifest) -> &std::collections::BTreeMap<String, String> {
        &m.overrides
    }
    let m: Manifest = Default::default();
    let _: &std::collections::BTreeMap<String, String> = _shape(&m);
}
