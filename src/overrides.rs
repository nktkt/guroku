//! `package.json#overrides` and `resolutions` lookup.
//!
//! v1.1 supports three forms:
//!   - **Flat name** (npm 8+ and yarn classic): `"foo": "1.2.3"` — pin every
//!     occurrence of `foo`.
//!   - **Path-keyed** (npm 8+): `"a > b > c": "1.0.0"` — pin `c` only when
//!     reached through `a → b → c`. Whitespace around `>` is tolerated.
//!   - **Glob** (yarn classic): `"**/foo": "1.0.0"` — pin any `foo`. Today we
//!     only honour the literal `**/<name>` form; richer globs are future
//!     work.

use crate::manifest::Manifest;
use std::collections::BTreeMap;

/// Return the override version string for `name`, ignoring the path —
/// matches v1.0 behaviour. Useful for callers that don't track the
/// dependency-graph path.
pub fn lookup(manifest: &Manifest, name: &str) -> Option<String> {
    lookup_with_path(manifest, &[name])
}

/// Return the override version that matches the given path through the
/// dependency graph. `path` is the chain of names from the root to the
/// resolving package, ending with the package's own name.
///
/// Match order (highest precedence first):
///   1. exact-path key in `overrides` (e.g. `"a > b > c"`).
///   2. flat-name key in `overrides`.
///   3. exact-path key in `resolutions`.
///   4. flat-name key in `resolutions`.
///   5. glob `**/<name>` in `resolutions`.
pub fn lookup_with_path(manifest: &Manifest, path: &[&str]) -> Option<String> {
    let leaf = *path.last()?;

    if let Some(v) = match_path(&manifest.overrides, path) {
        return Some(v);
    }
    if let Some(v) = manifest.overrides.get(leaf) {
        return Some(v.clone());
    }
    if let Some(v) = match_path(&manifest.resolutions, path) {
        return Some(v);
    }
    if let Some(v) = manifest.resolutions.get(leaf) {
        return Some(v.clone());
    }
    if let Some(v) = match_glob(&manifest.resolutions, leaf) {
        return Some(v);
    }
    None
}

fn match_path(map: &BTreeMap<String, String>, path: &[&str]) -> Option<String> {
    for (key, value) in map {
        if !key.contains('>') {
            continue;
        }
        let parts: Vec<&str> = key.split('>').map(|s| s.trim()).collect();
        if parts.is_empty() {
            continue;
        }
        // Match if `parts` appears as a contiguous suffix of `path`.
        if path.len() < parts.len() {
            continue;
        }
        let tail = &path[path.len() - parts.len()..];
        if tail == parts.as_slice() {
            return Some(value.clone());
        }
    }
    None
}

fn match_glob(map: &BTreeMap<String, String>, name: &str) -> Option<String> {
    for (key, value) in map {
        if let Some(suffix) = key.strip_prefix("**/") {
            if suffix == name {
                return Some(value.clone());
            }
        }
    }
    None
}

/// Return the merged override map (overrides taking precedence). Useful
/// for inspection / `guroku audit` output. v1.0 callers using this for
/// flat lookups continue to work; they just won't see path-keyed entries
/// because the keys are kept as-is.
pub fn merged(manifest: &Manifest) -> BTreeMap<String, String> {
    let mut out = manifest.resolutions.clone();
    for (k, v) in &manifest.overrides {
        out.insert(k.clone(), v.clone());
    }
    out
}

/// Iterate the manifest's override entries, classified for diagnostics.
pub fn classify_entries(manifest: &Manifest) -> Vec<OverrideEntry<'_>> {
    let mut out = Vec::new();
    for (k, v) in &manifest.overrides {
        out.push(OverrideEntry {
            source: OverrideSource::Overrides,
            key: k,
            value: v,
            kind: classify_key(k),
        });
    }
    for (k, v) in &manifest.resolutions {
        out.push(OverrideEntry {
            source: OverrideSource::Resolutions,
            key: k,
            value: v,
            kind: classify_key(k),
        });
    }
    out
}

#[derive(Debug, Clone, Copy)]
pub enum OverrideSource {
    Overrides,
    Resolutions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverrideKind {
    Flat,
    Path,
    Glob,
    Unknown,
}

#[derive(Debug)]
pub struct OverrideEntry<'a> {
    pub source: OverrideSource,
    pub key: &'a str,
    pub value: &'a str,
    pub kind: OverrideKind,
}

fn classify_key(k: &str) -> OverrideKind {
    if k.contains('>') {
        OverrideKind::Path
    } else if k.starts_with("**/") {
        OverrideKind::Glob
    } else if k.is_empty() {
        OverrideKind::Unknown
    } else {
        OverrideKind::Flat
    }
}
