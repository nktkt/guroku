use guroku::manifest::Manifest;
use guroku::overrides::{classify_entries, OverrideKind, OverrideSource};

fn manifest_with_overrides(pairs: &[(&str, &str)]) -> Manifest {
    let mut m = Manifest::default();
    for (k, v) in pairs {
        m.overrides.insert((*k).into(), (*v).into());
    }
    m
}

fn manifest_with_resolutions(pairs: &[(&str, &str)]) -> Manifest {
    let mut m = Manifest::default();
    for (k, v) in pairs {
        m.resolutions.insert((*k).into(), (*v).into());
    }
    m
}

#[test]
fn flat_key_is_classified_flat() {
    let m = manifest_with_overrides(&[("foo", "1.0.0")]);
    let entries = classify_entries(&m);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].kind, OverrideKind::Flat);
}

#[test]
fn path_key_is_classified_path() {
    let m = manifest_with_overrides(&[("a > b", "1.0.0")]);
    let entries = classify_entries(&m);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].kind, OverrideKind::Path);
}

#[test]
fn glob_key_is_classified_glob() {
    let m = manifest_with_resolutions(&[("**/foo", "1.0.0")]);
    let entries = classify_entries(&m);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].kind, OverrideKind::Glob);
}

#[test]
fn empty_key_is_unknown() {
    let mut m = Manifest::default();
    m.overrides.insert("".into(), "1.0".into());
    let entries = classify_entries(&m);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].kind, OverrideKind::Unknown);
}

#[test]
fn mixed_kinds_yields_correct_classification() {
    let mut m = Manifest::default();
    m.overrides.insert("flat".into(), "1".into());
    m.overrides.insert("a > b".into(), "2".into());
    m.resolutions.insert("**/foo".into(), "3".into());

    let entries = classify_entries(&m);
    assert_eq!(entries.len(), 3);

    let flat = entries
        .iter()
        .find(|e| e.kind == OverrideKind::Flat)
        .expect("expected a Flat entry");
    assert_eq!(flat.source, OverrideSource::Overrides);

    let path = entries
        .iter()
        .find(|e| e.kind == OverrideKind::Path)
        .expect("expected a Path entry");
    assert_eq!(path.source, OverrideSource::Overrides);

    let glob = entries
        .iter()
        .find(|e| e.kind == OverrideKind::Glob)
        .expect("expected a Glob entry");
    assert_eq!(glob.source, OverrideSource::Resolutions);
}

#[test]
fn source_field_distinguishes_overrides_from_resolutions() {
    let mut m = Manifest::default();
    m.overrides.insert("x".into(), "1".into());
    m.resolutions.insert("x".into(), "2".into());

    let entries = classify_entries(&m);
    assert_eq!(entries.len(), 2);

    assert!(entries
        .iter()
        .any(|e| e.source == OverrideSource::Overrides));
    assert!(entries
        .iter()
        .any(|e| e.source == OverrideSource::Resolutions));
}
