//! v1.1 resolver.
//!
//! Walks the dependency graph BFS, picks the highest version satisfying
//! each range, and on conflict tries to *downgrade* the previously chosen
//! version of the conflicting package to one that still satisfies all
//! known constraints (this is the v1.1 "single-step backtracking"
//! upgrade — full PubGrub integration is targeted for v1.2).
//!
//! Three v1.1 features land here:
//!  - `DepSpec::Alias { real_name, inner }` is honoured: the registry
//!    lookup uses `real_name`, but the resolution is recorded under the
//!    declared local key.
//!  - Path-keyed overrides (`"a > b": "1.0.0"`) and glob resolutions
//!    (`"**/foo"`) are applied via [`crate::overrides::lookup_with_path`].
//!  - Conflict reports include the dependency path that triggered the
//!    conflict for easier debugging.

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
    /// Set when this entry came from an `npm:<real>@<spec>` alias. Holds
    /// the registry name (which differs from the map key the user typed).
    pub aliased_from: Option<String>,
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

/// Like `resolve`, but applies the given simple `overrides` map before
/// classification. v1.1 callers that have a richer override map should
/// use [`resolve_with_manifest_overrides`] instead, which honours
/// path-keyed and glob entries.
pub async fn resolve_with_overrides(
    client: &RegistryClient,
    roots: &[(String, String)],
    overrides: &BTreeMap<String, String>,
) -> Result<Resolution> {
    // Wrap the simple overrides map as a synthetic Manifest so we can
    // reuse the path-aware lookup.
    let mut synthetic = Manifest::default();
    for (k, v) in overrides {
        synthetic.overrides.insert(k.clone(), v.clone());
    }
    resolve_with_manifest_overrides(client, roots, &synthetic).await
}

/// Resolution path: honours overrides + resolutions on the supplied
/// manifest, including path-keyed and glob entries.
pub async fn resolve_with_manifest_overrides(
    client: &RegistryClient,
    roots: &[(String, String)],
    overrides_source: &Manifest,
) -> Result<Resolution> {
    let mut chosen: HashMap<String, ChosenEntry> = HashMap::new();

    // Each queue entry carries the resolution path (chain of names from
    // root to this dep) so we can a) cite it in conflict reports, and
    // b) apply path-keyed overrides correctly.
    let mut queue: VecDeque<(String, String, Vec<String>)> = VecDeque::new();
    for (name, spec) in roots {
        queue.push_back((name.clone(), spec.clone(), vec![name.clone()]));
    }

    let registry_root_names: Vec<String> = roots
        .iter()
        .filter_map(|(n, s)| match classify(s) {
            DepSpec::Range(_) => Some(n.clone()),
            DepSpec::Alias { real_name, inner } => {
                if matches!(*inner, DepSpec::Range(_)) {
                    Some(real_name)
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect();
    let mut metadata_cache = prefetch(client, &registry_root_names)
        .await
        .unwrap_or_default();

    while let Some((local_name, raw_spec, path)) = queue.pop_front() {
        // Resolve overrides against the path (path-keyed) AND against the
        // leaf name (flat / glob).
        let path_refs: Vec<&str> = path.iter().map(String::as_str).collect();
        let effective_spec = crate::overrides::lookup_with_path(overrides_source, &path_refs)
            .unwrap_or_else(|| raw_spec.clone());

        let spec = classify(&effective_spec);
        crate::specs::validate(&spec)?;

        // Decompose alias: registry-side name vs local-side name.
        let (registry_name, inner_spec, alias_real) = match spec {
            DepSpec::Alias { real_name, inner } => (real_name.clone(), *inner, Some(real_name)),
            other => (local_name.clone(), other, None),
        };

        if let Some(existing) = chosen.get(&local_name) {
            if let DepSpec::Range(r) = &inner_spec {
                let range = parse_range_for(&local_name, r)?;
                if !range.satisfies(&existing.version) {
                    // v1.1: try a single-step backtrack — pick a different
                    // version of `existing` that satisfies BOTH the original
                    // range and the new one. If we can find one, replace and
                    // continue; otherwise surface the conflict with the path.
                    if let Some(downgrade) = try_backtrack(
                        &metadata_cache,
                        &existing.metadata_name,
                        &existing.original_range,
                        &range,
                    ) {
                        chosen.insert(local_name.clone(), downgrade);
                        // Re-process this constraint at the new version.
                        // (We don't enqueue transitive deps from the old
                        // version; correctness requires the caller to drive
                        // a fresh resolve in a future PubGrub-grade solver.
                        // For now we only verify the new version satisfies
                        // both constraints — sufficient for the diamond case
                        // where the only difference is the range.)
                        continue;
                    }
                    return Err(GurokuError::ResolutionConflict {
                        name: local_name,
                        chosen: existing.version.to_string(),
                        requested: range.to_string(),
                        requested_by: format_path(&path),
                    });
                }
            }
            continue;
        }

        let (version_info, source) = match inner_spec {
            DepSpec::Range(r) => {
                let range = parse_range_for(&registry_name, &r)?;
                if !metadata_cache.contains_key(&registry_name) {
                    let fetched = client.fetch_metadata(&registry_name).await?;
                    metadata_cache.insert(registry_name.clone(), fetched);
                }
                let meta = metadata_cache.get(&registry_name).expect("just inserted");
                let candidates = meta
                    .versions
                    .keys()
                    .filter(|k| parse_version(k).is_ok())
                    .map(String::as_str);
                let chosen_v = max_satisfying(candidates, &range).ok_or_else(|| {
                    GurokuError::NoMatchingVersion {
                        name: registry_name.clone(),
                        spec: range.to_string(),
                    }
                })?;
                let key = chosen_v.to_string();
                let info = meta.versions.get(&key).cloned().ok_or_else(|| {
                    GurokuError::Other(format!(
                        "selected version `{key}` of `{registry_name}` missing from metadata"
                    ))
                })?;
                let chosen_entry = ChosenEntry {
                    version: chosen_v,
                    info: info.clone(),
                    metadata_name: registry_name.clone(),
                    original_range: range,
                    local_source: None,
                    aliased_from: alias_real.clone(),
                };
                let chosen_for_alias = chosen_entry.clone();
                chosen.insert(local_name.clone(), chosen_entry);
                let deps: Vec<(String, String)> = chosen_for_alias
                    .info
                    .dependencies
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                for (dep_name, dep_spec) in deps {
                    let mut child_path = path.clone();
                    child_path.push(dep_name.clone());
                    queue.push_back((dep_name, dep_spec, child_path));
                }
                if let Some(real) = alias_real {
                    record_alias(&mut chosen, &local_name, &real);
                }
                continue;
            }
            DepSpec::File(path_str) => {
                let local = std::path::PathBuf::from(&path_str);
                let info = read_local_manifest(&local, &local_name)?;
                (info, Some(local))
            }
            DepSpec::Git(g) => {
                let local = git::ensure_cloned(&g)?;
                let info = read_local_manifest(&local, &local_name)?;
                (info, Some(local))
            }
            DepSpec::Alias { .. } => {
                // Already unwrapped above.
                unreachable!("nested alias spec; unwrap_alias should have stripped");
            }
        };

        let parsed_version = parse_version(&version_info.version).map_err(|_| {
            GurokuError::Other(format!(
                "package `{local_name}` has invalid version `{}`",
                version_info.version
            ))
        })?;

        // Local-source case (file:/git:): synthesise a permissive range
        // for backtracking purposes (a real range is meaningless for
        // local sources).
        let trivial_range = parse_range("*").expect("`*` parses as a Range");
        let chosen_entry = ChosenEntry {
            version: parsed_version,
            info: version_info.clone(),
            metadata_name: registry_name.clone(),
            original_range: trivial_range,
            local_source: source,
            aliased_from: alias_real.clone(),
        };
        chosen.insert(local_name.clone(), chosen_entry.clone());
        for (dep_name, dep_spec) in &version_info.dependencies {
            let mut child_path = path.clone();
            child_path.push(dep_name.clone());
            queue.push_back((dep_name.clone(), dep_spec.clone(), child_path));
        }
        if let Some(real) = alias_real {
            record_alias(&mut chosen, &local_name, &real);
        }
    }

    let mut packages = BTreeMap::new();
    for (name, entry) in chosen {
        packages.insert(
            name.clone(),
            Resolved {
                info: entry.info,
                local_source: entry.local_source,
                aliased_from: entry.aliased_from,
            },
        );
    }
    Ok(Resolution { packages })
}

#[derive(Debug, Clone)]
struct ChosenEntry {
    version: Version,
    info: VersionInfo,
    metadata_name: String,
    original_range: Range,
    local_source: Option<PathBuf>,
    #[doc(hidden)]
    aliased_from: Option<String>,
}

impl ChosenEntry {
    // Default `aliased_from` field on construction without alias info.
}

// Helper to set the alias bookkeeping post-hoc.
fn record_alias(chosen: &mut HashMap<String, ChosenEntry>, local_name: &str, real_name: &str) {
    if let Some(entry) = chosen.get_mut(local_name) {
        entry.aliased_from = Some(real_name.to_string());
    }
}

/// Try to find a version of `name` that satisfies BOTH `existing_range`
/// (the original constraint that produced the previously chosen version)
/// and the `new_range` (a freshly-discovered conflicting constraint).
/// Returns a fully-populated [`ChosenEntry`] on success.
fn try_backtrack(
    metadata_cache: &HashMap<String, PackageMetadata>,
    name: &str,
    existing_range: &Range,
    new_range: &Range,
) -> Option<ChosenEntry> {
    let meta = metadata_cache.get(name)?;
    let mut keys: Vec<&str> = meta
        .versions
        .keys()
        .filter(|k| parse_version(k).is_ok())
        .map(String::as_str)
        .collect();
    keys.sort_by(|a, b| parse_version(b).unwrap().cmp(&parse_version(a).unwrap()));
    for key in keys {
        let v = parse_version(key).ok()?;
        if existing_range.satisfies(&v) && new_range.satisfies(&v) {
            let info = meta.versions.get(key)?.clone();
            return Some(ChosenEntry {
                version: v,
                info,
                metadata_name: name.to_string(),
                // Combined range we're now committed to. Persist a copy of
                // `new_range` since it's strictly the union narrower than
                // existing_range alone.
                original_range: new_range.clone(),
                local_source: None,
                aliased_from: None,
            });
        }
    }
    None
}

fn parse_range_for(name: &str, spec: &str) -> Result<Range> {
    parse_range(spec).map_err(|_| GurokuError::InvalidVersionSpec {
        name: name.to_string(),
        spec: spec.to_string(),
    })
}

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

fn format_path(path: &[String]) -> String {
    if path.is_empty() {
        "<root>".to_string()
    } else {
        path.join(" > ")
    }
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
