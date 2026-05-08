pub mod add;
pub mod audit;
pub mod exec;
pub mod install;
pub mod remove;
pub mod run;
pub mod workspaces;

use crate::error::{GurokuError, Result};
use crate::linker::LinkedPackage;
use crate::manifest::Manifest;
use crate::registry::VersionInfo;
use crate::{integrity, registry, store};
use std::path::{Path, PathBuf};

/// Fetch a tarball, verify its sha512, and ensure the bytes are extracted
/// into the CAS. Returns the on-disk path to the CAS entry.
pub(crate) async fn fetch_into_cas(
    client: &registry::RegistryClient,
    v: &VersionInfo,
) -> Result<PathBuf> {
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

    store::ensure_extracted(&bytes)
}

/// Translate a `Resolution` into the `Vec<LinkedPackage>` shape the linker
/// wants, given a map from name → CAS dir. Reads each package's manifest
/// from disk to populate the `bin_entries` field used for `.bin/` shims.
pub(crate) fn into_linked_packages(
    resolution: &crate::resolver::Resolution,
    cas_paths: &std::collections::HashMap<String, PathBuf>,
) -> Vec<LinkedPackage> {
    resolution
        .iter()
        .filter_map(|(local_name, r)| {
            // Local sources (file:/git:) bypass the CAS: we hardlink
            // straight from the working tree. Otherwise look up by the
            // *local* name (the resolution map key), which is what
            // install_from_resolution keyed cas_paths on.
            let source_dir = match &r.local_source {
                Some(p) => p.clone(),
                None => cas_paths.get(local_name)?.clone(),
            };
            let mut deps = std::collections::BTreeMap::new();
            for dep_name in r.info.dependencies.keys() {
                if let Some(rd) = resolution.packages.get(dep_name) {
                    deps.insert(dep_name.clone(), rd.info.version.clone());
                }
            }
            let bin_entries = Manifest::read_from(&source_dir.join("package.json"))
                .map(|m| m.bin_entries())
                .unwrap_or_default();
            // For aliases, the in-`node_modules` directory is the local
            // (user-declared) name — `r.info.name` is the registry name
            // and only matters at fetch time.
            Some(LinkedPackage {
                name: local_name.clone(),
                version: r.info.version.clone(),
                source_dir,
                dependencies: deps,
                bin_entries,
            })
        })
        .collect()
}

#[allow(dead_code)]
pub(crate) fn ensure_store_dir(_root: &Path) -> Result<()> {
    Ok(())
}

/// Parse a CLI package spec like `lodash`, `lodash@4.17.21`, `@scope/x`,
/// or `@scope/x@1`.
pub(crate) fn parse_spec(input: &str) -> (String, String) {
    if let Some(rest) = input.strip_prefix('@') {
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
