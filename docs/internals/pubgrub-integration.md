# pubgrub Integration (v1.2)

This document describes how `guroku` integrates the `pubgrub` crate as its
resolver in v1.2, and the shape of the bridge between `pubgrub`'s synchronous,
algebraic core and our asynchronous, npm-flavored world.

## 1. Why pubgrub

The npm-style sticky-first BFS resolver that `guroku` shipped through v1.0
handles the common case well. It walks the dependency graph in breadth-first
order, picks the highest version that matches the first range it sees for a
given package, and keeps that choice "sticky" for any later range it
encounters. This is correct and fast for the 90% of trees where ranges agree.

The sticky-first BFS dies on diamond-plus-cascade conflicts. A diamond is the
classic case where two paths in the graph reach the same package with
incompatible ranges; v1.1 added a single-step backtracking pass that revisits
the most recent decision when a later range rejects it, and that fixed many
diamonds. It does not fix the cascade case, where backing out one decision
forces a different decision on a sibling, which then forces yet another
decision elsewhere. Single-step backtracking can only undo the last choice; it
has no notion of which earlier choice is to blame.

`pubgrub` is the textbook algorithm (Natalie Weizenbaum's CDCL-flavored
solver) for exactly this class of problem. It produces minimal incompatibilities
and explainable conflict traces, both of which we want.

## 2. Why pubgrub 0.2 not 0.3+

We pin `pubgrub = "0.2"` in v1.2. The `0.2.1` release is the most-stable
widely-released line; `0.3.0-alpha` exists but the API around `Range`,
`DependencyProvider::choose_version`, and the error reporter is in flux. We
prefer to integrate against a stable surface for v1.2 and bump to 0.3 in v1.3
once it is released and the changes have settled.

## 3. The async/sync bridge

`pubgrub` is synchronous. Its `DependencyProvider` trait method
`get_dependencies` returns `Result<Dependencies<...>, _>` and cannot `.await`.
Our metadata fetches are async (registry HTTP requests, ETag-aware caching,
auth headers, retries). We solve the impedance mismatch with a two-phase
resolver:

- **Phase 1 (async): prefetch closure.** Starting from the user's root deps,
  we BFS over package *names* (not versions), fetching the full versions
  document for each name we encounter, and walking each version's `dependencies`
  to discover further names. Each package name is fetched at most once per run.
  See `prefetch_closure` in `src/pubgrub_resolver.rs`.

- **Phase 2 (sync): solve.** With the prefetched cache populated, we hand it
  to a `DependencyProvider` whose `get_dependencies` and `choose_version` do
  pure cache lookups. No `.await`, no I/O, no surprises. `pubgrub::solver::resolve`
  runs to completion synchronously.

The cost is that we may fetch metadata for packages `pubgrub` would have ruled
out, but the HTTP cache absorbs most of this on warm runs (see Performance).

## 4. Range conversion

npm uses `node-semver` ranges (e.g. `^1.2.3`, `>=1.0 <2`, `1.x || 2.x`).
`pubgrub::range::Range<V>` is an algebraic union of half-open intervals.
A structural translation is possible but tedious; v1.2 takes a shortcut.

For each package, we already have the candidate set from Phase 1: the list of
versions present in the registry. We build the pubgrub `Range<NpmVersion>` as
a union of singletons (`Range::exact(v)`) for every candidate version that
satisfies the npm-semver `Range`.

This is correct but not structural. `pubgrub` only ever picks from the
candidate set, so the resulting set is identical to what a structural
translation would produce — for the purpose of *picking a version*. The
limitation shows up in conflict propagation: a structural translation
(`^1.2.3 -> between(1.2.3, 2.0.0)`) lets `pubgrub` reason about ranges
abstractly and propagate constraints further when metadata is partial. The
singleton-union form gives `pubgrub` no help there. Future work in v1.3 is
to add a structural translator alongside the singleton form.

## 5. The Version trait

`pubgrub::version::Version` requires `lowest()` (the bottom element) and
`bump()` (next-version successor) on top of `Ord`. We implement it on a
newtype `NpmVersion` that wraps `node_semver::Version`:

- `lowest()` returns `0.0.0`.
- `bump()` returns the version with `patch + 1` and pre-release tags stripped.

The bump semantics matter for `pubgrub`'s range complement: `pubgrub` uses
`bump()` to construct the open upper bound when complementing a singleton.
We strip pre-release tags because semver pre-releases sort *before* the
matching release (`1.2.3-rc.1 < 1.2.3`), so a naive bump that preserved them
could produce an upper bound that is itself less than the lower bound.

## 6. The synthetic root

`pubgrub::solver::resolve` solves a problem rooted at a single
`(package, version)` pair: "find the closure for package X version Y." Our
input is a `package.json` with a *set* of root dependencies, not a single
package.

We invent a synthetic root: `$guroku-root@0.0.0`. Its `get_dependencies`
returns the user's project dependencies. The leading `$` makes the name
unparseable as a real npm package name, so it can never collide with anything
the user might depend on. The synthetic root is filtered out of the final
`Resolution` map before returning to the caller.

## 7. Aliases

npm supports aliases via the `npm:<real>@<spec>` dependency syntax:
`"my-lodash": "npm:lodash@^4"` means "depend on `lodash` matching `^4`, but
expose it locally as `my-lodash`."

Aliases are decomposed at root-classification time, before the resolver runs.
For each alias entry we record `(local_name, real_name, spec)`. `pubgrub`
solves on `real_name` — it never sees the alias. After solving, we re-key the
`Resolution` map: each alias's entry is keyed under `local_name`, not
`real_name`, and its `aliased_from` field is populated with `real_name` from
the original decomposition.

This keeps `pubgrub`'s view clean (each package has one identity) while
preserving npm's local-name semantics in the output.

## 8. file:/git: fallback

`pubgrub` does not model local-source dependencies. `file:` and `git:` deps
have no version space — they are content-addressed by path or by commit hash,
and there is no candidate set to enumerate. We could model them as a
single-version package, but the dependencies they declare (which we can only
read after fetching the source) defeat the prefetch-closure design.

When any root is a non-`Range` non-alias-of-`Range` (i.e. a `file:` or `git:`
spec, or an alias whose target is one of those), we fall back to the v1.1
BFS resolver for the entire tree. This is documented in the entry point's
prelude (`resolve()` in `src/resolver.rs`). v1.3 may unify the paths by
treating `file:`/`git:` deps as opaque single-version packages whose
dependencies are discovered eagerly during Phase 1.

## 9. Conflict reports

When `pubgrub::solver::resolve` cannot find a solution it returns a
`PubGrubError::NoSolution(DerivationTree)`. The `DerivationTree` is the
structured proof of why no assignment exists. We render it to a human-readable
string with `pubgrub::report::DefaultStringReporter::report` and stuff the
result into `GurokuError::ResolutionConflict.requested_by`.

The structured `name`, `chosen`, and `requested` fields on
`ResolutionConflict` were designed for the v1.0 BFS resolver, which always
had a single offending package. Real `pubgrub` conflicts span multiple
packages and ranges; for `pubgrub`-sourced errors those fields are
placeholders (`name = "<pubgrub>"`, etc.) and the real content lives in
`requested_by`. Tests assert on `requested_by`.

## 10. Performance

Prefetching the closure of every reachable package name is more network work
than the v1.1 BFS path, which only fetched what its sticky-first walk asked
for. In the worst case (very wide graphs with many alternative versions per
package) the difference is significant.

Two things mitigate this:

- The HTTP cache (ETag-revalidated) absorbs most of the cost on warm runs.
  After the first install, subsequent resolutions hit `304 Not Modified` for
  unchanged registry entries.
- Most packages in a real tree have a single dominant version that the user
  ends up on; the candidate sets we enumerate are wide but the metadata
  documents themselves are not large.

Microbenchmarks in `benches/pubgrub_resolve.rs` (planned for v1.2.x) will
track the cost over representative trees and let us regression-test changes.

## 11. Determinism

`pubgrub`'s solving is deterministic given the same `DependencyProvider`
responses. Our `DependencyProvider` is deterministic given the same metadata
cache. The metadata cache is deterministic given the same registry state.

Therefore: lockfiles produced for the same `package.json` against the same
registry state are byte-identical, run-to-run and machine-to-machine. This
is a property tests in `tests/pubgrub_resolver_simple.rs` exercise.

## 12. Opt-out

Setting `GUROKU_RESOLVER=bfs` in the environment forces the v1.1
BFS-with-single-step-backtracking path, regardless of whether the input would
otherwise be eligible for `pubgrub`. This is useful for:

- Performance comparison on the same input.
- An escape hatch if `pubgrub` mis-resolves a corner case before we have a
  patched version of the resolver.

The default (unset, or any value other than `bfs`) is the `pubgrub` path,
with the file:/git: fallback described in section 8.

## 13. Test surface

The integration is covered by:

- `tests/pubgrub_npm_version.rs` — `NpmVersion::lowest`, `NpmVersion::bump`,
  and `Ord` semantics, including pre-release ordering and the strip-on-bump
  behavior.
- `tests/pubgrub_resolver_simple.rs` — public-surface compile checks, the
  determinism property, and a smoke test with a trivial dep graph.
- `tests/pubgrub_diamond_conflict.rs` — diamond conflict resolution: A and B
  both depend on C with overlapping ranges, and `pubgrub` finds the
  intersection.
- `tests/pubgrub_cascade_backtrack.rs` — the cascading backtrack case from
  section 1, the motivating example for adopting `pubgrub` at all.
- `tests/pubgrub_conflict_report_format.rs` — the shape of
  `GurokuError::ResolutionConflict.requested_by` for `pubgrub`-sourced
  errors, including the placeholder values for the structured fields.
