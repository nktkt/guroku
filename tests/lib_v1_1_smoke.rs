//! Library-side smoke test: confirms the v1.1 public surface still
//! type-checks. Catches accidental signature drift even before any
//! user-visible behaviour breaks.

#[test]
fn prelude_re_exports_resolved() {
    fn _check(_x: guroku::prelude::Resolved) {}
    // Also ensure the alias compiles via the public path: assigning a
    // function pointer typed against `resolver::Resolved` from one typed
    // against `prelude::Resolved` only succeeds if they're the same type.
    let _: fn(guroku::resolver::Resolved) = _check;
}

#[test]
fn prelude_re_exports_dep_spec() {
    use guroku::prelude::DepSpec;
    let _ = DepSpec::Range("1.2.3".into());
}

#[test]
fn dep_spec_alias_constructible() {
    let _ = guroku::specs::DepSpec::Alias {
        real_name: "real".into(),
        inner: Box::new(guroku::specs::DepSpec::Range("^1".into())),
    };
}

#[test]
fn resolved_carries_aliased_from() {
    use guroku::registry::{Dist, VersionInfo};
    use std::collections::BTreeMap;
    use url::Url;
    let info = VersionInfo {
        name: "real".into(),
        version: "1.0.0".into(),
        dist: Dist {
            tarball: Url::parse("https://example.invalid/x.tgz").unwrap(),
            integrity: None,
            shasum: None,
        },
        dependencies: BTreeMap::new(),
    };
    let r = guroku::resolver::Resolved {
        info,
        local_source: None,
        aliased_from: Some("real".into()),
    };
    assert_eq!(r.aliased_from.as_deref(), Some("real"));
}

#[test]
fn manifest_default_exists() {
    let _: guroku::manifest::Manifest = Default::default();
}
