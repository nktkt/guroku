use crate::error::{GurokuError, Result};
use crate::manifest::Manifest;
use std::fs;
use std::path::Path;

pub async fn run(cwd: &Path, packages: &[String]) -> Result<()> {
    let manifest_path = cwd.join("package.json");
    let mut manifest = Manifest::read_from(&manifest_path)?;
    let node_modules = cwd.join("node_modules");

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
    Ok(())
}
