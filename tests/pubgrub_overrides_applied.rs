use guroku::manifest::Manifest;
use guroku::overrides::lookup_with_path;

fn manifest_with_overrides(entries: &[(&str, &str)]) -> Manifest {
    let mut m = Manifest::default();
    for (k, v) in entries {
        m.overrides.insert((*k).to_string(), (*v).to_string());
    }
    m
}

fn manifest_with_resolutions(entries: &[(&str, &str)]) -> Manifest {
    let mut m = Manifest::default();
    for (k, v) in entries {
        m.resolutions.insert((*k).to_string(), (*v).to_string());
    }
    m
}

#[test]
fn flat_override_applies_at_root() {
    let m = manifest_with_overrides(&[("lodash", "9.9.9")]);
    assert_eq!(lookup_with_path(&m, &["lodash"]), Some("9.9.9".to_string()));
}

#[test]
fn path_override_applies_at_root() {
    let m = manifest_with_overrides(&[("is-odd > is-number", "9.9.9")]);
    assert_eq!(
        lookup_with_path(&m, &["is-odd", "is-number"]),
        Some("9.9.9".to_string())
    );
}

#[test]
fn glob_resolution_applies_at_root() {
    let m = manifest_with_resolutions(&[("**/is-number", "9.9.9")]);
    assert_eq!(
        lookup_with_path(&m, &["is-number"]),
        Some("9.9.9".to_string())
    );
}

#[test]
fn overrides_win_over_resolutions_for_pubgrub() {
    let mut m = Manifest::default();
    m.resolutions
        .insert("lodash".to_string(), "1.0.0".to_string());
    m.overrides
        .insert("lodash".to_string(), "2.0.0".to_string());
    assert_eq!(lookup_with_path(&m, &["lodash"]), Some("2.0.0".to_string()));
}

#[test]
fn overrides_module_path_unchanged() {
    // Module path remains `guroku::overrides::lookup_with_path` —
    // confirmed by every test above using that path.
    let _ = guroku::overrides::lookup_with_path(&Manifest::default(), &["nope"]);
}
