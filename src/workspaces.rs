//! Workspace discovery.
//!
//! v0.4 reads `package.json#workspaces` and expands its globs into a list
//! of sub-package directories. The resolver and linker do NOT yet treat
//! workspace packages as "first-class locals" (they don't get their own
//! resolution path); that lands in v0.5. What `guroku run --workspaces`
//! and tooling can rely on today is the discovered list of paths.

use crate::error::{GurokuError, Result};
use crate::manifest::Manifest;
use std::path::{Path, PathBuf};

/// One discovered workspace package.
#[derive(Debug, Clone)]
pub struct Workspace {
    pub root: PathBuf,
    pub manifest: Manifest,
}

impl Workspace {
    pub fn name(&self) -> Option<&str> {
        self.manifest.name.as_deref()
    }
}

/// Discover every workspace under `cwd` based on the root manifest's
/// `workspaces` field. Globs are evaluated relative to `cwd`; each match
/// must contain a readable `package.json`. Matches without a manifest are
/// silently skipped (so `packages/*` doesn't fail on stray dirs).
pub fn discover(cwd: &Path) -> Result<Vec<Workspace>> {
    let manifest_path = cwd.join("package.json");
    if !manifest_path.exists() {
        return Ok(vec![]);
    }
    let root_manifest = Manifest::read_from(&manifest_path)?;
    discover_with_manifest(cwd, &root_manifest)
}

pub fn discover_with_manifest(cwd: &Path, root_manifest: &Manifest) -> Result<Vec<Workspace>> {
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();

    for raw_glob in root_manifest.workspace_globs() {
        let pattern = cwd.join(&raw_glob).to_string_lossy().into_owned();
        let entries = glob::glob(&pattern).map_err(|e| {
            GurokuError::Other(format!("invalid workspaces glob `{raw_glob}`: {e}"))
        })?;
        for entry in entries {
            let path = match entry {
                Ok(p) => p,
                Err(_) => continue,
            };
            if !path.is_dir() {
                continue;
            }
            let pj = path.join("package.json");
            if !pj.is_file() {
                continue;
            }
            if !seen.insert(path.clone()) {
                continue;
            }
            let manifest = Manifest::read_from(&pj)?;
            out.push(Workspace {
                root: path,
                manifest,
            });
        }
    }

    out.sort_by(|a, b| a.root.cmp(&b.root));
    Ok(out)
}
