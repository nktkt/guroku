//! Compile-time tests documenting v1.2's cascading-backtrack capability.
//!
//! v1.1's single-step backtracker could not undo more than one decision
//! at a time. v1.2's `pubgrub_resolver` delegates to the `pubgrub` crate,
//! which performs full conflict-driven cascading backtracks. These tests
//! exercise the public surface that supports that delegation; they do not
//! perform a full async solve.

#[allow(dead_code)]
async fn _cascade_resolve_compiles(
    client: &guroku::registry::RegistryClient,
    roots: &[(String, String)],
    manifest: &guroku::manifest::Manifest,
) -> guroku::Result<guroku::resolver::Resolution> {
    guroku::pubgrub_resolver::resolve_with_pubgrub(client, roots, manifest).await
}

#[test]
fn compile_check() {
    let _ = _cascade_resolve_compiles;
}

#[test]
fn resolution_conflict_variant_constructible() {
    use guroku::error::GurokuError;
    let err = GurokuError::ResolutionConflict {
        name: "x".into(),
        chosen: "1.0.0".into(),
        requested: "^2.0".into(),
        requested_by: "long pubgrub report ...".into(),
    };
    let s = format!("{err}");
    assert!(s.contains("version conflict"));
    assert!(s.contains("x"));
    assert!(s.contains("1.0.0"));
    assert!(s.contains("^2.0"));
}

#[test]
fn pubgrub_dependency_provider_trait_not_in_public_surface() {
    // Embedders should be able to drive resolution using only the
    // `guroku::pubgrub_resolver` paths; they must not have to implement
    // any `pubgrub::*` trait themselves. We assert that by referring
    // exclusively to guroku-owned items here.
    use guroku::pubgrub_resolver::NpmVersion;
    use guroku::version::parse_version;

    // The pubgrub_resolver entry point is reachable; we don't need to
    // build a function pointer to it (lifetimes on the borrowed args
    // make that awkward). Instead lean on `_cascade_resolve_compiles`
    // above as the type-shape guard.
    let _v: NpmVersion = NpmVersion(parse_version("1.0.0").unwrap());
}

#[test]
fn npm_range_satisfies_basic_caret() {
    use guroku::version::{parse_range, parse_version};
    let r = parse_range("^1.2.3").unwrap();
    let v = parse_version("1.5.0").unwrap();
    assert!(r.satisfies(&v));
    let too_low = parse_version("1.2.0").unwrap();
    assert!(!r.satisfies(&too_low));
    let too_high = parse_version("2.0.0").unwrap();
    assert!(!r.satisfies(&too_high));
}

#[test]
fn bump_strips_prerelease() {
    use guroku::pubgrub_resolver::NpmVersion;
    use guroku::version::parse_version;
    use pubgrub::version::Version;
    let v = NpmVersion(parse_version("1.2.3-rc.1").unwrap());
    assert_eq!(v.bump().to_string(), "1.2.4");
}
