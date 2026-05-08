# Two-Phase Resolver: Async Prefetch + Sync Solve

This note explains how guroku bridges async metadata fetching to pubgrub's
synchronous solver. The pattern shows up in `src/pubgrub_resolver.rs` and
underlies every resolution we do in v1.2.

## 1. Why two phases

pubgrub's `DependencyProvider` trait is sync. Its methods (`choose_version`,
`get_dependencies`, etc.) cannot `.await`, so they cannot perform HTTP calls
directly.

Our `RegistryClient::fetch_metadata` is async. It is HTTP, and HTTP in tokio
is fundamentally async.

We can change neither side: pubgrub's trait is upstream, and our client must
stay async to interoperate with the rest of the runtime. So we bridge. Phase 1
fetches metadata async, upfront. Phase 2 hands a fully-loaded provider to
pubgrub for a sync solve.

## 2. The pattern

**Phase 1 (`prefetch_closure`).** Walk the dependency graph BFS by package
NAMES, not versions. Each package is fetched at most once. The output is a
`HashMap<String, PackageMetadata>` keyed by name, containing every package
pubgrub might reference during the solve.

**Phase 2 (`pubgrub::solver::resolve`).** A synchronous solve against the
cache. `DependencyProvider::get_dependencies` does pure cache lookups: it
indexes into the prefetched map, never touches the network.

The contract: if Phase 1 succeeds, Phase 2 will never miss the cache for any
name pubgrub asks about.

## 3. Why "BFS by names, not versions"

At prefetch time we do not know which versions pubgrub will pick. So we
conservatively follow EVERY edge in EVERY version's deps map. That is, for
each fetched package, we look at every published version, scan its
`dependencies` and `peerDependencies`, and enqueue every name we have not
seen.

Each name is fetched once. So a package with 200 published versions is one
HTTP round trip, not 200, because the registry returns the full packument in
a single response.

Total fetches = number of distinct package names in the closure, bounded by
the project's transitive dep count.

## 4. Cost analysis

Worst case: every transitive dep is fetched once. For a typical Node project
(~500 transitives) this is ~500 HTTP calls.

The HTTP/ETag cache (`http_cache.rs`) absorbs most of this on warm runs.
Conditional GETs return 304 Not Modified, which still costs a round trip but
no body transfer.

The v1.1 BFS resolver fetched only what it needed for the chosen path,
typically ~100-200 packages. So pubgrub costs more on cold runs. The tradeoff
is that pubgrub resolves problems v1.1 cannot — the extra fetches buy us
correct backtracking and proper conflict reporting.

## 5. Implementation walkthrough

File: `src/pubgrub_resolver.rs`. Function: `prefetch_closure`.

The structure:

- A `VecDeque<String>` queue holds names yet to fetch.
- A `HashSet<String>` seen set prevents re-enqueueing.
- The output `HashMap<String, PackageMetadata>` accumulates results.

Each iteration: pop a name, fetch its metadata, scan every version's deps
for new names, enqueue any not yet seen, repeat until the queue is empty.

Errors propagate. A 404 on any required package fails the whole solve before
pubgrub even starts; we never enter Phase 2 with an incomplete cache.

## 6. Why not lazy fetch

pubgrub's sync trait cannot await. We could use `tokio::task::block_in_place`
plus `Handle::current().block_on(...)` to run async fetches inside the sync
trait method. We rejected this for three reasons:

- It serializes fetches. Each `get_dependencies` call would block on its own
  HTTP round trip; we lose all concurrency.
- It pins us to the multi-thread runtime. `block_in_place` panics on the
  current-thread runtime, which constrains how callers embed guroku.
- It adds runtime-coupling complexity. The provider would need a `Handle`,
  the solver would need to run on a worker thread, and panics in the async
  layer would surface as opaque pubgrub errors.

The eager-prefetch approach is simpler and inherently parallel — Phase 1
can fan out fetches concurrently (we currently serialize them, but that's a
followup, not a constraint).

## 7. Alternatives considered

**Streaming pubgrub.** Write our own pubgrub-style solver that is
async-native. The maintenance burden is too high; we would be reimplementing
a well-tested algorithm to save a few hundred prefetches.

**Pre-resolved candidate sets.** Emit ranges as structural pubgrub `Range`
values upfront and skip candidate-set translation. This requires structural
range translation across the full npm semver grammar (including prerelease
and tag dispatch), which is deferred to v1.3.

**Cached metadata in Phase 2.** If a Phase 2 lookup misses, fall back to a
sync fetch via `tokio::runtime::Handle::block_on`. Considered but rejected —
the eager prefetch is reliable enough that adding a fallback path would be
dead code we'd have to maintain.

## 8. Determinism

Phase 1 is non-deterministic in fetch order — it is BFS over async streams,
and tokio scheduling is not order-stable. But the cache's *contents* are
deterministic given the same registry state: the same names map to the same
packuments regardless of fetch order.

Phase 2 is deterministic given the same cache: pubgrub's solver is a pure
function of its `DependencyProvider`.

Therefore lockfile bytes are deterministic for a given (project, registry
state) pair, which is the property our reproducibility guarantees rest on.

## 9. Error surfaces

- A network error in Phase 1 returns `GurokuError::Http` directly, with the
  underlying reqwest error attached.
- A logic error in Phase 1 (404 on a required package, malformed packument)
  returns `GurokuError::PackageNotFound` or `GurokuError::Metadata`.
- A solve error in Phase 2 returns `GurokuError::ResolutionConflict` carrying
  pubgrub's derivation report. The report is the user-facing explanation of
  *why* the solve failed.

Phase 1 errors abort before Phase 2 runs, so users never see a Phase 2 error
masking a missing package.

## 10. What this enables

Trivially: every cascading conflict pubgrub can solve. Backtracking across
multiple peer constraints, versioned-dep narrowing, etc.

With small extension: pubgrub-grade resolution against a different sync
solver, if we ever migrate off pubgrub-the-crate. The Phase 1 cache is a
generic interface; only Phase 2 is solver-specific.

Future: structural Range translation (v1.3) will let us cut the prefetch
closure aggressively. If we know `^1.2.3` rules out 90% of versions
upfront, we can also rule out their unique transitive deps, shrinking the
closure to something close to the v1.1 working set.

## 11. Testing

`tests/pubgrub_resolver_simple.rs` covers the public surface: small
synthetic dep graphs, conflict cases, and the happy path.

Network-touching tests live behind `#[ignore]` flags pending a proper test
registry. v1.2 does not ship those — they require a local registry harness
that is itself a v1.3 deliverable.
