# Resolver Algorithm Notes (v0.2)

This document explains the dependency resolution algorithm shipped in guroku
v0.2. It is intended for contributors working on the resolver and for anyone
trying to understand why a particular `ResolutionConflict` was produced.

The implementation lives in `src/resolver.rs`. Everything described below
refers to that file.

## 1. What v0.2 ships

v0.2 performs a **breadth-first walk** over the dependency graph. The walk
maintains a single chosen-version map, a metadata cache, and a FIFO queue of
edges still to be processed.

The two defining properties of the v0.2 resolver are:

- **Sticky first choice.** The first version selected for a package is the
  version that is kept. Subsequent edges that reach the same package never
  cause a re-selection.
- **Hard error on conflict.** If a later edge demands a version range that
  excludes the already-chosen version, the resolver returns a
  `ResolutionConflict` immediately. There is no backtracking.

This keeps the resolver under ~200 lines of straight-line Rust and trivial to
reason about. It is also enough to install most npm-style projects on the
first try, which is the bar v0.2 is aiming for.

## 2. Pseudocode

The core loop, with types elided to keep things readable:

```
chosen   : map<name, (Version, VersionInfo)>
metadata : map<name, PackageMetadata>
queue    : deque<(name, Range, requested_by)>

for each (name, range) in roots:
    queue.push_back((name, range, None))

while queue not empty:
    (name, range, requested_by) = queue.pop_front()
    if name in chosen:
        if not range.satisfies(chosen[name].0):
            return Err(ResolutionConflict)
        continue
    if name not in metadata:
        metadata[name] = registry.fetch_metadata(name)
    v = max_satisfying(metadata[name].versions, range)
    info = metadata[name].versions[v]
    chosen[name] = (v, info)
    for (dep_name, dep_spec) in info.dependencies:
        queue.push_back((dep_name, parse_range(dep_spec), Some(name)))
```

`requested_by` is the package that introduced the edge, or `None` for a root.
It is not used for the algorithm itself; it is carried so that error messages
can quote the chain that led to a conflict.

`max_satisfying` is the standard "highest version in the set that satisfies
the range" function, identical in semantics to npm's implementation.

## 3. Why BFS, not DFS

The choice between BFS and DFS here is **purely cosmetic**. Both visit the
same set of edges and (because of sticky-first) make the same decisions for
each package. Correctness does not depend on the order.

We picked BFS for two small reasons:

- **Network-fetch frontier stays small in pathological graphs.** A DFS that
  recurses into the leftmost subtree first can queue up a long chain of
  outstanding metadata fetches before it gets back to the siblings. BFS
  visits siblings together, which means the set of in-flight package names
  is bounded by the width of a single graph layer rather than the depth of
  the spine.
- **Error messages read more naturally.** When a conflict is reported, the
  "requested_by" chain points back through layers of the dependency graph
  rather than down a deep, narrow path.

Neither of these is load-bearing. A future revision could swap BFS for DFS
without changing what the resolver accepts or rejects.

## 4. Why sticky-first

Once a version has been written into `chosen`, it stays. If a later edge
reaches the same package with a range that excludes the chosen version, the
resolver returns a `ResolutionConflict`.

Concretely: if the resolver has installed `react@18.3.1` in response to an
early edge, and then a later edge demands `react@^17`, v0.2 does not search
for an alternate `react` version that could satisfy both edges. It surfaces
the conflict.

This is the simplest policy that is correct on the happy path. It has two
nice properties:

- **No backtracking state.** The resolver never has to undo an installed
  package, which means no rollback, no re-fetching, and no "did we already
  try this combination?" bookkeeping.
- **Deterministic.** Given the same roots and the same registry contents,
  v0.2 always produces the same resolution or the same error. There is no
  search order that can flip an outcome.

The cost is that some resolvable graphs are reported as conflicts. Section 5
describes the canonical case.

## 5. What this gets wrong (compared to PubGrub)

The textbook example where sticky-first goes wrong is the **diamond pattern**:

```
root --> A --> C@^1
root --> B --> C@^2
```

A real solver such as PubGrub will look at this and try, in order:

1. Latest A and latest B. They disagree on C, so this combination is
   incompatible.
2. Some older A or some older B that pulls a C range compatible with the
   other branch.

If any combination of A and B versions exists that agrees on C, PubGrub will
find it. v0.2 does not look. It picks the first A or B it sees, locks in
their preferred C, and reports `ResolutionConflict` against the other
branch.

In practice this means v0.2 will sometimes refuse a project that a smarter
solver would install. Often the v0.2 verdict is the right one - the project
really is wedged and needs a manifest fix - but sometimes a downgrade of A
or B would have worked.

This is the single biggest correctness gap between v0.2 and a "real" solver,
and it is the main thing v0.3 is meant to fix.

## 6. Where the simple algorithm is fine

Despite the diamond-pattern hole, v0.2 handles the great majority of real
npm projects. There are two reasons:

- **npm graphs are shallow.** Most projects have a handful of direct
  dependencies and a transitive graph dominated by a few hubs (`react`,
  `lodash`, `webpack`-adjacent packages). The opportunity for diamond
  conflicts is smaller than it looks on paper.
- **npm is flat-when-possible.** The ecosystem norm of bumping peer
  dependencies aggressively means most active packages converge on a
  compatible set of major versions. "Stick to the first choice" lands on
  the same answer a search-based solver would, most of the time.

The expected operating mode for v0.2 is: install most projects on the first
try, and produce a clear error on the rest so the human can decide whether
to bump a manifest or wait for v0.3.

## 7. Concurrency

Inside the resolver, registry fetches happen **one at a time**. The BFS loop
calls `registry.fetch_metadata(name)` synchronously when it first encounters
a package, blocks until the metadata arrives, and then continues. This is
deliberately simple - it keeps the resolver itself single-threaded and free
of synchronization concerns.

For callers that want to overlap network with other work, the resolver
exposes `prefetch()`. Given a list of root package names, `prefetch()` warms
the metadata cache in parallel before the main resolve runs. The resolve
loop then finds those entries already populated and skips the fetch.

`prefetch()` is a cache warm-up, not a parallel resolver. It does not change
which versions are chosen.

v0.3 will move all metadata fetches behind a parallel pipeline, so that the
resolver issues fetches as soon as a new package name is observed and only
blocks the loop when it actually needs the metadata to make a decision.

## 8. Memory

Each package's metadata is fetched once and retained in the `metadata` map
until the resolve completes. There is no eviction during a resolve.

For a typical project of around 500 transitive packages, this works out to
roughly 10-30 MB of JSON parsed into Rust structs. The exact number depends
on how chatty the published metadata is - some packages list every version
ever published, with full per-version dependency tables, and those dominate.

This is acceptable for v0.2. The resolver runs once, exits, and the OS
reclaims the memory. v0.3 may stream metadata or drop fields that the
resolver has finished using, but only if profiles show it matters.

## 9. Roadmap: replacing the loop with PubGrub

The v0.3 plan is to delete the BFS loop and delegate resolution to the
`pubgrub` crate. The integration is small in surface area but touches a
couple of types:

- **`DependencyProvider` impl.** A new struct wraps the existing
  `RegistryClient` and implements the `pubgrub::DependencyProvider` trait.
  Its `get_dependencies` calls into the registry, and its
  `choose_package_version` defers to the same "highest satisfying" rule the
  v0.2 loop uses.
- **Custom `Range`.** `node_semver::Range` already does what we need for
  satisfaction checks, but `pubgrub` expects its own `Range` type with
  union and intersection. The plan is a thin newtype wrapping
  `node_semver::Range` that implements the `pubgrub::Range` trait, lifting
  the set operations through the existing semver code.
- **Error mapping.** `pubgrub::PubGrubError` carries a derivation tree.
  v0.3 will translate that into the same `ResolutionConflict` shape v0.2
  returns, so callers do not have to change.

Estimated complexity: **1-2 weeks**, dominated by the `Range` newtype and
its tests rather than the `DependencyProvider` glue.

When v0.3 lands, the diamond-pattern case in section 5 will resolve
correctly, and the "concurrency" section will collapse into "fetches happen
as the solver asks for them, in parallel".

## 10. References

- PubGrub blog post (Natalie Weizenbaum, original write-up of the algorithm
  used by Dart's pub and now widely adopted): the canonical introduction to
  why backtracking solvers can produce good error messages.
- `pubgrub` crate documentation on docs.rs: trait definitions for
  `DependencyProvider`, `Range`, and the error types.
- npm's documentation on the install algorithm: useful context for the
  "flat-when-possible" assumption that makes v0.2's sticky-first policy
  workable in practice.

These are the three documents to read before touching the resolver.
