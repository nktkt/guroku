use crate::error::{GurokuError, Result};
use crate::lockfile::{Lockfile, PackageLock, LOCKFILE_NAME};
use crate::manifest::Manifest;
use crate::registry::RegistryClient;
use crate::resolver;
use futures::stream::{self, StreamExt};
use std::path::Path;

const CONCURRENCY: usize = 8;

pub async fn run(cwd: &Path, frozen_lockfile: bool) -> Result<()> {
    let manifest_path = cwd.join("package.json");
    let manifest = Manifest::read_from(&manifest_path)?;
    let node_modules = cwd.join("node_modules");
    let lock_path = cwd.join(LOCKFILE_NAME);

    let client = RegistryClient::with_default_registry()?;
    let roots: Vec<(String, String)> = manifest
        .all_dependencies()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    if roots.is_empty() {
        tracing::info!("no dependencies declared in {}", manifest_path.display());
        return Ok(());
    }

    let existing_lock = if lock_path.exists() {
        Some(Lockfile::read_from(&lock_path)?)
    } else {
        None
    };

    if frozen_lockfile {
        let lock = existing_lock
            .as_ref()
            .ok_or(GurokuError::LockfileOutOfDate)?;
        if !lock_covers(lock, &roots) {
            return Err(GurokuError::LockfileOutOfDate);
        }
        return install_from_lock(&client, lock, &node_modules).await;
    }

    tracing::info!("resolving {} root packages", roots.len());
    let resolution = resolver::resolve(&client, &roots).await?;
    tracing::info!("resolved {} packages", resolution.len());

    install_from_resolution(&client, &resolution, &node_modules).await?;
    write_lockfile(&resolution, &lock_path)?;

    tracing::info!("done");
    Ok(())
}

fn lock_covers(lock: &Lockfile, roots: &[(String, String)]) -> bool {
    // The lockfile must contain at least one entry whose name matches each
    // declared root. A more precise check would re-resolve and compare; this
    // is the cheap version.
    for (name, _) in roots {
        let mut found = false;
        for key in lock.packages.keys() {
            if let Some((k_name, _)) = key.split_once('@') {
                if k_name == name || (name.starts_with('@') && key.starts_with(name)) {
                    found = true;
                    break;
                }
            }
        }
        if !found {
            return false;
        }
    }
    true
}

pub(crate) async fn install_from_resolution(
    client: &RegistryClient,
    resolution: &resolver::Resolution,
    node_modules: &Path,
) -> Result<()> {
    let items: Vec<crate::registry::VersionInfo> =
        resolution.iter().map(|(_, r)| r.info.clone()).collect();

    let results: Vec<Result<()>> = stream::iter(items)
        .map(|info| {
            let client = client.clone();
            let node_modules = node_modules.to_path_buf();
            async move { super::install_version(&client, &info, &node_modules).await }
        })
        .buffer_unordered(CONCURRENCY)
        .collect()
        .await;

    let mut failures = Vec::new();
    for r in results {
        if let Err(e) = r {
            failures.push(e.to_string());
        }
    }
    if !failures.is_empty() {
        return Err(GurokuError::Other(format!(
            "{} package(s) failed to install: {}",
            failures.len(),
            failures.join("; ")
        )));
    }
    Ok(())
}

async fn install_from_lock(
    client: &RegistryClient,
    lock: &Lockfile,
    node_modules: &Path,
) -> Result<()> {
    use crate::registry::{Dist, VersionInfo};
    use std::collections::BTreeMap;
    use url::Url;

    let mut items: Vec<VersionInfo> = Vec::with_capacity(lock.packages.len());
    for (key, entry) in &lock.packages {
        let (name, version) = key
            .rsplit_once('@')
            .ok_or_else(|| GurokuError::Other(format!("malformed lockfile key `{key}`")))?;
        let url = Url::parse(&entry.resolved)?;
        items.push(VersionInfo {
            name: name.to_string(),
            version: version.to_string(),
            dist: Dist {
                tarball: url,
                integrity: entry.integrity.clone(),
                shasum: None,
            },
            dependencies: BTreeMap::new(),
        });
    }

    let results: Vec<Result<()>> = stream::iter(items)
        .map(|info| {
            let client = client.clone();
            let node_modules = node_modules.to_path_buf();
            async move { super::install_version(&client, &info, &node_modules).await }
        })
        .buffer_unordered(CONCURRENCY)
        .collect()
        .await;

    for r in results {
        r?;
    }
    Ok(())
}

pub(crate) fn write_lockfile(resolution: &resolver::Resolution, path: &Path) -> Result<()> {
    let mut lock = Lockfile::new();
    for (name, r) in resolution.iter() {
        let entry = PackageLock {
            resolved: r.info.dist.tarball.to_string(),
            integrity: r.info.dist.integrity.clone(),
            dependencies: r.info.dependencies.clone(),
        };
        lock.insert(name, &r.info.version, entry);
    }
    lock.write_to(path)
}
