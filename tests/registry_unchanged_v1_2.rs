//! Compile-time stability checks for `guroku::registry` in v1.2.
//!
//! v1.2 introduces pubgrub-based resolution but reuses the v1.0 registry
//! types unchanged. These tests pin the public surface so accidental
//! breakage is caught at compile time.

use guroku::Result;

#[test]
fn registry_client_constructor_with_default_registry() {
    let _: Result<_> = Ok(guroku::registry::RegistryClient::with_default_registry());
}

#[test]
fn registry_client_from_npmrc_signature() {
    let _: fn(&std::path::Path) -> _ = guroku::registry::RegistryClient::from_npmrc;
}

#[test]
fn package_metadata_struct_shape() {
    fn _shape(m: &guroku::registry::PackageMetadata) -> usize {
        m.versions.len()
    }
}

#[test]
fn version_info_struct_shape() {
    fn _shape(v: &guroku::registry::VersionInfo) -> &str {
        &v.name
    }
}

#[test]
fn dist_struct_shape() {
    fn _shape(d: &guroku::registry::Dist) -> &url::Url {
        &d.tarball
    }
}

#[test]
fn default_registry_constant() {
    assert!(guroku::registry::DEFAULT_REGISTRY.starts_with("https://"));
}
