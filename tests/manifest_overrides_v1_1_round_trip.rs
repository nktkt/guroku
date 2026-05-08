use guroku::manifest::Manifest;
use std::path::PathBuf;

fn fixture(name: &str) -> Manifest {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/fixtures");
    p.push(name);
    Manifest::read_from(&p).unwrap_or_else(|e| panic!("read {name}: {e}"))
}

#[test]
fn path_keyed_overrides_round_trip() {
    let m = fixture("manifest_with_path_keyed_overrides.json");
    assert!(
        m.overrides.keys().any(|k| k.contains('>')),
        "expected at least one path-keyed override (containing '>'), got: {:?}",
        m.overrides.keys().collect::<Vec<_>>()
    );
}

#[test]
fn glob_resolutions_round_trip() {
    let m = fixture("manifest_with_glob_resolutions.json");
    assert!(
        m.resolutions.keys().any(|k| k.starts_with("**/")),
        "expected at least one resolution starting with '**/', got: {:?}",
        m.resolutions.keys().collect::<Vec<_>>()
    );
}

#[test]
fn npm_alias_round_trip() {
    let m = fixture("manifest_with_npm_alias.json");
    assert!(
        m.dependencies.values().any(|v| v.starts_with("npm:")),
        "expected at least one dependency value starting with 'npm:', got: {:?}",
        m.dependencies.values().collect::<Vec<_>>()
    );
}

#[test]
fn path_keyed_overrides_no_truncation() {
    let m = fixture("manifest_with_path_keyed_overrides.json");
    let (k, v) = m
        .overrides
        .iter()
        .find(|(k, _)| k.contains('>'))
        .expect("at least one path-keyed override entry");
    assert!(
        !v.is_empty(),
        "path-keyed override value for key {k:?} must not be empty"
    );
}
