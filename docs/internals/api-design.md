# API Design (internals)

This document is for guroku contributors thinking about the shape of the
public API: what gets re-exported, what stays in module paths, and what
stays private. It is not a user-facing API tour; for that, see
`docs/api-overview.md`.

The audience here is anyone proposing a change to a `pub` item, adding a
new `pub use` to `prelude`, or wondering whether something should be
hidden from rustdoc.

## 1. The two-level surface

guroku's public API is intentionally split into two rings.

- **Inner ring (stable).** Items re-exported from `guroku::prelude`. These
  are covered by SemVer for the v1.x line. We will not rename, restructure,
  or remove them in a minor release. Adding new items is permitted.
- **Outer ring (semi-stable).** Every `pub` item that is NOT re-exported
  through `prelude`. We try not to break these in minor releases, but we
  reserve the right to rename, move, or restructure them when the internal
  design demands it. Embedders who depend on outer-ring items should pin
  to a specific minor version and read the changelog.

Anything not marked `pub` is fully internal. There is no stability promise,
no deprecation cycle, and no rustdoc page. Reflect on whether a new type
needs to be `pub` at all before adding it.

## 2. What lands in prelude

A `pub use` into `prelude` is a strong promise. The bar is three rules,
all of which must hold:

1. **Necessary for the typical embedding flow.** The flow we optimize for
   is: read manifest, resolve dependency graph, install to a target
   directory. If an embedder cannot do that without reaching into a
   non-prelude module, the missing piece probably belongs in prelude.
2. **Signature unlikely to change shape over the v1.x line.** If we expect
   to add fields, change a return type, or split the type into two during
   v1, leaving it out of prelude protects us from a SemVer corner. The
   outer ring is where those items live until they settle.
3. **Not just a convenience.** Helpers that wrap two or three calls into
   one are usually better as documentation. If a caller can write the
   helper themselves in five lines, prelude is the wrong place.

If a candidate item fails any rule, it stays in its module. The default
is "outer ring."

## 3. What stays out of prelude

Some items are deliberately reachable only through their module path,
even though they are `pub`.

- **Internal-shape types like `LinkedPackage`.** Necessary for advanced
  embedders that want to inspect the result of linking, but the shape is
  expected to evolve as the linker grows (extra fields, splitting bin
  entry handling out, etc.). Surfacing them through prelude would tie
  our hands.
- **Error variant constructors.** `GurokuError` is `#[non_exhaustive]`
  precisely because we expect to add new variants. Re-exporting individual
  variants into prelude would either bloat the prelude or imply per-variant
  stability we don't want to promise.
- **Module-level helper functions that mostly call into other public
  items.** If `manifest::quick_parse` is "open the file then call
  `Manifest::from_str`," embedders should write the two lines.

## 4. Why `#[non_exhaustive]` on `GurokuError`

Adding a new error variant is, empirically, the single most common reason
we want to extend the API. New registry features, new lockfile validation
rules, new lifecycle script categories: each one tends to want its own
error kind so callers can distinguish it. Without `#[non_exhaustive]`,
adding a variant is a breaking change, which forces us to either bundle
new error kinds into existing variants (lossy) or wait for a major
release (slow).

With `#[non_exhaustive]` on the enum, adding a variant is non-breaking.
The cost is that every external `match` against `GurokuError` must have
a `_` arm. We accept that cost, and document it in `api-overview.md` so
embedders see the requirement up front.

```rust
match err {
    GurokuError::Io(e) => report_io(e),
    GurokuError::Manifest(e) => report_manifest(e),
    GurokuError::Resolution(e) => report_resolution(e),
    // Required because GurokuError is #[non_exhaustive].
    _ => report_unknown(&err),
}
```

The same reasoning applies to `#[non_exhaustive]` on individual struct
variants of `GurokuError`: we want freedom to add fields. If a variant
is genuinely "this will never grow," it can be left exhaustive, but the
default is `#[non_exhaustive]`.

## 5. Why `Resolved::local_source` exists at all

The `Resolved` struct represents the result of resolving a single
dependency. For registry packages, every field on `Resolved::info`
(version, integrity, tarball URL) is meaningful. For `file:` and `git:`
specifiers, several of those fields are either synthetic or absent.

We had two options.

- **Field approach (current).** Keep `Resolved` as a struct, with
  `info: VersionInfo` always present, and add `local_source:
  Option<LocalSource>`. When `local_source` is `Some`, callers know to
  treat the `info` fields as best-effort and read paths/refs from
  `local_source` instead.
- **Sealed enum approach.** Replace `Resolved` with an enum:
  ```rust
  pub enum Resolved {
      Registry { info: VersionInfo, /* ... */ },
      Local    { source: LocalSource, /* ... */ },
  }
  ```

The enum is arguably cleaner for new code, but it forces every existing
embedder that reads `r.info.version` to switch on the variant first.
Since v0.x embedders have written exactly that pattern, the field
approach preserves their code unchanged. The cost is that `local_source`
is a "tag" that callers must remember to check; we mitigate that by
documenting the convention on `Resolved` itself.

The decision is recorded here so a future contributor doesn't try to
"clean it up" without understanding the compatibility tradeoff.

## 6. `#[doc(hidden)]` policy

`#[doc(hidden)]` is for items that are technically `pub` (because the
compiler requires it for some reason) but that we don't want to encourage
callers to reach for. It is not a substitute for visibility.

Currently applied to:

- The `bin_entries` field on `LinkedPackage`. It is `pub` because the
  linker writes to it, and the linker lives in a different module from
  `LinkedPackage` itself. But no embedder should construct a
  `LinkedPackage` literal; they should treat it as opaque. Hiding the
  field from rustdoc steers them away from doing so.

When in doubt, leave the item visible in rustdoc and document the
unstable status in the doc comment. `#[doc(hidden)]` should be reached
for only when the item is genuinely a wart of the visibility system,
not a real part of the API.

## 7. Re-export hierarchy

`prelude` always re-exports from the canonical module path of the type.
That means: if `LinkedPackage` is defined in `guroku::link::linked`, the
prelude re-export is `pub use crate::link::linked::LinkedPackage;`, not
`pub use crate::link::LinkedPackage;` (even if `link` itself re-exports
the type).

Why: rustdoc follows re-exports back to the source. If we re-export an
item from a non-canonical path, rustdoc can produce confusing "see also"
links and the search index ends up with multiple entries pointing at the
same type. Sticking to canonical paths keeps rustdoc tidy.

The same rule applies to outer-ring re-exports between modules. If
`link::mod.rs` wants to re-export `linked::LinkedPackage` for ergonomics,
that's allowed, but `prelude` still imports from `link::linked`.

## 8. Adding to prelude in a minor release

Adding a new `pub use` into `prelude` is non-breaking.

- Embedders who already imported the item via its module path
  (`use guroku::link::linked::LinkedPackage;`) keep working unchanged.
- Embedders who use `use guroku::prelude::*;` get the new name visible
  in their scope. This can in theory shadow a same-named item, but the
  name we just added is the canonical one, so the conflict is the
  embedder's to resolve. We document name additions in the changelog.

The takeaway: an item can be promoted from outer ring to inner ring at
any minor release once it has settled. The reverse is not free.

## 9. Removing from prelude

Removing an item from prelude is a breaking change, regardless of
whether the item itself remains `pub` at its module path. Embedders
who wrote `use guroku::prelude::Foo;` will fail to compile.

A removal therefore requires the deprecation cycle described in
`docs/deprecation-policy.md`: mark the item `#[deprecated]` for at least
one minor release, document the replacement, and only remove the
re-export in the next major release.

## 10. Renaming an item

Renames are the trickiest case because they intersect with both prelude
and rustdoc.

The rule:

```rust
// In the canonical module:
pub struct NewName { /* ... */ }

#[deprecated(since = "1.4.0", note = "renamed to NewName")]
pub use NewName as OldName;
```

That is, `NewName` is the real definition, and `OldName` is a
`pub use` re-export with `#[deprecated]`. Old code keeps compiling
(with a warning), new code uses the new name, and rustdoc shows
`NewName` as the primary item.

Do NOT do this:

```rust
// Wrong: hides NewName from the rustdoc page that documents this module.
pub use NewName as OldName;
pub struct NewName { /* ... */ }
```

When `pub use` and the original definition share a name resolution path,
rustdoc can elide the original. Keep the `pub use` as the alias, not as
the primary export.

Remove `OldName` in v2.0, not earlier.

## 11. Async vs sync

The rule is mechanical: anything that performs I/O is `async`. Anything
that is CPU-only is sync.

- `Manifest::from_path` reads a file: async.
- `Manifest::from_str` parses a string: sync.
- `Resolver::resolve` hits the registry: async.
- `Version::satisfies` compares ranges: sync.

We do NOT add `_async` and `_sync` variants of the same function. The
async-by-default convention is consistent throughout the public API.
Embedders who need sync wrappers can build them with their runtime of
choice; we do not ship blocking shims.

This also keeps the `prelude` smaller: there is exactly one `Resolver`,
not two.

## 12. Why no traits in the public API today

guroku v1.0 ships zero `pub trait` items in the inner ring, and only
a small number of marker-style traits in the outer ring (e.g. error
conversion). Every embedding scenario we have seriously considered is
satisfied by concrete types: `Resolver`, `Linker`, `Installer`.

The reason is asymmetric: adding a trait later is non-breaking, but
turning a concrete type into a trait object (or replacing a struct with
a trait) is breaking. By starting with concrete types we keep the option
open in both directions.

If a user request makes it clear that pluggability is the right answer
(see Future considerations below), we can introduce a trait, give it a
default implementation that delegates to the existing concrete type,
and migrate.

## 13. Future considerations

These are not part of v1.0. They are recorded so that contributors
proposing similar changes can see the shape we have in mind.

- **`RegistryProvider` trait.** Once we want to abstract over npm-style
  registries and other registry shapes (JSR, a private mirror with a
  custom protocol, an in-memory test registry), a trait makes sense.
  Today, `Registry` is a concrete struct; embedders who need test
  doubles use HTTP-level mocking. A future `RegistryProvider` would be
  additive: existing `Registry` implements the trait, existing call
  sites keep working.
- **`LinkerStrategy` trait.** Once we want to support hoisted layouts
  alongside the strict layout, a strategy trait lets us select at the
  call site. Today, the linker is a single concrete implementation of
  the strict layout. Adding the trait later is non-breaking; the
  concrete linker becomes one impl of it.
- **Streaming install events.** A `pub trait InstallObserver` for
  progress reporting. The current `Installer` takes a closure; promoting
  that to a trait is additive.

In all three cases the migration looks the same: introduce the trait,
implement it for the existing concrete type, leave the concrete type
re-exported. No embedder is forced to rewrite their code on a minor
release boundary.

## Checklist for API changes

Before opening a PR that touches a `pub` item, walk through:

- Is this item necessary for the typical embedding flow, or is it an
  advanced-use case?
- If it's typical, does it belong in `prelude`? Have all three rules in
  Section 2 been satisfied?
- If it's an enum, should it be `#[non_exhaustive]`?
- If it's a struct, should any field be `#[non_exhaustive]`-equivalent
  (private + accessor)?
- Does the re-export path point at the canonical module?
- If you're renaming, is the old name a deprecated `pub use` of the new
  name, not the other way around?
- If you're adding a trait, is there a default impl path that keeps
  existing embedders working without changes?

If you cannot answer "yes" to the relevant items, the change is not
ready. Open a draft PR and tag the API design owners.
