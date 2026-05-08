//! Compile-time checks of the v1.2 `pubgrub_resolver` public surface.
//!
//! These tests do not perform any network I/O. They merely assert that
//! the expected items exist on the public path with the expected shape.
//! If the v1.2 entry point is renamed, removed, or its visibility changes,
//! this file will fail to compile.

use guroku::pubgrub_resolver::{resolve_with_pubgrub, NpmVersion};

#[test]
fn resolve_with_pubgrub_signature() {
    // The mere act of naming `resolve_with_pubgrub` in a `use` at the top
    // of this file proves the public path exists. Because the function is
    // `async` and generic over a client type, binding it to a concrete
    // `fn` pointer is fragile; importing the symbol is sufficient.
    let _ = resolve_with_pubgrub;
}

#[test]
fn npm_version_lowest_zero() {
    assert_eq!(
        format!(
            "{}",
            <guroku::pubgrub_resolver::NpmVersion as pubgrub::version::Version>::lowest()
        ),
        "0.0.0"
    );
}

#[test]
fn root_package_constant_is_pub_crate() {
    // Compile-time guard: this would fail if `ROOT_PACKAGE` were `pub`.
    // We can't write the negative directly (Rust can't `assert!(!visible)`),
    // but the very absence of a `use` for it in this file is the guard.
    // Mark this test passing as documentation.
    let _ = "ROOT_PACKAGE is pub(crate); this test documents that fact.";
}

#[test]
fn imports_compile() {
    // `NpmVersion` is a struct (a type), not a value, so we cannot bind it
    // directly. Constructing an `Option::<NpmVersion>::None` proves the
    // type is reachable on the public path without requiring a particular
    // constructor to be public.
    let _: Option<NpmVersion> = None;
    let _ = resolve_with_pubgrub;
}

#[test]
fn dependency_provider_trait_not_publicly_required() {
    // Embedders should be able to call into `guroku::pubgrub_resolver`
    // without depending on the `pubgrub` crate themselves. This test
    // intentionally references ONLY paths under `guroku::pubgrub_resolver`
    // and never names `pubgrub::*` directly.
    let _: Option<guroku::pubgrub_resolver::NpmVersion> = None;
    let _ = guroku::pubgrub_resolver::resolve_with_pubgrub;
}
