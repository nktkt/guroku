use crate::error::{GurokuError, Result};
use crate::linker;
use crate::lockfile::{Lockfile, PackageLock, LOCKFILE_NAME};
use crate::manifest::Manifest;
use crate::registry::RegistryClient;
use crate::resolver;
use crate::scripts;
use futures::stream::{self, StreamExt};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const CONCURRENCY: usize = 8;

/// Lifecycle script names invoked at install time, in order.
/// `prepare` is run after `postinstall` for the *root* project (matches npm).
const ROOT_PRE_SCRIPTS: &[&str] = &["preinstall"];
const ROOT_POST_SCRIPTS: &[&str] = &["install", "postinstall", "prepare"];

pub async fn run(cwd: &Path, frozen_lockfile: bool, ignore_scripts: bool) -> Result<()> {
    let manifest_path = cwd.join("package.json");
    let manifest = Manifest::read_from(&manifest_path)?;
    let node_modules = cwd.join("node_modules");
    let lock_path = cwd.join(LOCKFILE_NAME);
    let bin_dir = node_modules.join(".bin");

    let client = RegistryClient::from_npmrc(cwd)?;
    let roots: Vec<(String, String)> = manifest
        .all_dependencies()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    if roots.is_empty() {
        tracing::info!("no dependencies declared in {}", manifest_path.display());
        return Ok(());
    }

    let direct_dep_names: Vec<String> = roots.iter().map(|(n, _)| n.clone()).collect();

    if !ignore_scripts {
        run_root_scripts(cwd, &manifest, &bin_dir, ROOT_PRE_SCRIPTS)?;
    }

    let existing_lock = if lock_path.exists() {
        Some(Lockfile::read_from(&lock_path)?)
    } else {
        None
    };

    let linked = if frozen_lockfile {
        let lock = existing_lock
            .as_ref()
            .ok_or(GurokuError::LockfileOutOfDate)?;
        if !lock_covers(lock, &roots) {
            return Err(GurokuError::LockfileOutOfDate);
        }
        install_from_lock(&client, lock, &node_modules, &direct_dep_names).await?
    } else {
        tracing::info!("resolving {} root packages", roots.len());
        let resolution = resolver::resolve(&client, &roots).await?;
        tracing::info!("resolved {} packages", resolution.len());

        let linked =
            install_from_resolution(&client, &resolution, &node_modules, &direct_dep_names).await?;
        write_lockfile(&resolution, &lock_path)?;
        linked
    };

    if !ignore_scripts {
        run_per_package_postinstall(&linked, &node_modules);
        run_root_scripts(cwd, &manifest, &bin_dir, ROOT_POST_SCRIPTS)?;
    }

    tracing::info!("done");
    Ok(())
}

fn lock_covers(lock: &Lockfile, roots: &[(String, String)]) -> bool {
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
    direct_deps: &[String],
) -> Result<Vec<linker::LinkedPackage>> {
    let items: Vec<crate::registry::VersionInfo> =
        resolution.iter().map(|(_, r)| r.info.clone()).collect();

    let cas_results: Vec<Result<(String, PathBuf)>> = stream::iter(items)
        .map(|info| {
            let client = client.clone();
            async move {
                let path = super::fetch_into_cas(&client, &info).await?;
                Ok((info.name.clone(), path))
            }
        })
        .buffer_unordered(CONCURRENCY)
        .collect()
        .await;

    let mut cas_paths: HashMap<String, PathBuf> = HashMap::new();
    let mut failures = Vec::new();
    for r in cas_results {
        match r {
            Ok((name, path)) => {
                cas_paths.insert(name, path);
            }
            Err(e) => failures.push(e.to_string()),
        }
    }
    if !failures.is_empty() {
        return Err(GurokuError::Other(format!(
            "{} package(s) failed to download: {}",
            failures.len(),
            failures.join("; ")
        )));
    }

    let linked = super::into_linked_packages(resolution, &cas_paths);
    linker::populate_node_modules(&linked, node_modules, direct_deps)?;
    Ok(linked)
}

async fn install_from_lock(
    client: &RegistryClient,
    lock: &Lockfile,
    node_modules: &Path,
    direct_deps: &[String],
) -> Result<Vec<linker::LinkedPackage>> {
    use crate::registry::{Dist, VersionInfo};
    use std::collections::BTreeMap;
    use url::Url;

    let mut items: Vec<VersionInfo> = Vec::with_capacity(lock.packages.len());
    let mut declared_deps: HashMap<String, BTreeMap<String, String>> = HashMap::new();
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
        declared_deps.insert(name.to_string(), entry.dependencies.clone());
    }

    let cas_results: Vec<Result<(String, PathBuf)>> = stream::iter(items)
        .map(|info| {
            let client = client.clone();
            async move {
                let path = super::fetch_into_cas(&client, &info).await?;
                Ok((info.name.clone(), path))
            }
        })
        .buffer_unordered(CONCURRENCY)
        .collect()
        .await;

    let mut cas_paths: HashMap<String, PathBuf> = HashMap::new();
    for r in cas_results {
        let (name, path) = r?;
        cas_paths.insert(name, path);
    }

    let mut linked = Vec::with_capacity(lock.packages.len());
    for (key, entry) in &lock.packages {
        let (name, version) = key.rsplit_once('@').unwrap();
        let Some(source_dir) = cas_paths.get(name) else {
            continue;
        };
        let bin_entries = Manifest::read_from(&source_dir.join("package.json"))
            .map(|m| m.bin_entries())
            .unwrap_or_default();
        linked.push(linker::LinkedPackage {
            name: name.to_string(),
            version: version.to_string(),
            source_dir: source_dir.clone(),
            dependencies: entry.dependencies.clone(),
            bin_entries,
        });
    }
    linker::populate_node_modules(&linked, node_modules, direct_deps)?;
    Ok(linked)
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

fn run_root_scripts(cwd: &Path, manifest: &Manifest, bin_dir: &Path, names: &[&str]) -> Result<()> {
    for n in names {
        if let Some(body) = manifest.scripts.get(*n) {
            scripts::run_in(cwd, n, body, &[bin_dir])?;
        }
    }
    Ok(())
}

/// Per-package `postinstall` scripts. Failures here are warnings — npm
/// treats them as fatal by default but in v0.4 we err on the side of
/// "install at least mostly succeeded" so the user can debug. Use
/// `--ignore-scripts` to skip altogether.
fn run_per_package_postinstall(packages: &[linker::LinkedPackage], node_modules: &Path) {
    for pkg in packages {
        let inner_pkg_dir = node_modules.join(".guroku").join(format!(
            "{}@{}",
            pkg.name.replace('/', "+"),
            pkg.version
        ));
        let pkg_dir = inner_pkg_dir.join("node_modules").join(&pkg.name);
        let manifest_path = pkg_dir.join("package.json");
        let Ok(m) = Manifest::read_from(&manifest_path) else {
            continue;
        };
        for hook in ["preinstall", "install", "postinstall"] {
            if let Some(body) = m.scripts.get(hook) {
                let bin = node_modules.join(".bin");
                let inner_bin = inner_pkg_dir.join("node_modules").join(".bin");
                if let Err(e) = scripts::run_in(
                    &pkg_dir,
                    &format!("{}@{} ({hook})", pkg.name, pkg.version),
                    body,
                    &[bin.as_path(), inner_bin.as_path()],
                ) {
                    tracing::warn!("{e}");
                }
            }
        }
    }
}
