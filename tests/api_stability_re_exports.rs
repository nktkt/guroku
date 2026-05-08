//! API stability fence: these tests exist to break compilation if a future PR
//! removes a v1.0 public re-export. The compile-time check IS the test.

#![allow(dead_code, unused_imports, unused_variables)]

fn crate_root_re_exports_present() {
    let _: Option<guroku::Result<()>> = None;
    let _: Option<guroku::GurokuError> = None;
}

fn prelude_re_exports_present() {
    use guroku::prelude::*;
    let _: Option<Lockfile> = None;
    let _: Option<Manifest> = None;
    let _: Option<DepSpec> = None;
    let _: Option<Range> = None;
    let _: Option<Version> = None;
    let _: Option<Resolution> = None;
    let _: Option<Resolved> = None;
    let _: Option<RegistryClient> = None;
    let _: Option<PackageMetadata> = None;
    let _: Option<VersionInfo> = None;
    let _: Option<PackageLock> = None;
    let _: Option<GurokuError> = None;
    let _: Option<Result<()>> = None;
}

fn module_paths_still_work() {
    let _: Option<guroku::lockfile::Lockfile> = None;
    let _: Option<guroku::lockfile::PackageLock> = None;
    let _: Option<guroku::manifest::Manifest> = None;
    let _: Option<guroku::specs::DepSpec> = None;
    let _: Option<guroku::version::Range> = None;
    let _: Option<guroku::version::Version> = None;
    let _: Option<guroku::resolver::Resolution> = None;
    let _: Option<guroku::resolver::Resolved> = None;
    let _: Option<guroku::registry::RegistryClient> = None;
    let _: Option<guroku::registry::PackageMetadata> = None;
    let _: Option<guroku::registry::VersionInfo> = None;
    let _: Option<guroku::error::GurokuError> = None;
}

fn lockfile_constants_in_prelude() {
    use guroku::prelude::*;
    let _: Option<&str> = Some(LOCKFILE_NAME);
    let _: Option<u32> = Some(LOCKFILE_VERSION);
}

fn default_registry_constant_in_prelude() {
    use guroku::prelude::*;
    let _: Option<&str> = Some(DEFAULT_REGISTRY);
}

#[test]
fn pub_api_compiles() {
    crate_root_re_exports_present();
    prelude_re_exports_present();
    module_paths_still_work();
    lockfile_constants_in_prelude();
    default_registry_constant_in_prelude();
}
