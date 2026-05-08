# Single-Step Backtracking Resolver (v1.1)

Status: implemented in v1.1. This document describes the **interim** resolver
that ships in guroku 1.1. It is explicitly **not** a full PubGrub implementation.
Full PubGrub is targeted for v1.2 and will replace this code wholesale; the
public `resolve` / `resolve_with_manifest_overrides` entry points will not
change.

If you are looking for the long-term design, read `docs/internals/resolution.md`
and the v1.2 PubGrub notes once they land. This file documents what is in the
tree today.

## 1. What changed in v1.1

In v1.0, the resolver was a breadth-first walk of the dependency graph with a
"sticky-first" rule: the first time we discovered a `(name, range)` pair, we
picked the highest version of `name` satisfying `range` and recorded that
choice. Subsequent ranges for the same `name` were checked against the already
chosen version. If they were satisfied, we continued. If they were not, we
returned `ResolutionConflict` immediately.

That was sound but pessimistic. The sticky choice depended on BFS visit order,
which is not something users control or reason about. Two parents that share a
common compatible version of a leaf could fail to resolve simply because the
first parent's range admitted a higher version that the second parent did not
allow.

v1.1 keeps the BFS sticky-first scaffolding but adds **one level of
backtracking**. When a freshly discovered range conflicts with the previously
chosen version, we look for a higher (or any) version of the package that
satisfies **both** the original range and the new range. If we find one, we
substitute the chosen version. If we do not, we surface `ResolutionConflict`
the way v1.0 did, but with a richer error.

This is deliberately a small change. It is not the full algorithm. It catches
the common diamond pattern, which is what users actually hit.

## 2. The algorithm

The relevant pseudocode, expressed against the BFS work queue:

```
when (name, new_range) is queued and chosen[name] = (existing_v, existing_range):
    if new_range.satisfies(existing_v):
        continue            # happy path, no work needed

    candidates = sorted_versions_descending(name)
    for v in candidates:
        if existing_range.satisfies(v) AND new_range.satisfies(v):
            chosen[name] = (v, existing_range.union(new_range))
            break
    else:
        Err(ResolutionConflict {
            name,
            existing: existing_v,
            existing_range,
            new_range,
            requested_by: format_path(path),
        })
```

A few things to note:

- We do **not** widen `existing_range` to `new_range` and discard the original.
  The kept `original_range` is the constraint the original parent imposed; we
  must keep validating against it. We track the conjunction implicitly by
  checking both ranges in the loop.
- "Sorted descending" means newest-first. We prefer the highest version
  consistent with both ranges, mirroring npm/yarn behavior on first-pick.
- The `for / else` construction in the pseudocode mirrors a Python idiom; in
  Rust this is `if let Some(v) = ... { ... } else { return Err(...) }`.
- We do not re-enqueue the substituted version's transitive deps. See
  section 3.

In Rust the substitution path inside `try_backtrack` looks roughly like:

```rust
fn try_backtrack(
    name: &PackageName,
    existing_range: &VersionReq,
    new_range: &VersionReq,
    candidates: &[Version],
) -> Option<Version> {
    candidates
        .iter()
        .rev() // candidates are stored ascending; iterate newest first
        .find(|v| existing_range.matches(v) && new_range.matches(v))
        .cloned()
}
```

The caller then writes the new version into `ChosenEntry.version` and leaves
`original_range` untouched. The new range is conceptually folded into the
constraint set but is not stored as a separate field; on the next conflict we
will repeat the same union check using the (now even older) original range plus
the next discovered range.

## 3. Why this is not enough for full correctness

There is one significant cheat. When `try_backtrack` substitutes a new version,
we do **not** re-resolve that package's transitive dependencies. The BFS has
already enqueued the children of the previously chosen version. If the new
version of the package has a different `dependencies` map, those changes are
silently dropped on the floor.

In other words, v1.1's BFS is fundamentally append-only. It can change a leaf
choice but it cannot retract subtrees that have already been queued or
resolved. Doing that correctly is what PubGrub's conflict-driven clause
learning gives us, and it is why v1.2 is the real fix.

We ship the conservative path in v1.1 because, in practice, the diamond cases
that bite users in the wild involve packages whose transitive dependency set
is stable across the patch range that satisfies both parents. The
`lodash@^4.17.x` family is the canonical example: every 4.17.x release of
lodash has the same (empty) runtime dependency list, so substituting
4.17.20 for 4.17.21 is dependency-graph-equivalent.

The pathological case where v1.1 produces a wrong graph is documented in
section 5.

## 4. What this fixes

The motivating case is a diamond where two parents are happy with a range that
covers a single shared version of the leaf, but BFS picked the wrong version
on the first visit.

Concrete example, all versions of `lodash`:

```
A -> lodash ^4.17.0
B -> lodash ^4.17.10
```

In v1.0, BFS visits A first, picks the highest match for `^4.17.0` (say
4.17.20), and records that. Then BFS visits B, checks 4.17.20 against
`^4.17.10`, sees it satisfies, and continues. So this case actually worked in
v1.0. v1.1 takes the same path: `new_range.satisfies(existing_v)` is true on
the first check, so we never enter the backtrack loop.

The case where v1.0 **would** fail and v1.1 succeeds:

```
A -> lodash <4.17.21
B -> lodash >=4.17.20
```

If BFS visits A first, v1.0 picks the highest version satisfying `<4.17.21`,
which on a registry that includes 4.17.99 (hypothetical) or `5.0.0-beta.1`
under permissive parsing is 4.17.20 or higher; let us say it picked 5.0.0
because the range parser admitted prereleases. (In our actual range engine we
would not, but the example generalizes to any case where the v1.0 sticky pick
is a version inside `<4.17.21` that does not also satisfy `>=4.17.20`.)

When B arrives with `>=4.17.20`, v1.0 sees the existing pick fails B's range
and errors. v1.1 enters `try_backtrack`, walks candidates descending, and
finds 4.17.20 satisfies both `<4.17.21` and `>=4.17.20`. It substitutes.
Resolution succeeds.

The point of the v1.1 patch is that any time both parents have at least one
shared compatible version, we will find it, regardless of BFS visit order.

## 5. What this still fails on

Pathological diamonds where the leaf's transitive deps differ across versions.

Construct this scenario:

```
A -> leaf ^1.0      // happy with leaf 1.0.0 or 1.1.0
B -> leaf ^1.1      // requires leaf 1.1.0

leaf 1.0.0 -> X 1.x
leaf 1.1.0 -> Y 1.x
```

BFS visits A first, picks `leaf 1.0.0` (say it was the first listed in the
registry response, or had a smaller `original_range`'s preferred max), enqueues
`X 1.x`, resolves X, and moves on. Then BFS visits B, finds `1.0.0` does not
satisfy `^1.1`, runs `try_backtrack`, finds `1.1.0` satisfies both `^1.0` and
`^1.1`, and substitutes.

We now have `leaf` pinned at 1.1.0 in the resolved graph but `X` is also in
the graph. `Y` is not. The resolved graph claims `leaf 1.1.0`'s deps are
satisfied by `X`, which is a lie according to the registry.

v1.2 will fix this by retracting the subtree rooted at `leaf 1.0.0` when the
substitution fires. PubGrub does this naturally via term-set propagation.

For now, the failure mode is silently producing a graph that is internally
consistent with respect to ranges but inconsistent with respect to actual
package metadata. The lockfile writer does not currently re-fetch the
substituted version's manifest to cross-check; doing so as a defensive
measure was considered and deferred to v1.2 along with the real fix.

If you hit this in practice in v1.1, the workaround is to add an explicit
override in your manifest pinning the leaf version. Overrides are processed
through `resolve_with_manifest_overrides` and bypass the BFS sticky pick.

## 6. The `ChosenEntry` structure

`ChosenEntry` represents a finalized package selection in the resolver's
working set. v1.1 added two fields beyond what v1.0 carried.

```rust
pub struct ChosenEntry {
    /// The local name as referenced in the importing package's manifest.
    /// For aliased deps (`"lodash4": "npm:lodash@^4"`) this is `lodash4`.
    pub name: PackageName,

    /// The registry-side name. Equal to `name` except for aliases, where this
    /// is the underlying package name (`lodash`). Added in v1.1.
    pub metadata_name: PackageName,

    /// The selected version.
    pub version: Version,

    /// The range that originally selected this version. Needed by
    /// `try_backtrack` to find a version satisfying the union of the
    /// original constraint and any newly discovered constraint.
    /// Added in v1.1.
    pub original_range: VersionReq,

    /// The integrity / source descriptor recorded for the lockfile.
    pub source: ResolvedSource,
}
```

`metadata_name` was previously folded into `name` with a side table, which
made alias resolution ambiguous when the same underlying package appeared
under multiple aliases. v1.1 splits them so `try_backtrack` can fetch the
correct candidate list (`sorted_versions_descending(metadata_name)`).

`original_range` did not exist in v1.0; it was not needed because v1.0 never
re-evaluated a chosen version. v1.1 needs it to evaluate the union with the
new conflicting range.

## 7. Conflict reports include the path

When backtracking fails, `ResolutionConflict.requested_by` is now a path
string formatted by `format_path`:

```
"a > b > c"
```

The components are the chain of importers that led the resolver to this
constraint, starting from the root. v1.0 only printed the immediate parent,
which made conflicts in deep dependency trees nearly impossible to debug.

`format_path` joins the path elements with `" > "` and intentionally does not
truncate. For pathological depths (>20) the message gets long, but that is
preferable to losing context. The CLI does not wrap or truncate
`ResolutionConflict.requested_by` either; it is the user's job to widen their
terminal or pipe through `less`.

The `ResolutionConflict` struct in v1.1:

```rust
pub struct ResolutionConflict {
    pub name: PackageName,
    pub existing: Version,
    pub existing_range: VersionReq,
    pub new_range: VersionReq,
    pub requested_by: String, // path, e.g. "root > a > b"
}
```

## 8. Performance

`try_backtrack` walks the candidate version list once per conflict. The
candidate list is the registry's full version list for the package, which we
already had in memory because it was needed to make the original sticky pick.

For typical projects (tens of conflicts in a graph of hundreds of packages),
the cost is negligible. We measured low single-digit milliseconds added to
resolution time on `next.js` and `vite` template installs, well within
noise.

PubGrub is likely faster at scale because of unit propagation: it can derive
new constraints without re-walking candidate lists. But "likely faster" is
not "measurably faster on a workload we care about", and we do not have the
benchmarks yet. v1.2 will ship with comparative numbers; until then we do not
treat performance as a reason to push for PubGrub.

## 9. What v1.2 will do

Replace the BFS-with-single-step-backtrack with a true PubGrub solver. The
public interface (`resolve` and `resolve_with_manifest_overrides`) will not
change. Internally, `ChosenEntry`, `ResolutionConflict`, and the BFS work
queue will all go away, replaced by PubGrub's term store and incompatibility
list.

Migration concerns:

- `original_range` becomes redundant once PubGrub replaces the
  constraint-tracking mechanism. We will remove it.
- `metadata_name` stays. Alias handling is orthogonal to the solver.
- The path-bearing `ResolutionConflict.requested_by` stays and will be
  reconstructed from PubGrub's derivation chain instead of from the BFS
  parent stack.

The v1.1 code should be considered transitional. Bug reports against the v1.1
solver that describe the section-5 failure mode will be triaged as "fixed in
v1.2" rather than addressed in v1.1, unless they have a concrete workaround
need that overrides cannot solve.

## 10. Testing

Three integration tests cover the new behavior:

- `tests/resolver_backtracks_simple_diamond.rs` exercises a constructed
  diamond where v1.0 would have failed and v1.1 must substitute. Asserts
  the substituted version is the highest one satisfying both constraints.
- `tests/resolver_no_backtrack_when_compatible.rs` confirms the happy path
  is unchanged: when the existing pick already satisfies the new range,
  `try_backtrack` is not entered. Asserts via a counter on the test
  fixture's resolver hook.
- `tests/resolver_conflict_includes_path.rs` confirms the path-bearing
  error message: builds a chain `root > a > b > c` where `c`'s constraint
  is unsatisfiable, asserts `requested_by == "root > a > b > c"`.

The test fixtures use a fake registry implemented in
`tests/support/fake_registry.rs`. They do not hit the network. Running them
locally:

```
cargo test --test resolver_backtracks_simple_diamond
cargo test --test resolver_no_backtrack_when_compatible
cargo test --test resolver_conflict_includes_path
```

When v1.2's PubGrub solver lands, these three tests should pass unchanged.
If they do not, that is a regression in observable behavior and must be
addressed before v1.2 ships, even if PubGrub would technically produce a
different (also-correct) resolution.
