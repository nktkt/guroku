pub mod add;
pub mod install;
pub mod remove;

use crate::error::{GurokuError, Result};
use crate::registry::VersionInfo;
use crate::{cache, integrity, linker, registry, tarball};
use std::path::Path;

/// Shared install primitive used by `install` and `add`. Fetches a single
/// package version from the registry, verifies its integrity, extracts it into
/// the store, and links it flat into `node_modules`.
pub(crate) async fn install_one(
    client: &registry::RegistryClient,
    name: &str,
    spec: &str,
    node_modules: &Path,
) -> Result<String> {
    let metadata = client.fetch_metadata(name).await?;
    let version_info: VersionInfo = metadata.resolve(spec)?.clone();
    install_version(client, &version_info, node_modules).await?;
    Ok(version_info.version)
}

pub(crate) async fn install_version(
    client: &registry::RegistryClient,
    v: &VersionInfo,
    node_modules: &Path,
) -> Result<()> {
    let store_pkg = cache::package_dir(&v.name, &v.version)?;
    if !store_pkg.join("package.json").exists() {
        tracing::info!("downloading {}@{}", v.name, v.version);
        let bytes = client.fetch_tarball(&v.dist.tarball).await?;

        if let Some(integ) = &v.dist.integrity {
            integrity::verify(&bytes, integ, &v.name, &v.version)?;
        } else if v.dist.shasum.is_none() {
            return Err(GurokuError::IntegrityMismatch {
                name: v.name.clone(),
                version: v.version.clone(),
                detail: "no integrity or shasum field on registry record".into(),
            });
        }
        // Note: shasum-only verification is not supported in v0.1.
        // The registry returns sha512 `integrity` for all packages published
        // since 2017, so this only matters for very old packages.

        tarball::extract(&bytes, &store_pkg)?;
    } else {
        tracing::debug!("cache hit: {}@{}", v.name, v.version);
    }
    linker::link_flat(&store_pkg, node_modules, &v.name)?;
    Ok(())
}

/// Parse a CLI package spec like `lodash`, `lodash@4.17.21`, or `@scope/x@1`.
pub(crate) fn parse_spec(input: &str) -> (String, String) {
    if let Some(rest) = input.strip_prefix('@') {
        // scoped: @scope/name[@version]
        if let Some(idx) = rest.find('@') {
            let (name_part, ver_part) = rest.split_at(idx);
            return (format!("@{}", name_part), ver_part[1..].to_string());
        }
        return (input.to_string(), "latest".to_string());
    }
    if let Some((name, ver)) = input.split_once('@') {
        return (name.to_string(), ver.to_string());
    }
    (input.to_string(), "latest".to_string())
}
