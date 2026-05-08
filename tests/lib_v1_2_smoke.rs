//! Library-side smoke test for the v1.2 surface.
//!
//! These tests are compile-only: they verify that public re-exports and
//! module paths exist with the expected types/signatures. Nothing is run.

#[test]
fn prelude_re_exports_resolved() {
    fn _check(_x: guroku::prelude::Resolved) {}
    let _: fn(guroku::resolver::Resolved) = _check;
}

#[test]
fn prelude_re_exports_resolution() {
    fn _check(_x: guroku::prelude::Resolution) {}
    let _: fn(guroku::resolver::Resolution) = _check;
}

#[test]
fn pubgrub_resolver_at_crate_root() {
    use guroku::pubgrub_resolver;
    let _ = std::marker::PhantomData::<pubgrub_resolver::NpmVersion>;
}

#[allow(dead_code)]
async fn _bfs_call(
    client: &guroku::registry::RegistryClient,
    roots: &[(String, String)],
    manifest: &guroku::manifest::Manifest,
) -> guroku::Result<guroku::resolver::Resolution> {
    guroku::resolver::resolve_with_manifest_overrides(client, roots, manifest).await
}

#[test]
fn bfs_resolver_still_callable() {
    let _ = _bfs_call;
}

#[allow(dead_code)]
async fn _pubgrub_call(
    client: &guroku::registry::RegistryClient,
    roots: &[(String, String)],
    manifest: &guroku::manifest::Manifest,
) -> guroku::Result<guroku::resolver::Resolution> {
    guroku::pubgrub_resolver::resolve_with_pubgrub(client, roots, manifest).await
}

#[test]
fn pubgrub_resolver_async_call_compiles_with_async_block() {
    let _ = _pubgrub_call;
}
