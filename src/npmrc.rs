//! Minimal `.npmrc` reader.
//!
//! v0.4 supports the keys we actually use:
//!   - `registry=...` — default registry URL
//!   - `<scope>:registry=...` — scoped registry override
//!   - `//host/:_authToken=...` — read but currently unused (private
//!     registry support lands in v0.5)
//!
//! Lookup order matches npm/pnpm: project-local `<cwd>/.npmrc` overrides
//! `<HOME>/.npmrc`. Comments (`;` or `#`) are skipped. Values are not yet
//! interpolated for `${VAR}` references.

use crate::error::{GurokuError, Result};
use crate::registry::DEFAULT_REGISTRY;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct Npmrc {
    pub entries: BTreeMap<String, String>,
}

impl Npmrc {
    pub fn registry(&self) -> &str {
        self.entries
            .get("registry")
            .map(String::as_str)
            .unwrap_or(DEFAULT_REGISTRY)
    }

    pub fn scoped_registry(&self, scope: &str) -> Option<&str> {
        let scope = scope.trim_start_matches('@');
        self.entries
            .get(&format!("@{scope}:registry"))
            .map(String::as_str)
    }

    pub fn auth_token(&self, registry_host: &str) -> Option<&str> {
        // Match keys like `//registry.npmjs.org/:_authToken=...`
        let host = registry_host.trim_end_matches('/');
        for (k, v) in &self.entries {
            if let Some(rest) = k.strip_prefix("//") {
                if let Some(prefix) = rest.split_once('/').map(|(h, _)| h) {
                    if prefix == host && k.ends_with(":_authToken") {
                        return Some(v.as_str());
                    }
                }
            }
        }
        None
    }

    /// Read `<cwd>/.npmrc` then `~/.npmrc`, merge with project taking
    /// priority. Missing files are not errors; an empty Npmrc is returned
    /// when both are absent.
    pub fn discover(cwd: &Path) -> Result<Self> {
        let mut merged = BTreeMap::new();
        if let Some(home) = dirs::home_dir() {
            let p = home.join(".npmrc");
            if let Some(map) = read_optional(&p)? {
                merged.extend(map);
            }
        }
        let proj = cwd.join(".npmrc");
        if let Some(map) = read_optional(&proj)? {
            merged.extend(map);
        }
        Ok(Self { entries: merged })
    }

    /// Read a single `.npmrc` file and return its key/value map.
    pub fn read_from(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path).map_err(|e| GurokuError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        Ok(Self {
            entries: parse(&text),
        })
    }
}

fn read_optional(path: &Path) -> Result<Option<BTreeMap<String, String>>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(parse(&text))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(GurokuError::Io {
            path: path.to_path_buf(),
            source: e,
        }),
    }
}

pub fn parse(text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let key = k.trim().to_string();
            let val = v.trim().trim_matches('"').to_string();
            if !key.is_empty() {
                out.insert(key, val);
            }
        }
    }
    out
}

/// Convenience: `~/.npmrc`.
pub fn user_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".npmrc"))
}
