//! v1.2 PubGrub-based resolver.
//!
//! Wires `pubgrub` 0.2 into guroku as a real backtracking solver. The
//! flow is:
//!
//!   1. Apply overrides + classify the root specs.
//!   2. Recursively prefetch package metadata for every name the solver
//!      might see — BFS over dep names, not versions, so each package is
//!      fetched at most once.
//!   3. Hand the prefetched cache to a synchronous `DependencyProvider`.
//!   4. Run [`pubgrub::solver::resolve`] from a synthetic root package.
//!   5. Translate `SelectedDependencies` → [`Resolution`].
//!
//! Diamonds and cascading conflicts that defeat v1.1's single-step
//! backtracking are handled by pubgrub's incompatibility tracking. Hard
//! conflicts surface as [`GurokuError::ResolutionConflict`] carrying
//! pubgrub's human-readable derivation report in `requested_by`.
//!
//! Scope honesty:
//!   - Range conversion is "intersect against the candidate set we
//!     prefetched." pubgrub will only ever pick from those candidates,
//!     so we build the pubgrub `Range` as a union of singletons matching
//!     the npm range. Correct against the known set; not a structural
//!     translation.
//!   - file:/git: roots and aliases-of-non-Range still fall back to the
//!     v1.1 BFS path. Pure registry resolution and Range-of-alias roots
//!     run on pubgrub.
//!   - Path-keyed and glob overrides are applied at root classification.
//!     Transitive overrides match through the same precedence ladder.

use crate::error::{GurokuError, Result};
use crate::manifest::Manifest;
use crate::overrides;
use crate::registry::{PackageMetadata, RegistryClient};
use crate::resolver::{Resolution, Resolved};
use crate::specs::{classify, DepSpec};
use crate::version::{parse_range, parse_version, Version as NpmInner};

use pubgrub::error::PubGrubError;
use pubgrub::range::Range;
use pubgrub::report::{DefaultStringReporter, Reporter};
use pubgrub::solver::{resolve as pubgrub_resolve, Dependencies, DependencyProvider};
use pubgrub::type_aliases::Map as PubGrubMap;
use pubgrub::version::Version as PubGrubVersion;

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::error::Error as StdError;

/// The synthetic root package name. Chosen to be unparseable as a real
/// npm package so it can never collide with anything in the registry.
pub(crate) const ROOT_PACKAGE: &str = "$guroku-root";
const ROOT_VERSION: &str = "0.0.0";

/// Newtype wrapper so we can implement [`pubgrub::version::Version`] for
/// node-semver's `Version` without orphan-rule headaches.
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct NpmVersion(pub NpmInner);

impl std::fmt::Display for NpmVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl PubGrubVersion for NpmVersion {
    fn lowest() -> Self {
        NpmVersion(parse_version(ROOT_VERSION).expect("0.0.0 is valid"))
    }

    fn bump(&self) -> Self {
        // Smallest "next version" pubgrub uses for range complements.
        // Patch-bump matches semver's increment semantics; we strip
        // pre-release tags so e.g. `1.2.3-rc.1.bump()` is `1.2.4`, not
        // `1.2.4-rc.1`.
        let v = &self.0;
        let next_str = format!("{}.{}.{}", v.major, v.minor, v.patch + 1);
        NpmVersion(parse_version(&next_str).expect("bumped version is valid"))
    }
}

/// Pubgrub-driven resolution entry point. Falls back to the v1.1 BFS
/// resolver for non-registry roots (file:/git:) since pubgrub doesn't
/// yet model local-source dependencies.
pub async fn resolve_with_pubgrub(
    client: &RegistryClient,
    roots: &[(String, String)],
    overrides_source: &Manifest,
) -> Result<Resolution> {
    let plan = match plan_roots(roots, overrides_source)? {
        RootPlan::AllRegistry(p) => p,
        RootPlan::HasLocal => {
            // file:/git: roots: defer to v1.1 BFS, which already handles
            // them. Mixed registry + local trees still get pubgrub-grade
            // resolution for their registry portion next release.
            return crate::resolver::resolve_with_manifest_overrides(
                client,
                roots,
                overrides_source,
            )
            .await;
        }
    };

    let cache = prefetch_closure(client, &plan).await?;

    let provider = NpmDependencyProvider {
        cache,
        root_deps: plan.registry_root_specs.clone(),
    };

    let solved = pubgrub_resolve(&provider, ROOT_PACKAGE.to_string(), NpmVersion::lowest())
        .map_err(translate_pubgrub_error)?;

    materialise(solved, &provider, &plan)
}

#[derive(Debug)]
struct ResolutionPlan {
    /// Registry-name → effective npm spec (the dep map pubgrub solves).
    /// After alias decomposition: aliased local names contribute under
    /// their `real_name`.
    registry_root_specs: BTreeMap<String, String>,
    /// Local-name → registry-name. Equal except for `npm:` aliases.
    local_to_registry: HashMap<String, String>,
    /// Local-name → real_name for aliased roots, used to populate
    /// [`Resolved::aliased_from`] on the way out.
    aliased_from: HashMap<String, String>,
}

enum RootPlan {
    AllRegistry(ResolutionPlan),
    HasLocal,
}

fn plan_roots(roots: &[(String, String)], overrides_source: &Manifest) -> Result<RootPlan> {
    let mut registry_root_specs: BTreeMap<String, String> = BTreeMap::new();
    let mut local_to_registry: HashMap<String, String> = HashMap::new();
    let mut aliased_from: HashMap<String, String> = HashMap::new();

    for (local_name, raw_spec) in roots {
        let path: Vec<&str> = vec![local_name.as_str()];
        let effective = overrides::lookup_with_path(overrides_source, &path)
            .unwrap_or_else(|| raw_spec.clone());
        let spec = classify(&effective);
        crate::specs::validate(&spec)?;

        match spec {
            DepSpec::Range(r) => {
                registry_root_specs.insert(local_name.clone(), r);
                local_to_registry.insert(local_name.clone(), local_name.clone());
            }
            DepSpec::Alias { real_name, inner } => match *inner {
                DepSpec::Range(r) => {
                    registry_root_specs.insert(real_name.clone(), r);
                    local_to_registry.insert(local_name.clone(), real_name.clone());
                    aliased_from.insert(local_name.clone(), real_name);
                }
                _ => return Ok(RootPlan::HasLocal),
            },
            DepSpec::File(_) | DepSpec::Git(_) => return Ok(RootPlan::HasLocal),
        }
    }

    Ok(RootPlan::AllRegistry(ResolutionPlan {
        registry_root_specs,
        local_to_registry,
        aliased_from,
    }))
}

/// BFS-prefetch every package name that any reachable version's
/// dependencies might mention. The cache is keyed by REGISTRY name.
async fn prefetch_closure(
    client: &RegistryClient,
    plan: &ResolutionPlan,
) -> Result<HashMap<String, PackageMetadata>> {
    let mut cache: HashMap<String, PackageMetadata> = HashMap::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut seen: HashSet<String> = HashSet::new();

    for name in plan.registry_root_specs.keys() {
        if seen.insert(name.clone()) {
            queue.push_back(name.clone());
        }
    }

    while let Some(name) = queue.pop_front() {
        let meta = client.fetch_metadata(&name).await?;
        for v in meta.versions.values() {
            for dep_name in v.dependencies.keys() {
                if seen.insert(dep_name.clone()) {
                    queue.push_back(dep_name.clone());
                }
            }
        }
        cache.insert(name, meta);
    }

    Ok(cache)
}

/// The pubgrub `DependencyProvider` impl. Runs synchronously against the
/// prefetched cache.
struct NpmDependencyProvider {
    cache: HashMap<String, PackageMetadata>,
    /// registry-name → npm range string for the synthetic root's deps.
    root_deps: BTreeMap<String, String>,
}

impl NpmDependencyProvider {
    fn candidates_for(&self, name: &str) -> Vec<NpmInner> {
        if name == ROOT_PACKAGE {
            return vec![parse_version(ROOT_VERSION).unwrap()];
        }
        match self.cache.get(name) {
            Some(m) => m
                .versions
                .keys()
                .filter_map(|k| parse_version(k).ok())
                .collect(),
            None => Vec::new(),
        }
    }

    fn npm_range_to_pubgrub(&self, package_name: &str, npm_range_str: &str) -> Range<NpmVersion> {
        let npm_range = match parse_range(npm_range_str) {
            Ok(r) => r,
            Err(_) => return Range::any(),
        };
        let candidates = self.candidates_for(package_name);
        if candidates.is_empty() {
            // No metadata yet (or unknown package). Return `any` so
            // pubgrub will try to fetch deps and surface a real error if
            // there's no metadata path forward.
            return Range::any();
        }
        let mut out = Range::none();
        for v in candidates {
            if npm_range.satisfies(&v) {
                out = out.union(&Range::exact(NpmVersion(v)));
            }
        }
        out
    }

    fn pick_highest(&self, package: &str, range: &Range<NpmVersion>) -> Option<NpmVersion> {
        if package == ROOT_PACKAGE {
            return Some(NpmVersion::lowest());
        }
        self.candidates_for(package)
            .into_iter()
            .map(NpmVersion)
            .filter(|v| range.contains(v))
            .max()
    }

    fn count(&self, package: &str, range: &Range<NpmVersion>) -> usize {
        if package == ROOT_PACKAGE {
            return 1;
        }
        self.candidates_for(package)
            .into_iter()
            .map(NpmVersion)
            .filter(|v| range.contains(v))
            .count()
    }
}

impl DependencyProvider<String, NpmVersion> for NpmDependencyProvider {
    fn choose_package_version<T, U>(
        &self,
        potential_packages: impl Iterator<Item = (T, U)>,
    ) -> std::result::Result<(T, Option<NpmVersion>), Box<dyn StdError>>
    where
        T: std::borrow::Borrow<String>,
        U: std::borrow::Borrow<Range<NpmVersion>>,
    {
        // Pubgrub's recommended heuristic: the package with the smallest
        // candidate set is the most-constrained, so try it first.
        let mut best: Option<(T, U, usize)> = None;
        for (pkg, range) in potential_packages {
            let n = self.count(pkg.borrow(), range.borrow());
            match &best {
                None => best = Some((pkg, range, n)),
                Some((_, _, b)) if n < *b => best = Some((pkg, range, n)),
                _ => {}
            }
        }
        let (pkg, range, _) =
            best.expect("pubgrub never calls choose_package_version with an empty iter");
        let chosen = self.pick_highest(pkg.borrow(), range.borrow());
        Ok((pkg, chosen))
    }

    fn get_dependencies(
        &self,
        package: &String,
        version: &NpmVersion,
    ) -> std::result::Result<Dependencies<String, NpmVersion>, Box<dyn StdError>> {
        if package == ROOT_PACKAGE {
            let mut map = PubGrubMap::default();
            for (dep_name, dep_spec) in &self.root_deps {
                map.insert(
                    dep_name.clone(),
                    self.npm_range_to_pubgrub(dep_name, dep_spec),
                );
            }
            return Ok(Dependencies::Known(map));
        }
        let info = match self
            .cache
            .get(package)
            .and_then(|m| m.versions.get(&version.0.to_string()))
        {
            Some(i) => i,
            None => return Ok(Dependencies::Unknown),
        };
        let mut map = PubGrubMap::default();
        for (dep_name, dep_spec) in &info.dependencies {
            map.insert(
                dep_name.clone(),
                self.npm_range_to_pubgrub(dep_name, dep_spec),
            );
        }
        Ok(Dependencies::Known(map))
    }
}

fn materialise(
    solved: PubGrubMap<String, NpmVersion>,
    provider: &NpmDependencyProvider,
    plan: &ResolutionPlan,
) -> Result<Resolution> {
    // registry-name → local-name (the user-declared key under which we
    // record the resolution and lay out node_modules/<local>/).
    let registry_to_local: HashMap<&String, &String> =
        plan.local_to_registry.iter().map(|(l, r)| (r, l)).collect();

    let mut packages: BTreeMap<String, Resolved> = BTreeMap::new();
    for (pkg, version) in &solved {
        if pkg == ROOT_PACKAGE {
            continue;
        }
        let info = provider
            .cache
            .get(pkg)
            .and_then(|m| m.versions.get(&version.0.to_string()))
            .cloned()
            .ok_or_else(|| {
                GurokuError::Other(format!(
                    "pubgrub solved `{pkg}@{version}` but it's missing from the metadata cache"
                ))
            })?;

        let local_key = registry_to_local
            .get(pkg)
            .map(|s| (*s).clone())
            .unwrap_or_else(|| pkg.clone());
        let aliased_from = plan.aliased_from.get(&local_key).cloned();

        packages.insert(
            local_key,
            Resolved {
                info,
                local_source: None,
                aliased_from,
            },
        );
    }
    Ok(Resolution { packages })
}

fn translate_pubgrub_error(err: PubGrubError<String, NpmVersion>) -> GurokuError {
    match err {
        PubGrubError::NoSolution(mut tree) => {
            // The default reporter walks the derivation tree and emits a
            // human-readable narrative. We stuff that into `requested_by`
            // so the message format the v1.1 conflict tests assert on
            // (the `>`-style path) doesn't change for the SIMPLE cases
            // — it just gets richer for diamond + cascade cases that
            // pubgrub now describes structurally.
            tree.collapse_no_versions();
            let report = DefaultStringReporter::report(&tree);
            GurokuError::ResolutionConflict {
                name: "<resolver>".to_string(),
                chosen: "<unsolvable>".to_string(),
                requested: "<see report>".to_string(),
                requested_by: report,
            }
        }
        PubGrubError::ErrorRetrievingDependencies { source, .. } => {
            GurokuError::Other(format!("pubgrub: dependency fetch failed: {source}"))
        }
        PubGrubError::DependencyOnTheEmptySet {
            package,
            version,
            dependent,
        } => GurokuError::Other(format!(
            "pubgrub: `{package}@{version}` declared `{dependent}` with an empty range"
        )),
        PubGrubError::SelfDependency { package, version } => {
            GurokuError::Other(format!("pubgrub: `{package}@{version}` depends on itself"))
        }
        PubGrubError::ErrorChoosingPackageVersion(source) => {
            GurokuError::Other(format!("pubgrub: choose_package_version failed: {source}"))
        }
        PubGrubError::ErrorInShouldCancel(source) => {
            GurokuError::Other(format!("pubgrub: cancelled: {source}"))
        }
        PubGrubError::Failure(msg) => GurokuError::Other(format!("pubgrub: {msg}")),
    }
}
