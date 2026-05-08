# NpmVersion: our pubgrub::version::Version impl

This doc explains how `NpmVersion` (a thin newtype around `node_semver::Version`)
satisfies the `pubgrub::version::Version` trait, and the design choices baked
into our `lowest()` and `bump()` implementations.

## 1. What pubgrub needs

The `pubgrub::version::Version` trait extends `Clone + Ord + Debug + Display`,
and adds two custom methods:

- `lowest()` — returns the smallest representable `Version`.
- `bump()` — returns the smallest `Version` that is strictly greater than
  `self`.

Pubgrub uses these to reason about ranges and to compute range complements
during conflict-driven backtracking.

## 2. How we satisfy each requirement

- **Clone, Debug, Ord**: derived on `NpmVersion` via the wrapped
  `node_semver::Version`. Ordering follows npm semver rules (pre-release sorts
  before the matching release; build metadata is ignored).
- **Display**: forwarded to the wrapped `Version`'s `Display` impl, so
  formatting matches `node-semver`'s canonical output.
- **lowest**: returns `0.0.0`, parsed at runtime via our own
  `parse_version("0.0.0")`. We construct it through the parser rather than a
  literal so the value flows through the same validation path as user input.
- **bump**: increments the patch component, then strips both the pre-release
  identifiers and the build metadata. The result is always a clean
  `MAJOR.MINOR.(PATCH+1)`.

## 3. Why bump strips pre-release

In npm semver, a pre-release version sorts strictly *before* the matching
release: `1.2.3-rc.1 < 1.2.3`. That asymmetry is the whole point of the
trailing identifiers — they signal "not yet the real 1.2.3."

If `bump(1.2.3-rc.1)` returned `1.2.3-rc.2`, pubgrub would compute a range
complement whose upper bound is `1.2.3-rc.2`. That complement still includes
`1.2.3` itself, because `1.2.3-rc.2 < 1.2.3`. The semantics would be confusing
and almost certainly wrong: "everything strictly greater than `1.2.3-rc.1`"
would silently exclude `1.2.3-rc.2` while including `1.2.3` and every later
release — but only by accident of where the upper bound lands.

Stripping pre-release means `bump(1.2.3-rc.1) = 1.2.4`, which sits cleanly
above both `1.2.3` (the release) and `1.2.3-rc.1` (the prerelease). The
resulting range complement is unambiguous.

## 4. Why bump increments patch, not minor

Pubgrub specifies that `bump(self)` should be the *smallest* version strictly
greater than `self`. Patch is the smallest semver component, so incrementing
patch is the most conservative choice.

Pubgrub uses `bump()` to compute range complements: the complement of
`[a, b)` is `(-inf, a) | [bump(b'), +inf)` for some boundary `b'`. An
aggressive bump (e.g. `1.2.3 -> 1.3.0`) would over-exclude every version in
`[1.2.4, 1.3.0)` from the complement, silently making them unreachable to the
solver. Patch-bump keeps the boundary tight.

## 5. Edge cases

- `bump(99.99.99) = 99.99.100`. There is no rollover into `100.0.0`. npm
  semver allows arbitrarily large patch numbers, and pubgrub only requires a
  total order, not human-friendly numbering.
- `bump(1.2.3+build.42) = 1.2.4`. Build metadata is stripped. Semver `Ord`
  ignores build metadata anyway, so leaving it in place would produce a
  version that compares equal to the bumped value's "no-metadata" form,
  breaking the strictly-greater contract.
- `bump(0.0.0) = 0.0.1`. Defined by our impl; it falls naturally out of the
  patch-increment rule and is exercised in tests.

## 6. Why a newtype

Rust's orphan rules forbid implementing a foreign trait
(`pubgrub::version::Version`) on a foreign type (`node_semver::Version`).
Neither crate is ours, so we wrap:

```rust
pub struct NpmVersion(pub node_semver::Version);
```

The newtype is intentionally thin. Everything except the trait impls
(`Version`, plus the derived/forwarded `Clone`, `Ord`, `Debug`, `Display`)
forwards to the inner type via `self.0`. There is no semantic divergence from
`node_semver::Version` — only a place to hang the trait impl.

## 7. What this enables

With `lowest()`, `bump()`, and the comparison ops in place, pubgrub can:

- Construct `Range<NpmVersion>` from arbitrary semver bounds.
- Compute range complements correctly during unit propagation, because
  `bump()` returns a semantically meaningful "next version" rather than an
  arbitrary successor.
- Backtrack across version space using the same machinery it uses for any
  other `Version` impl (Cargo, Python, etc.).

## 8. What it doesn't enable

It does *not* enable pre-release-aware backtracking. Pubgrub treats versions
as a totally ordered set; it has no notion of "include pre-releases only when
explicitly requested." That npm rule is enforced one layer up, in the
npm-Range / candidate-set translation (see `range-conversion.md` and
`pubgrub-integration.md`), not in the `Version` trait impl.

In practice: when a user writes `^1.2.3`, the candidate set we hand to
pubgrub already excludes `1.2.4-beta.1` unless the user opted in. The
`Version` trait sees only the filtered set, so its total order is enough.

## 9. Testing

The relevant test files:

- `tests/pubgrub_npm_version.rs` — basic `Version` trait checks: `lowest()`
  returns `0.0.0`, `bump()` increments patch, ordering matches semver.
- `tests/pubgrub_npm_version_bump_more.rs` — bump corner cases: pre-release
  stripping, build-metadata stripping, large patch numbers, the `0.0.0`
  case.
- `tests/pubgrub_resolver_smoke_lib.rs` — generic-bound checks that confirm
  `NpmVersion: pubgrub::version::Version` at compile time, plus a smoke test
  of the resolver running end-to-end on a tiny graph.
