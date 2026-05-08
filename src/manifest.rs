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
    #[serde(default)]
    pub scripts: BTreeMap<String, String>,
    #[serde(default)]
    pub bin: Option<serde_json::Value>,
    #[serde(default)]
    pub workspaces: Option<serde_json::Value>,
    /// `package.json#overrides` (npm 8+) — short-circuit version selection
    /// for the named transitive dep. v0.5 supports the simple top-level
    /// `name → exact-version` form. `resolutions` (yarn) is read into the
    /// same map.
    #[serde(default)]
    pub overrides: BTreeMap<String, String>,
    #[serde(default)]
    pub resolutions: BTreeMap<String, String>,
    #[serde(flatten)]
    pub other: BTreeMap<String, serde_json::Value>,
}

impl Manifest {
    /// Normalised list of `bin` entries: `Vec<(name, relative-path)>`.
    /// Handles both forms — `"bin": "./cli.js"` (single, name from manifest)
    /// and `"bin": { "name": "./cli.js" }`. Returns empty when absent.
    pub fn bin_entries(&self) -> Vec<(String, String)> {
        match &self.bin {
            Some(serde_json::Value::String(p)) => match &self.name {
                Some(n) => {
                    let trimmed = n
                        .strip_prefix('@')
                        .and_then(|r| r.split_once('/'))
                        .map(|(_, base)| base.to_string())
                        .unwrap_or_else(|| n.clone());
                    vec![(trimmed, p.clone())]
                }
                None => vec![],
            },
            Some(serde_json::Value::Object(o)) => o
                .iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect(),
            _ => vec![],
        }
    }

    /// Normalised list of workspace globs from `package.json#workspaces`.
    /// Accepts the array form (`["packages/*"]`) and the pnpm-style object
    /// form (`{ "packages": ["packages/*"] }`). Empty when absent.
    pub fn workspace_globs(&self) -> Vec<String> {
        match &self.workspaces {
            Some(serde_json::Value::Array(a)) => a
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            Some(serde_json::Value::Object(o)) => o
                .get("packages")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            _ => vec![],
        }
    }
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
