//! `guroku.lock` — the on-disk lockfile written by v0.2.
//!
//! The format is intentionally JSON so it round-trips cleanly through serde
//! and is trivial to inspect by hand. The schema is:
//!
//! ```json
//! {
//!   "lockfileVersion": 1,
//!   "generatedBy": "guroku 0.2.0",
//!   "packages": {
//!     "lodash@4.17.21": {
//!       "resolved": "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz",
//!       "integrity": "sha512-...",
//!       "dependencies": { "package-name": "1.2.3" }
//!     }
//!   }
//! }
//! ```
//!
//! Keys in `packages` are always `<name>@<version>`; `<version>` is exact (no
//! ranges). The values in each package's `dependencies` map are *resolved*
//! exact versions, not ranges — that is the entire point of the lockfile.

use crate::error::{GurokuError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub const LOCKFILE_NAME: &str = "guroku.lock";
pub const LOCKFILE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lockfile {
    #[serde(rename = "lockfileVersion")]
    pub lockfile_version: u32,
    #[serde(rename = "generatedBy")]
    pub generated_by: String,
    #[serde(default)]
    pub packages: BTreeMap<String, PackageLock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageLock {
    pub resolved: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integrity: Option<String>,
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
}

impl Lockfile {
    pub fn new() -> Self {
        Self {
            lockfile_version: LOCKFILE_VERSION,
            generated_by: format!("guroku {}", env!("CARGO_PKG_VERSION")),
            packages: BTreeMap::new(),
        }
    }

    pub fn read_from(path: &Path) -> Result<Self> {
        let bytes = fs::read(path).map_err(|e| GurokuError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        let lock: Self =
            serde_json::from_slice(&bytes).map_err(|e| GurokuError::ParseManifest {
                path: path.to_path_buf(),
                source: e,
            })?;
        if lock.lockfile_version != LOCKFILE_VERSION {
            return Err(GurokuError::LockfileVersionMismatch {
                found: lock.lockfile_version,
                expected: LOCKFILE_VERSION,
            });
        }
        Ok(lock)
    }

    pub fn write_to(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json + "\n").map_err(|e| GurokuError::Io {
            path: path.to_path_buf(),
            source: e,
        })
    }

    pub fn key(name: &str, version: &str) -> String {
        format!("{name}@{version}")
    }

    pub fn insert(&mut self, name: &str, version: &str, entry: PackageLock) {
        self.packages.insert(Self::key(name, version), entry);
    }

    pub fn contains(&self, name: &str, version: &str) -> bool {
        self.packages.contains_key(&Self::key(name, version))
    }
}

impl Default for Lockfile {
    fn default() -> Self {
        Self::new()
    }
}
