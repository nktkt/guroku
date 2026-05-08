//! v1.2 stability checks for `guroku::overrides`.
//!
//! These tests pin the public surface and the precedence ladder so that
//! v1.2 keeps the v1.1 contract.

use guroku::manifest::Manifest;

#[test]
fn lookup_signature_unchanged() {
    let _: fn(&Manifest, &str) -> Option<String> = guroku::overrides::lookup;
}

#[test]
fn lookup_with_path_signature_unchanged() {
    fn _shape(m: &Manifest, p: &[&str]) -> Option<String> {
        guroku::overrides::lookup_with_path(m, p)
    }
    let _: fn(&Manifest, &[&str]) -> Option<String> = _shape;
}

#[test]
fn merged_signature_unchanged() {
    let _ = guroku::overrides::merged(&Manifest::default());
}

#[test]
fn classify_entries_signature_unchanged() {
    let m = Manifest::default();
    let entries = guroku::overrides::classify_entries(&m);
    let _: Vec<_> = entries;
}

#[test]
fn override_kind_variants_present() {
    fn _kind_match(k: guroku::overrides::OverrideKind) -> &'static str {
        match k {
            guroku::overrides::OverrideKind::Flat => "flat",
            guroku::overrides::OverrideKind::Path => "path",
            guroku::overrides::OverrideKind::Glob => "glob",
            guroku::overrides::OverrideKind::Unknown => "unknown",
        }
    }
    assert_eq!(_kind_match(guroku::overrides::OverrideKind::Flat), "flat");
    assert_eq!(_kind_match(guroku::overrides::OverrideKind::Path), "path");
    assert_eq!(_kind_match(guroku::overrides::OverrideKind::Glob), "glob");
    assert_eq!(
        _kind_match(guroku::overrides::OverrideKind::Unknown),
        "unknown"
    );
}

#[test]
fn override_source_variants_present() {
    fn _source_match(s: guroku::overrides::OverrideSource) -> &'static str {
        match s {
            guroku::overrides::OverrideSource::Overrides => "overrides",
            guroku::overrides::OverrideSource::Resolutions => "resolutions",
        }
    }
    assert_eq!(
        _source_match(guroku::overrides::OverrideSource::Overrides),
        "overrides"
    );
    assert_eq!(
        _source_match(guroku::overrides::OverrideSource::Resolutions),
        "resolutions"
    );
}

#[test]
fn precedence_ladder_v1_2_matches_v1_1() {
    // Ladder (highest first):
    //   1. exact-path key in `overrides`
    //   2. flat-name key in `overrides`
    //   3. exact-path key in `resolutions`
    //   4. flat-name key in `resolutions`
    //   5. glob `**/<name>` in `resolutions`
    let mut m = Manifest::default();
    m.overrides.insert(
        "a > b > leaf".to_string(),
        "1.0.0-overrides-path".to_string(),
    );
    m.overrides
        .insert("leaf".to_string(), "1.0.0-overrides-flat".to_string());
    m.resolutions.insert(
        "x > y > leaf".to_string(),
        "1.0.0-resolutions-path".to_string(),
    );
    m.resolutions
        .insert("leaf".to_string(), "1.0.0-resolutions-flat".to_string());
    m.resolutions
        .insert("**/leaf".to_string(), "1.0.0-resolutions-glob".to_string());

    // 1. exact-path overrides wins.
    assert_eq!(
        guroku::overrides::lookup_with_path(&m, &["a", "b", "leaf"]),
        Some("1.0.0-overrides-path".to_string())
    );

    // 2. with overrides path removed, flat overrides wins.
    let mut m2 = m.clone();
    m2.overrides.remove("a > b > leaf");
    assert_eq!(
        guroku::overrides::lookup_with_path(&m2, &["a", "b", "leaf"]),
        Some("1.0.0-overrides-flat".to_string())
    );

    // 3. drop flat overrides too — exact-path resolutions wins.
    let mut m3 = m2.clone();
    m3.overrides.remove("leaf");
    assert_eq!(
        guroku::overrides::lookup_with_path(&m3, &["x", "y", "leaf"]),
        Some("1.0.0-resolutions-path".to_string())
    );

    // 4. drop the resolutions path key — flat resolutions wins.
    let mut m4 = m3.clone();
    m4.resolutions.remove("x > y > leaf");
    assert_eq!(
        guroku::overrides::lookup_with_path(&m4, &["x", "y", "leaf"]),
        Some("1.0.0-resolutions-flat".to_string())
    );

    // 5. drop the flat resolutions key — glob resolutions wins.
    let mut m5 = m4.clone();
    m5.resolutions.remove("leaf");
    assert_eq!(
        guroku::overrides::lookup_with_path(&m5, &["x", "y", "leaf"]),
        Some("1.0.0-resolutions-glob".to_string())
    );
}
