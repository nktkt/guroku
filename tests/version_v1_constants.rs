//! Pin v1.0 public constants so any stealth change in a minor release
//! surfaces immediately as a failing test.

#[test]
fn lockfile_version_is_one() {
    assert_eq!(guroku::lockfile::LOCKFILE_VERSION, 1);
}

#[test]
fn lockfile_name_is_guroku_lock() {
    assert_eq!(guroku::lockfile::LOCKFILE_NAME, "guroku.lock");
}

#[test]
fn default_registry_is_npmjs_org() {
    assert_eq!(
        guroku::registry::DEFAULT_REGISTRY,
        "https://registry.npmjs.org"
    );
}

#[test]
fn cas_ready_marker_is_dotfile() {
    assert_eq!(guroku::store::CAS_READY_MARKER, ".guroku-cas-ready");
}

#[test]
fn crate_version_is_one_dot_x() {
    assert!(env!("CARGO_PKG_VERSION").starts_with("1."));
}
