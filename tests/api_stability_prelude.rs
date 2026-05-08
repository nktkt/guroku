//! API stability tests for the v1.0 `guroku::prelude` re-export surface.
//!
//! These tests are written from an embedder's perspective: they `use
//! guroku::prelude::*;` and exercise each item via its documented public
//! constructor. If anything here stops compiling we have made an
//! incompatible change to the v1.0 surface.

use guroku::prelude::*;

#[test]
fn compile_smoke_with_use_glob() {
    // Manifest::default() is the documented zero-config constructor.
    let _: Manifest = Manifest::default();

    // Lockfile::new() builds an empty lockfile at the current LOCKFILE_VERSION.
    let _: Lockfile = Lockfile::new();

    // RegistryClient::with_default_registry() points at DEFAULT_REGISTRY.
    let _: RegistryClient =
        RegistryClient::with_default_registry().expect("default registry url is valid");

    // parse_range / parse_version are the canonical entry points for semver.
    let _: Range = parse_range("^1.2.3").expect("caret range parses");
    let _: Version = parse_version("1.2.3").expect("triple-dot version parses");

    // Touch the remaining prelude symbols so a missing re-export would fail
    // to compile rather than be silently dropped from the surface.
    let _spec: DepSpec = classify_spec("1.0.0");
    let _git: Option<GitRef> = None;
    let _err: Option<GurokuError> = None;
    let _ok: Result<()> = Ok(());
    let _meta: Option<PackageMetadata> = None;
    let _vinfo: Option<VersionInfo> = None;
    let _resolution: Option<Resolution> = None;
    let _resolved: Option<Resolved> = None;
    let _pkg: Option<PackageLock> = None;
}

#[test]
fn lockfile_constants_have_expected_values() {
    assert_eq!(LOCKFILE_VERSION, 1, "v1.0 lockfile version is frozen at 1");
    assert_eq!(
        LOCKFILE_NAME, "guroku.lock",
        "v1.0 lockfile filename is frozen"
    );
}

#[test]
fn default_registry_constant_value() {
    assert!(
        DEFAULT_REGISTRY.contains("registry.npmjs.org"),
        "DEFAULT_REGISTRY should point at the public npm registry, got {DEFAULT_REGISTRY:?}"
    );
}

#[test]
fn classify_spec_alias_works() {
    // The prelude exposes `classify` under the friendlier name `classify_spec`.
    match classify_spec("^1") {
        DepSpec::Range(r) => assert_eq!(r, "^1"),
        other => panic!("expected DepSpec::Range, got {other:?}"),
    }

    match classify_spec("file:./x") {
        DepSpec::File(path) => assert_eq!(path, "./x"),
        other => panic!("expected DepSpec::File, got {other:?}"),
    }
}

#[test]
fn max_satisfying_via_prelude() {
    let range = parse_range("^1.2.0").expect("range parses");
    let candidates = ["1.0.0", "1.2.3", "1.4.0", "2.0.0", "not-a-version"];
    let pick = max_satisfying(candidates.iter().copied(), &range)
        .expect("at least one candidate satisfies ^1.2.0");
    let expected = parse_version("1.4.0").expect("expected pick parses");
    assert_eq!(
        pick, expected,
        "max_satisfying should pick the highest in-range"
    );
}
