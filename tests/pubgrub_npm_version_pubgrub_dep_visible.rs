//! Linker-presence tests confirming the `pubgrub` crate (a regular dep, v1.2)
//! is reachable from integration tests. These act as compile-time canaries:
//! they fail to build if the dep is removed or if pubgrub's public API
//! surface shifts between 0.2.x patches.

#[test]
fn pubgrub_range_constructible() {
    let _ = pubgrub::range::Range::<guroku::pubgrub_resolver::NpmVersion>::any();
}

#[test]
fn default_string_reporter_lives_at_expected_path() {
    fn _check<R: pubgrub::report::Reporter<String, guroku::pubgrub_resolver::NpmVersion>>() {}
    _check::<pubgrub::report::DefaultStringReporter>();
}

#[test]
fn pubgrub_solver_resolve_function_visible() {
    // The `use` is the canary: if `pubgrub::solver::resolve` is renamed
    // or moved between 0.2.x patches, this stops compiling.
    #[allow(unused_imports)]
    use pubgrub::solver::resolve;
}

#[test]
fn pubgrub_dependencies_unknown_constructible() {
    use pubgrub::solver::Dependencies;
    let _: Dependencies<String, guroku::pubgrub_resolver::NpmVersion> = Dependencies::Unknown;
}
