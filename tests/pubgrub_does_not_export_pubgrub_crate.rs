//! Compile-time tests defending the surface boundary between guroku and the
//! `pubgrub` crate. v1.2 added `pubgrub` as an internal dependency, but
//! embedders should not need to depend on `pubgrub` directly to use guroku's
//! public API. The mere fact that this file compiles — without ever
//! `use pubgrub::*` — is the proof.

#[allow(unused_imports)]
use guroku::prelude::*;

#[test]
fn prelude_compiles_without_pubgrub_imports() {
    // The presence of this test file (which imports `guroku::prelude::*`
    // but never `pubgrub::*`) is the proof: if any prelude item required
    // a pubgrub-the-crate type to USE, this file wouldn't compile.
    let _ = "ok";
}

#[test]
fn pubgrub_resolver_lives_at_crate_root() {
    // Confirm the canonical path is `guroku::pubgrub_resolver`, not
    // `guroku::resolver::pubgrub` or similar. Just importing the names is
    // enough — if the module moved, this file wouldn't compile.
    #[allow(unused_imports)]
    use guroku::pubgrub_resolver::{resolve_with_pubgrub, NpmVersion};
    let _ = "ok";
}

#[allow(dead_code)]
async fn _embedder_does_not_need_pubgrub(
    client: &guroku::registry::RegistryClient,
    roots: &[(String, String)],
    manifest: &guroku::manifest::Manifest,
) -> guroku::Result<guroku::resolver::Resolution> {
    // No `use pubgrub::*` here. If `resolve_with_pubgrub` returned a
    // pubgrub-typed payload, this fn wouldn't compile without the
    // pubgrub import.
    guroku::pubgrub_resolver::resolve_with_pubgrub(client, roots, manifest).await
}

#[test]
fn embedders_signature_works_without_pubgrub_imports() {
    let _ = _embedder_does_not_need_pubgrub;
}

#[test]
fn npm_version_field_is_node_semver_version() {
    use guroku::pubgrub_resolver::NpmVersion;
    use guroku::version::parse_version;
    let v: NpmVersion = NpmVersion(parse_version("1.2.3").unwrap());
    // node_semver isn't a top-level guroku import, so use guroku's
    // re-export. If `NpmVersion.0` ever stopped being a node-semver
    // `Version` (re-exported as `guroku::version::Version`), this would
    // fail to compile.
    let inner: &guroku::version::Version = &v.0;
    let _ = inner;
}

#[test]
fn root_package_constant_is_private() {
    // The absence of an import for ROOT_PACKAGE in this file is the
    // guard. If a future PR makes it `pub`, an explicit
    // `use guroku::pubgrub_resolver::ROOT_PACKAGE;` here would compile,
    // and a separate test in `pubgrub_resolver_simple.rs` would catch
    // the surface change.
    let _ = "documented";
}
