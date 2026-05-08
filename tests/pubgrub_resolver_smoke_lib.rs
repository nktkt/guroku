//! Library-side smoke test for the v1.2 `pubgrub_resolver` public surface.
//!
//! These checks exercise the type-level contract only: that the names
//! re-export, that `NpmVersion` carries the expected trait bag, that it
//! actually implements `pubgrub::version::Version`, and that
//! `Resolution` / `Resolved` can be constructed from outside the crate
//! with the documented field shape.

#[test]
fn imports_compile() {
    use guroku::pubgrub_resolver::{resolve_with_pubgrub, NpmVersion};
    // Silence unused-import lints; the point of this test is purely
    // that the `use` line above resolves.
    let _ = std::any::type_name::<NpmVersion>();
    let _ = resolve_with_pubgrub;
}

#[test]
fn npm_version_traits() {
    fn assert_clone_eq_hash<T: Clone + Eq + std::hash::Hash>() {}
    assert_clone_eq_hash::<guroku::pubgrub_resolver::NpmVersion>();
}

#[test]
fn npm_version_implements_ord() {
    fn assert_ord<T: Ord>() {}
    assert_ord::<guroku::pubgrub_resolver::NpmVersion>();
}

#[test]
fn npm_version_implements_display() {
    fn assert_display<T: std::fmt::Display>() {}
    assert_display::<guroku::pubgrub_resolver::NpmVersion>();
}

#[test]
fn pubgrub_version_trait_implemented() {
    fn _check<T: pubgrub::version::Version>() {}
    _check::<guroku::pubgrub_resolver::NpmVersion>();
}

#[test]
fn resolution_default_is_empty() {
    use guroku::resolver::Resolution;
    let r: Resolution = Default::default();
    assert_eq!(r.len(), 0);
}

#[test]
fn resolved_construction_with_aliased_from_some() {
    use guroku::registry::{Dist, VersionInfo};
    use guroku::resolver::Resolved;
    use std::collections::BTreeMap;
    use url::Url;

    let aliased: Option<String> = Some("lodash".to_string());

    let resolved = Resolved {
        info: VersionInfo {
            name: "my-lodash".to_string(),
            version: "4.17.21".to_string(),
            dist: Dist {
                tarball: Url::parse("https://example.com/x.tgz").unwrap(),
                integrity: None,
                shasum: None,
            },
            dependencies: BTreeMap::new(),
        },
        local_source: None,
        aliased_from: aliased,
    };

    assert_eq!(resolved.aliased_from.as_deref(), Some("lodash"));
}
