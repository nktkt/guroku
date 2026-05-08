//! v0.5 resolver.
//!
//! Walks the dependency graph BFS, picks the highest version satisfying
//! each range, and reports `ResolutionConflict` when sticky-first fails.
//! v0.5 also handles non-registry deps (`file:`, `git+`) by reading the
//! local manifest directly, plus root-level `overrides`/`resolutions`.

use crate::error::{GurokuError, Result};
use crate::git;
use crate::manifest::Manifest;
use crate::registry::{Dist, PackageMetadata, RegistryClient, VersionInfo};
use crate::specs::{classify, DepSpec};
use crate::version::{max_satisfying, parse_range, parse_version, Range, Version};
use futures::stream::{FuturesUnordered, StreamExt};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::PathBuf;
use url::Url;

#[derive(Debug, Clone)]
pub struct Resolved {
    pub info: VersionInfo,
    /// Set when the package's bytes come from a local path (file:/git:)
    /// rather than the registry. The install pipeline uses this to skip
    /// the CAS fetch+verify step.
    pub local_source: Option<PathBuf>,
}

#[derive(Debug, Default, Clone)]
pub struct Resolution {
    pub packages: BTreeMap<String, Resolved>,
}

impl Resolution {
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Resolved)> {
        self.packages.iter()
    }
    pub fn len(&self) -> usize {
        self.packages.len()
    }
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }
}

pub async fn resolve(client: &RegistryClient, roots: &[(String, String)]) -> Result<Resolution> {
    resolve_with_overrides(client, roots, &BTreeMap::new()).await
}

/// Like `resolve`, but applies the given `overrides` map to short-circuit
/// version selection by name. Used by the install command.
pub async fn resolve_with_overrides(
    client: &RegistryClient,
    roots: &[(String, String)],
    overrides: &BTreeMap<String, String>,
) -> Result<Resolution> {
    let mut chosen: HashMap<String, (Version, VersionInfo, Option<PathBuf>)> = HashMap::new();
    let mut queue: VecDeque<(String, String, Option<String>)> = VecDeque::new();

    for (name, spec) in roots {
        queue.push_back((name.clone(), spec.clone(), None));
    }

    let registry_root_names: Vec<String> = roots
        .iter()
        .filter(|(_, s)| matches!(classify(s), DepSpec::Range(_)))
        .map(|(n, _)| n.clone())
        .collect();
    let mut metadata_cache = prefetch(client, &registry_root_names)
        .await
        .unwrap_or_default();

    while let Some((name, raw_spec, requested_by)) = queue.pop_front() {
        let effective_spec = overrides
            .get(&name)
            .cloned()
            .unwrap_or_else(|| raw_spec.clone());
        let spec = classify(&effective_spec);
        crate::specs::validate(&spec)?;

        if let Some((existing, _info, _src)) = chosen.get(&name) {
            if let DepSpec::Range(r) = &spec {
                let range = parse_range_for(&name, r)?;
                if !range.satisfies(existing) {
                    return Err(GurokuError::ResolutionConflict {
                        name,
                        chosen: existing.to_string(),
                        requested: range.to_string(),
                        requested_by: requested_by.unwrap_or_else(|| "<root>".to_string()),
                    });
                }
            }
            continue;
        }

        let (version_info, source) = match spec {
            DepSpec::Range(r) => {
                let range = parse_range_for(&name, &r)?;
                if !metadata_cache.contains_key(&name) {
                    let fetched = client.fetch_metadata(&name).await?;
                    metadata_cache.insert(name.clone(), fetched);
                }
                let meta = metadata_cache.get(&name).expect("just inserted");
                let candidates = meta
                    .versions
                    .keys()
                    .filter(|k| parse_version(k).is_ok())
                    .map(String::as_str);
                let chosen_v = max_satisfying(candidates, &range).ok_or_else(|| {
                    GurokuError::NoMatchingVersion {
                        name: name.clone(),
                        spec: range.to_string(),
                    }
                })?;
                let key = chosen_v.to_string();
                let info = meta.versions.get(&key).cloned().ok_or_else(|| {
                    GurokuError::Other(format!(
                        "selected version `{key}` of `{name}` missing from metadata"
                    ))
                })?;
                (info, None)
            }
            DepSpec::File(path) => {
                let local = std::path::PathBuf::from(&path);
                let info = read_local_manifest(&local, &name)?;
                (info, Some(local))
            }
            DepSpec::Git(g) => {
                let local = git::ensure_cloned(&g)?;
                let info = read_local_manifest(&local, &name)?;
                (info, Some(local))
            }
        };

        let parsed_version = parse_version(&version_info.version).map_err(|_| {
            GurokuError::Other(format!(
                "package `{name}` has invalid version `{}`",
                version_info.version
            ))
        })?;

        // Enqueue transitive deps before recording the choice so circular
        // self-deps resolve to the same version.
        let deps: Vec<(String, String)> = version_info
            .dependencies
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        chosen.insert(name.clone(), (parsed_version, version_info, source));
        for (dep_name, dep_spec) in deps {
            queue.push_back((dep_name, dep_spec, Some(name.clone())));
        }
    }

    let mut packages = BTreeMap::new();
    for (name, (_v, info, src)) in chosen {
        packages.insert(
            name,
            Resolved {
                info,
                local_source: src,
            },
        );
    }
    Ok(Resolution { packages })
}

fn parse_range_for(name: &str, spec: &str) -> Result<Range> {
    parse_range(spec).map_err(|_| GurokuError::InvalidVersionSpec {
        name: name.to_string(),
        spec: spec.to_string(),
    })
}

/// Construct a synthetic VersionInfo from a local `package.json`. The
/// `dist.tarball` URL is a placeholder — install paths consult
/// `Resolved::local_source` to skip the fetch.
fn read_local_manifest(dir: &std::path::Path, expected_name: &str) -> Result<VersionInfo> {
    let manifest_path = dir.join("package.json");
    if !manifest_path.is_file() {
        return Err(GurokuError::FileDepMissingManifest {
            path: dir.to_string_lossy().into_owned(),
        });
    }
    let m = Manifest::read_from(&manifest_path)?;
    let name = m.name.unwrap_or_else(|| expected_name.to_string());
    let version = m.version.unwrap_or_else(|| "0.0.0-local".to_string());
    let placeholder = Url::parse("file:///guroku-local-source").expect("valid placeholder");
    Ok(VersionInfo {
        name,
        version,
        dist: Dist {
            tarball: placeholder,
            integrity: None,
            shasum: None,
        },
        dependencies: m.dependencies,
    })
}

pub async fn prefetch(
    client: &RegistryClient,
    names: &[String],
) -> Result<HashMap<String, PackageMetadata>> {
    let mut results = HashMap::new();
    let mut futs = FuturesUnordered::new();
    for name in names {
        let client = client.clone();
        let name = name.clone();
        futs.push(async move {
            let m = client.fetch_metadata(&name).await?;
            Ok::<_, GurokuError>((name, m))
        });
    }
    while let Some(r) = futs.next().await {
        let (name, meta) = r?;
        results.insert(name, meta);
    }
    Ok(results)
}
