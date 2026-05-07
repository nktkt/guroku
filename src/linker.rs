use crate::error::{GurokuError, Result};
use std::fs;
use std::path::Path;

/// v0.1: copy the contents of a store package into `node_modules/<name>` flat.
/// Hardlink-based linking is a v0.3 milestone.
pub fn link_flat(store_pkg_dir: &Path, node_modules: &Path, name: &str) -> Result<()> {
    let dest = node_modules.join(name);
    if dest.exists() {
        fs::remove_dir_all(&dest).map_err(|e| GurokuError::Io {
            path: dest.clone(),
            source: e,
        })?;
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| GurokuError::Io {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }
    copy_dir(store_pkg_dir, &dest)
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).map_err(|e| GurokuError::Io {
        path: dst.to_path_buf(),
        source: e,
    })?;
    for entry in fs::read_dir(src).map_err(|e| GurokuError::Io {
        path: src.to_path_buf(),
        source: e,
    })? {
        let entry = entry.map_err(|e| GurokuError::Io {
            path: src.to_path_buf(),
            source: e,
        })?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let ft = entry.file_type().map_err(|e| GurokuError::Io {
            path: from.clone(),
            source: e,
        })?;
        if ft.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            fs::copy(&from, &to).map_err(|e| GurokuError::Io {
                path: to.clone(),
                source: e,
            })?;
        }
    }
    Ok(())
}
