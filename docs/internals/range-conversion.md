# Range Conversion: npm-semver to pubgrub

How npm-semver `Range` strings are translated into pubgrub `Range<NpmVersion>`
for the v1.2 resolver.

## 1. The problem

We have two different "set of versions" representations:

- **npm-semver Range**: caret/tilde/comparator strings like `^1.2.3`, `~2.0`,
  `>=1 <2`, `1.x`, `^1 || ^2`. Also includes dist-tag references (`latest`,
  `next`) which name a specific version rather than a range.
- **pubgrub `Range<V>`**: a structured set built from `any`, `none`,
  `exact(v)`, `higher_than(v)`, `strictly_lower_than(v)`, `between(v1, v2)`,
  combined with `union`, `intersection`, and `negate`.

The resolver speaks pubgrub; the registry speaks npm-semver. We need to
translate npm Range to pubgrub Range every time a dependency edge enters the
solver.

## 2. Two strategies

**Structural translation.** Walk the npm Range's parsed comparator tree and
emit equivalent pubgrub Range constructors (`>=1 <2` becomes
`between(1.0.0, 2.0.0)`, etc.).

- Pros: Works without registry metadata. Lazy-friendly.
- Cons: Requires faithfully implementing every npm Range shape, including
  subtleties like `^0.x.y` (which does NOT widen to `>=0.x.0 <1`) and
  prerelease-ordering rules. Mistakes here are silent: pubgrub will happily
  pick a version we shouldn't have allowed.

**Candidate-set translation.** For each candidate version we already know
about (from prefetched registry metadata), keep a singleton in the pubgrub
Range iff `npm_range.satisfies(v)`. Union the singletons.

- Pros: Trivially correct semantics — we delegate to the npm-semver
  implementation we already trust.
- Cons: Requires knowing the candidate set up front. For a package with no
  fetched metadata yet, the best we can return is `Range::any()`.

## 3. What v1.2 ships: candidate-set translation

Implementation: `NpmDependencyProvider::npm_range_to_pubgrub` in
`src/pubgrub_resolver.rs`.

The flow:

1. Call `parse_range` (in `src/version.rs`) to get the npm Range. On parse
   error, return `Range::any()` rather than poisoning the solver — pubgrub
   will reject the eventual choice when no candidate ends up matching, which
   surfaces as a clean `NoSolution`.
2. Look up the package's candidate versions in the prefetched cache via
   `candidates_for(name)`.
3. For each candidate version satisfying the npm Range (using
   `Range::satisfies`), union a `Range::exact(v)` singleton into the result.
4. If the candidate set is empty (we have no metadata yet), return
   `Range::any()`. pubgrub will trip when it tries to pick a version and none
   exists; this surfaces as a `NoSolution` with the missing package
   highlighted in the derivation tree.

## 4. Why we accept the cost

pubgrub only ever picks a version from the candidate set we supply via
`choose_package_version`. So a Range built as a union of candidate
singletons is semantically identical to a structural Range — within the
bounds of what pubgrub will ever query.

The cost is `O(|candidates|)` per range conversion. Each version in the
metadata becomes one `Range::exact` plus one `union`. For a 200-version
package this is roughly 200 set unions per range. pubgrub does the merges
internally as a sorted segment tree, so it stays cheap in absolute terms.

## 5. Worked examples

Candidate set: `[1.0.0, 1.2.0, 1.2.3, 1.5.0, 2.0.0]`.

- `^1.2.3` produces a Range covering `{1.2.3, 1.5.0}`. (Excludes 1.0.0 and
  1.2.0 as too low, 2.0.0 as too high.)
- `^1 || ^2` produces `{1.0.0, 1.2.0, 1.2.3, 1.5.0, 2.0.0}`.
- `*` produces all of them.
- `next` (a dist tag) typically produces empty: dist tags are not versions,
  and `parse_range` rejects them. The prefetcher resolves dist tags to
  concrete versions BEFORE handing them to pubgrub, so this code path
  shouldn't fire in practice.

## 6. Edge cases the candidate-set translation handles correctly

- **Pre-release versions.** Only included if the npm Range explicitly admits
  them, per node-semver's prerelease-ordering rules. We get this for free
  because we delegate to `Range::satisfies`.
- **Build metadata.** Ignored by both npm-semver comparison and pubgrub's
  `Ord` impl on `NpmVersion`. No special handling needed.
- **Empty candidate set.** Return `Range::any()` so the solver doesn't trip
  immediately on a package whose metadata hasn't been fetched yet. pubgrub
  surfaces a clean error during version selection if no candidate
  ultimately exists.

## 7. Edge cases v1.2's translation does NOT handle structurally

- **Constraint propagation across un-fetched packages.** pubgrub can
  normally infer "if X requires `Y@>=2` and Z requires `Y@<2`, no
  solution" without enumerating Y's versions. With the candidate-set
  translation that inference requires Y's metadata to be present, because
  both ranges become unions-of-singletons keyed on Y's actual versions.
  Our prefetch is closure-of-all-reachable-names, so we DO eventually
  fetch Y, but only after seeing X and Z reference it.
- **Performance under aggressive prefetch pruning.** If a future v1.3
  decides to skip metadata fetches for packages it has reason to believe
  won't be selected, we'll need a structural translator that doesn't
  depend on knowing the candidate set. Today we eat the prefetch cost and
  keep the translation simple.

## 8. Future work (v1.3)

- A structural Range translator for the npm-semver shapes we care about.
  Then prefetch can be lazier (only fetch packages we haven't pruned yet).
- Caching translated Ranges per `(package, npm-range-string)` pair, so a
  popular range like `^1.0.0` doesn't re-walk the candidate list every time
  a new dependency edge cites it.

## 9. References

- `src/pubgrub_resolver.rs` — `NpmDependencyProvider::npm_range_to_pubgrub`.
- `src/version.rs` — `parse_range`, `parse_version`, and the
  `Range::satisfies` bridge.
