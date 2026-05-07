use crate::error::{GurokuError, Result};
use crate::lockfile::LOCKFILE_NAME;
use crate::manifest::Manifest;
use crate::registry::RegistryClient;
use crate::resolver;
use std::fs;
use std::path::Path;

pub async fn run(cwd: &Path, packages: &[String]) -> Result<()> {
    let manifest_path = cwd.join("package.json");
    let mut manifest = Manifest::read_from(&manifest_path)?;
    let node_modules = cwd.join("node_modules");
    let lock_path = cwd.join(LOCKFILE_NAME);

    for name in packages {
        let removed = manifest.remove_dependency(name);
        if !removed {
            tracing::warn!("`{name}` was not in dependencies");
        }
        let dir = node_modules.join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).map_err(|e| GurokuError::Io {
                path: dir.clone(),
                source: e,
            })?;
            tracing::info!("removed {}", dir.display());
        }
    }

    manifest.write_to(&manifest_path)?;

    // Re-resolve from the trimmed manifest so the lockfile reflects only the
    // packages that are still reachable. If there are no deps left, drop the
    // lockfile entirely.
    let roots: Vec<(String, String)> = manifest
        .all_dependencies()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    if roots.is_empty() {
        if lock_path.exists() {
            fs::remove_file(&lock_path).map_err(|e| GurokuError::Io {
                path: lock_path.clone(),
                source: e,
            })?;
        }
        return Ok(());
    }

    let client = RegistryClient::from_npmrc(cwd)?;
    let resolution = resolver::resolve(&client, &roots).await?;
    super::install::write_lockfile(&resolution, &lock_path)?;

    Ok(())
}
