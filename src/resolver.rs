//! v0.2 resolver.
//!
//! The algorithm is a breadth-first walk of the dependency graph. For each
//! `(name, range)` request we fetch package metadata from the registry, pick
//! the highest version satisfying `range`, and enqueue its `dependencies`.
//! The first version chosen for a name is sticky: if a later request asks for
//! the same name with a different range, we check that the already-chosen
//! version satisfies it and surface a `ResolutionConflict` if it does not.
//!
//! This is the simplest correct-on-the-happy-path resolver that also gives us
//! a real lockfile shape. It is intentionally NOT a PubGrub-grade solver:
//!  - We do not backtrack on conflicts; we report them.
//!  - We do not attempt to find a different combination that satisfies all
//!    sides of a diamond.
//!  - Peer and optional dependencies are recorded on the manifest but not
//!    walked (peers stay declarative; optionals are skipped).
//!
//! Replacing the inner solver with `pubgrub` is tracked in the v0.3 roadmap.

use crate::error::{GurokuError, Result};
use crate::registry::{PackageMetadata, RegistryClient, VersionInfo};
use crate::version::{max_satisfying, parse_range, parse_version, Range, Version};
use futures::stream::{FuturesUnordered, StreamExt};
use std::collections::{BTreeMap, HashMap, VecDeque};

/// One resolved package.
#[derive(Debug, Clone)]
pub struct Resolved {
    pub info: VersionInfo,
}

/// Output of a successful resolve: a flat map keyed by package name.
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

/// Resolve a set of root requests into a flat package set.
///
/// `roots` is the list of `(name, range_spec)` pairs from `package.json`.
pub async fn resolve(client: &RegistryClient, roots: &[(String, String)]) -> Result<Resolution> {
    let mut chosen: HashMap<String, (Version, VersionInfo)> = HashMap::new();
    let mut metadata_cache: HashMap<String, PackageMetadata> = HashMap::new();
    let mut queue: VecDeque<(String, Range, Option<String>)> = VecDeque::new();

    for (name, spec) in roots {
        let range = parse_range_for(name, spec)?;
        queue.push_back((name.clone(), range, None));
    }

    while let Some((name, range, requested_by)) = queue.pop_front() {
        // If we've already chosen a version for this name, just check
        // the new range and continue.
        if let Some((existing, _)) = chosen.get(&name) {
            if !range.satisfies(existing) {
                return Err(GurokuError::ResolutionConflict {
                    name,
                    chosen: existing.to_string(),
                    requested: range.to_string(),
                    requested_by: requested_by.unwrap_or_else(|| "<root>".to_string()),
                });
            }
            continue;
        }

        // Fetch metadata once per package name.
        if !metadata_cache.contains_key(&name) {
            let fetched = client.fetch_metadata(&name).await?;
            metadata_cache.insert(name.clone(), fetched);
        }
        let meta = metadata_cache.get(&name).expect("just inserted");

        // Pick the highest version satisfying `range`. Skip non-semver keys.
        let candidates = meta
            .versions
            .keys()
            .filter(|k| parse_version(k).is_ok())
            .map(String::as_str);
        let chosen_v =
            max_satisfying(candidates, &range).ok_or_else(|| GurokuError::NoMatchingVersion {
                name: name.clone(),
                spec: range.to_string(),
            })?;

        let chosen_str = chosen_v.to_string();
        let info = meta.versions.get(&chosen_str).cloned().ok_or_else(|| {
            GurokuError::Other(format!(
                "selected version `{chosen_str}` of `{name}` missing from metadata"
            ))
        })?;

        // Enqueue transitive deps before recording the choice so circular
        // self-deps resolve to the same version.
        let deps: Vec<(String, String)> = info
            .dependencies
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        chosen.insert(name.clone(), (chosen_v, info));
        for (dep_name, dep_spec) in deps {
            let dep_range = parse_range_for(&dep_name, &dep_spec)?;
            queue.push_back((dep_name, dep_range, Some(name.clone())));
        }
    }

    let mut packages = BTreeMap::new();
    for (name, (_v, info)) in chosen {
        packages.insert(name, Resolved { info });
    }
    Ok(Resolution { packages })
}

fn parse_range_for(name: &str, spec: &str) -> Result<Range> {
    parse_range(spec).map_err(|_| GurokuError::InvalidVersionSpec {
        name: name.to_string(),
        spec: spec.to_string(),
    })
}

/// Optional convenience: prefetch metadata for a list of root names in
/// parallel. Useful when callers want to drive concurrency themselves.
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
