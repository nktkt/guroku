//! Compile-time API stability tests for the v1.0-frozen `guroku::prelude`.
//!
//! v1.1 and v1.2 are strictly additive on the v1.0 surface. These tests
//! confirm each v1.0 prelude item is still re-exported and minimally usable
//! in v1.2.

#[test]
fn prelude_has_lockfile_type() {
    let _: Option<guroku::prelude::Lockfile> = None;
}

#[test]
fn prelude_has_manifest_type() {
    let _: Option<guroku::prelude::Manifest> = None;
}

#[test]
fn prelude_has_registry_client_type() {
    let _: Option<guroku::prelude::RegistryClient> = None;
}

#[test]
fn prelude_has_resolution_type() {
    let _: guroku::prelude::Resolution = Default::default();
}

#[test]
fn prelude_has_resolved_type() {
    let _: Option<guroku::prelude::Resolved> = None;
}

#[test]
fn prelude_has_dep_spec_type() {
    let _ = guroku::prelude::DepSpec::Range("1.2.3".into());
}

#[test]
fn prelude_has_git_ref_type() {
    let _: Option<guroku::prelude::GitRef> = None;
}

#[test]
fn prelude_has_classify_spec() {
    let _ = guroku::prelude::classify_spec("^1.2.3");
}

#[test]
fn prelude_has_range_type() {
    let _: Option<guroku::prelude::Range> = None;
}

#[test]
fn prelude_has_version_type() {
    let _: Option<guroku::prelude::Version> = None;
}

#[test]
fn prelude_has_parse_range_fn() {
    let _ = guroku::prelude::parse_range("^1");
}

#[test]
fn prelude_has_parse_version_fn() {
    let _ = guroku::prelude::parse_version("1.2.3");
}

#[test]
fn prelude_has_max_satisfying_fn() {
    let _ = guroku::prelude::max_satisfying(["1.0.0"], &guroku::prelude::parse_range("*").unwrap());
}

#[test]
fn prelude_has_lockfile_name_constant() {
    assert_eq!(guroku::prelude::LOCKFILE_NAME, "guroku.lock");
}

#[test]
fn prelude_has_lockfile_version_constant() {
    assert_eq!(guroku::prelude::LOCKFILE_VERSION, 1);
}

#[test]
fn prelude_has_default_registry_constant() {
    assert!(guroku::prelude::DEFAULT_REGISTRY.starts_with("https://"));
}

#[test]
fn prelude_has_guroku_error_type() {
    let _: Option<guroku::prelude::GurokuError> = None;
}

#[test]
fn prelude_has_result_alias() {
    let _: guroku::prelude::Result<u32> = Ok(0);
}
