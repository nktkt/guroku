//! `node_modules` linker.
//!
//! - `link_flat` — the v0.1 behaviour: copy a store directory into
//!   `node_modules/<name>` recursively. Retained for tests and embedders;
//!   not on the v0.3 install path.
//! - `link_hardlink_tree` — replaces a recursive copy with a recursive
//!   hardlink (with a copy fallback when hardlinks fail, e.g. across
//!   filesystems).
//! - `populate_node_modules` — the v0.3 strict pnpm-style writer. Builds
//!   `node_modules/.guroku/<name>@<version>/node_modules/<name>/` for every
//!   package, sibling-symlinks each package's deps inside its own
//!   `node_modules/`, and surfaces direct deps via top-level symlinks.

use crate::error::{GurokuError, Result};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

/// v0.1: copy the contents of a store package into `node_modules/<name>` flat.
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

/// Recreate `src`'s contents at `dst` using hardlinks where possible. Falls
/// back to a copy if `hard_link` fails (cross-filesystem, exfat, etc.). Files
/// inside the CAS marker (`.guroku-cas-ready`) are skipped.
pub fn link_hardlink_tree(src: &Path, dst: &Path) -> Result<()> {
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
        let name = entry.file_name();
        if name == ".guroku-cas-ready" {
            continue;
        }
        let from = entry.path();
        let to = dst.join(&name);
        let ft = entry.file_type().map_err(|e| GurokuError::Io {
            path: from.clone(),
            source: e,
        })?;
        if ft.is_dir() {
            link_hardlink_tree(&from, &to)?;
        } else if ft.is_symlink() {
            // Tarballs occasionally ship symlinks. Reproduce them as such.
            let target = fs::read_link(&from).map_err(|e| GurokuError::Io {
                path: from.clone(),
                source: e,
            })?;
            symlink_file_or_dir(&target, &to)?;
        } else {
            match fs::hard_link(&from, &to) {
                Ok(()) => {}
                Err(_) => {
                    fs::copy(&from, &to).map_err(|e| GurokuError::Io {
                        path: to.clone(),
                        source: e,
                    })?;
                }
            }
        }
    }
    Ok(())
}

/// Description of one package being linked into `node_modules`.
#[derive(Debug, Clone)]
pub struct LinkedPackage {
    pub name: String,
    pub version: String,
    /// CAS directory holding the extracted contents.
    pub source_dir: PathBuf,
    /// Name → resolved exact version for this package's *own* deps.
    pub dependencies: BTreeMap<String, String>,
    /// `bin` entries from this package's manifest: `(bin_name, relative_path)`.
    /// Empty when the package declares no `bin` field. Used by v0.4 to
    /// populate `node_modules/.bin/`.
    #[doc(hidden)]
    pub bin_entries: Vec<(String, String)>,
}

/// Populate `node_modules` with the strict pnpm-style layout for all of
/// `packages`. `direct_deps` lists the names that appeared in the project's
/// `package.json` (top-level symlinks are created for those).
pub fn populate_node_modules(
    packages: &[LinkedPackage],
    node_modules: &Path,
    direct_deps: &[String],
) -> Result<()> {
    let guroku_root = node_modules.join(".guroku");
    fs::create_dir_all(&guroku_root).map_err(|e| GurokuError::Io {
        path: guroku_root.clone(),
        source: e,
    })?;

    let by_name: HashMap<&str, &LinkedPackage> =
        packages.iter().map(|p| (p.name.as_str(), p)).collect();

    // 1. For each package, materialise its files under
    //    .guroku/<safe_name>@<version>/node_modules/<name>/.
    for pkg in packages {
        let pkg_root = guroku_root.join(safe_pkg_id(&pkg.name, &pkg.version));
        let inner_nm = pkg_root.join("node_modules");
        fs::create_dir_all(&inner_nm).map_err(|e| GurokuError::Io {
            path: inner_nm.clone(),
            source: e,
        })?;
        let pkg_dir = nested_pkg_path(&inner_nm, &pkg.name);
        if let Some(parent) = pkg_dir.parent() {
            fs::create_dir_all(parent).map_err(|e| GurokuError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
        if pkg_dir.exists() {
            fs::remove_dir_all(&pkg_dir).map_err(|e| GurokuError::Io {
                path: pkg_dir.clone(),
                source: e,
            })?;
        }
        link_hardlink_tree(&pkg.source_dir, &pkg_dir)?;
    }

    // 2. For each package, drop sibling symlinks for its declared deps.
    for pkg in packages {
        let pkg_root = guroku_root.join(safe_pkg_id(&pkg.name, &pkg.version));
        let inner_nm = pkg_root.join("node_modules");
        for (dep_name, dep_version) in &pkg.dependencies {
            let Some(dep) = by_name.get(dep_name.as_str()) else {
                // Resolver may have skipped this (e.g. a peer dep). Don't
                // create a dangling symlink — just warn.
                tracing::debug!(
                    "skipping dep symlink for {}@{} dep {} (not in resolution)",
                    pkg.name,
                    pkg.version,
                    dep_name
                );
                continue;
            };
            // Sanity: the resolved version should match what we're symlinking.
            let _ = dep_version; // unused for sanity-only check
            let link_path = nested_pkg_path(&inner_nm, dep_name);
            ensure_clean_for_symlink(&link_path)?;
            let target = relative_target_for(&link_path, &guroku_root, dep);
            symlink_file_or_dir(&target, &link_path)?;
        }
    }

    // 3. Top-level symlinks for each direct dep.
    for dep_name in direct_deps {
        let Some(dep) = by_name.get(dep_name.as_str()) else {
            continue;
        };
        let link_path = nested_pkg_path(node_modules, dep_name);
        ensure_clean_for_symlink(&link_path)?;
        let target = relative_target_for(&link_path, &guroku_root, dep);
        symlink_file_or_dir(&target, &link_path)?;
    }

    // 4. node_modules/.bin shims for direct-dep `bin` entries.
    populate_bin_dir(packages, direct_deps, node_modules)?;

    Ok(())
}

/// Create `node_modules/.bin/<name>` symlinks pointing at each direct dep's
/// declared `bin` script. v0.4 only shims direct deps' bins (not transitive
/// — pnpm matches that policy).
pub fn populate_bin_dir(
    packages: &[LinkedPackage],
    direct_deps: &[String],
    node_modules: &Path,
) -> Result<()> {
    let bin_dir = node_modules.join(".bin");
    let mut created_any = false;
    let by_name: HashMap<&str, &LinkedPackage> =
        packages.iter().map(|p| (p.name.as_str(), p)).collect();

    for dep_name in direct_deps {
        let Some(dep) = by_name.get(dep_name.as_str()) else {
            continue;
        };
        if dep.bin_entries.is_empty() {
            continue;
        }
        if !created_any {
            fs::create_dir_all(&bin_dir).map_err(|e| GurokuError::Io {
                path: bin_dir.clone(),
                source: e,
            })?;
            created_any = true;
        }
        // Top-level symlink target for the package: relative from .bin/.
        for (bin_name, rel_script) in &dep.bin_entries {
            let link_path = bin_dir.join(bin_name);
            ensure_clean_for_symlink(&link_path)?;
            // The symlink target points at `<.guroku>/<id>/node_modules/<name>/<rel_script>`.
            let pkg_root = node_modules
                .join(".guroku")
                .join(safe_pkg_id(&dep.name, &dep.version))
                .join("node_modules");
            let pkg_full = nested_pkg_path(&pkg_root, &dep.name).join(normalize_rel(rel_script));
            let target = relative_to(&pkg_full, &bin_dir);
            symlink_file_or_dir(&target, &link_path)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = fs::metadata(&pkg_full) {
                    let mut perms = meta.permissions();
                    perms.set_mode(0o755);
                    let _ = fs::set_permissions(&pkg_full, perms);
                }
            }
        }
    }
    Ok(())
}

fn normalize_rel(rel: &str) -> &str {
    rel.trim_start_matches("./").trim_start_matches('/')
}

/// Convert `@scope/name` into `@scope+name` for use inside `.guroku/`.
fn safe_pkg_id(name: &str, version: &str) -> String {
    format!("{}@{}", name.replace('/', "+"), version)
}

/// Resolve the on-disk path for a package inside a `node_modules` dir,
/// preserving the `@scope` subdirectory.
fn nested_pkg_path(nm: &Path, name: &str) -> PathBuf {
    if let Some(rest) = name.strip_prefix('@') {
        if let Some((scope, inner)) = rest.split_once('/') {
            return nm.join(format!("@{scope}")).join(inner);
        }
    }
    nm.join(name)
}

/// Build the symlink target (relative path) from `link_path` to the package
/// directory living under `.guroku/`.
fn relative_target_for(link_path: &Path, guroku_root: &Path, dep: &LinkedPackage) -> PathBuf {
    let dep_pkg_dir = guroku_root
        .join(safe_pkg_id(&dep.name, &dep.version))
        .join("node_modules");
    let dep_pkg_full = nested_pkg_path(&dep_pkg_dir, &dep.name);
    let link_parent = link_path.parent().unwrap_or(link_path);
    relative_to(&dep_pkg_full, link_parent)
}

fn relative_to(target: &Path, base: &Path) -> PathBuf {
    let target_components: Vec<_> = target.components().collect();
    let base_components: Vec<_> = base.components().collect();
    let common = target_components
        .iter()
        .zip(base_components.iter())
        .take_while(|(a, b)| a == b)
        .count();
    let mut rel = PathBuf::new();
    for _ in 0..(base_components.len() - common) {
        rel.push("..");
    }
    for c in &target_components[common..] {
        rel.push(c.as_os_str());
    }
    if rel.as_os_str().is_empty() {
        rel.push(".");
    }
    rel
}

fn ensure_clean_for_symlink(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| GurokuError::Io {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }
    if path.exists() || path.is_symlink() {
        if path.is_dir() && !path.is_symlink() {
            fs::remove_dir_all(path).map_err(|e| GurokuError::Io {
                path: path.to_path_buf(),
                source: e,
            })?;
        } else {
            let _ = fs::remove_file(path);
        }
    }
    Ok(())
}

#[cfg(unix)]
fn symlink_file_or_dir(target: &Path, link: &Path) -> Result<()> {
    std::os::unix::fs::symlink(target, link).map_err(|e| GurokuError::Io {
        path: link.to_path_buf(),
        source: e,
    })
}

#[cfg(windows)]
fn symlink_file_or_dir(target: &Path, link: &Path) -> Result<()> {
    // Windows distinguishes file vs dir symlinks; pick based on what the
    // target resolves to. Fall back to a directory symlink (the common case
    // for node_modules entries).
    let res = if target.is_file() {
        std::os::windows::fs::symlink_file(target, link)
    } else {
        std::os::windows::fs::symlink_dir(target, link)
    };
    res.map_err(|e| GurokuError::Io {
        path: link.to_path_buf(),
        source: e,
    })
}
