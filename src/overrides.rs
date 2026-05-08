//! `package.json#overrides` and `resolutions` lookup.
//!
//! v0.5 supports the simplest useful shape: a flat map from package name
//! to an exact-version string. npm and yarn have richer schemas (per-path
//! overrides like `"a > b": "1.0.0"`); those are not yet implemented.

use crate::manifest::Manifest;

/// Return the override version string for `name`, if the root manifest
/// declares one. `overrides` wins over `resolutions` when both are set.
pub fn lookup(manifest: &Manifest, name: &str) -> Option<String> {
    if let Some(v) = manifest.overrides.get(name) {
        return Some(v.clone());
    }
    if let Some(v) = manifest.resolutions.get(name) {
        return Some(v.clone());
    }
    None
}

/// Return the merged override map (overrides taking precedence). Useful
/// for inspection / `guroku audit` output.
pub fn merged(manifest: &Manifest) -> std::collections::BTreeMap<String, String> {
    let mut out = manifest.resolutions.clone();
    for (k, v) in &manifest.overrides {
        out.insert(k.clone(), v.clone());
    }
    out
}
