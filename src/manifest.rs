use crate::error::{GurokuError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Manifest {
    pub name: Option<String>,
    pub version: Option<String>,
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
    #[serde(default, rename = "devDependencies")]
    pub dev_dependencies: BTreeMap<String, String>,
    #[serde(default, rename = "peerDependencies")]
    pub peer_dependencies: BTreeMap<String, String>,
    #[serde(default, rename = "optionalDependencies")]
    pub optional_dependencies: BTreeMap<String, String>,
    #[serde(flatten)]
    pub other: BTreeMap<String, serde_json::Value>,
}

impl Manifest {
    pub fn read_from(path: &Path) -> Result<Self> {
        let bytes = fs::read(path).map_err(|e| GurokuError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        serde_json::from_slice(&bytes).map_err(|e| GurokuError::ParseManifest {
            path: path.to_path_buf(),
            source: e,
        })
    }

    pub fn write_to(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json + "\n").map_err(|e| GurokuError::Io {
            path: path.to_path_buf(),
            source: e,
        })
    }

    pub fn add_dependency(&mut self, name: &str, spec: &str) {
        self.dependencies.insert(name.to_string(), spec.to_string());
    }

    pub fn remove_dependency(&mut self, name: &str) -> bool {
        let a = self.dependencies.remove(name).is_some();
        let b = self.dev_dependencies.remove(name).is_some();
        let c = self.optional_dependencies.remove(name).is_some();
        a | b | c
    }

    /// Iterate over the dependency maps that the resolver should walk:
    /// `dependencies` and `devDependencies`. Peer and optional deps are
    /// excluded because the v0.2 resolver does not walk them.
    pub fn all_dependencies(&self) -> impl Iterator<Item = (&String, &String)> {
        self.dependencies.iter().chain(self.dev_dependencies.iter())
    }
}
