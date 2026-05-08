# Making API Changes Safely After v1.0

This guide is for contributors who are about to touch a public item in the
`guroku` crate. It walks through the rules, the decision tree, the common
patterns we have seen, and the tooling that will catch mistakes before they
ship.

This document is distinct from two siblings:

- `docs/STABILITY.md` is the user-facing commitment. It tells consumers what
  they can rely on.
- `docs/internals/api-design.md` is the design notebook. It captures why the
  API looks the way it does.

This file is the operational guide: given a proposed change, what do you do?

## 1. The promise we made

A quick recap of what v1.0 committed us to. Read `docs/STABILITY.md` for the
full text; the bullets below are the load-bearing parts for day-to-day work.

- Items reachable through `guroku::prelude` are SemVer-covered. Adding,
  renaming, or removing names in `prelude` is a public API change.
- Items marked `#[doc(hidden)]` are not part of the SemVer commitment, even
  if they are technically `pub`. Treat them as implementation detail.
- Adding a new variant to an enum that is `#[non_exhaustive]` is a
  non-breaking change. Adding a variant to a plain enum is breaking.
- Adding a new field to a struct that is `#[non_exhaustive]` is non-breaking.
  Adding a field to a plain struct breaks any external code that constructs
  it with field-literal syntax.
- The minimum supported Rust version (MSRV) is part of the contract. Bumping
  it is a minor-version change.
- Bug fixes that change observable behavior are exempt from SemVer, but must
  be called out in `CHANGELOG.md`.

If you cannot tell whether a change is breaking, assume it is, and ask in
the PR.

## 2. The decision tree for any API change

Walk through these questions in order. Stop at the first answer that
applies.

1. **Is the changed item public?** If the item is `pub(crate)`, private,
   or only reachable through a `#[doc(hidden)]` path, you are not touching
   the public API. Skip the rest of this guide; only the normal review
   rules apply.
2. **Is the item reachable through `guroku::prelude`?** If yes, you are
   touching the SemVer-covered surface. Every remaining question matters.
   If no, but the item is still `pub` and not hidden, the same rules apply
   — `prelude` is the most visible surface, not the only covered one.
3. **Is the change additive?** Specifically:
   - a new free function, type, or trait;
   - a new method on an existing type, where the trait it belongs to is
     either ours or already had the method as a default;
   - a new field on a `#[non_exhaustive]` struct;
   - a new variant on a `#[non_exhaustive]` enum.

   If yes, the change is a minor version bump. Proceed with the checklist
   in section 4.
4. **Is the change a rename, signature change, or removal?** If yes, you
   need a deprecation cycle. See `docs/deprecation-policy.md` for the full
   procedure. The short version: keep the old name working, mark it
   `#[deprecated]`, add the new name, and schedule the removal for v2.0.
5. **Is the change motivated by a bug?** Bugs are exempt from SemVer. If a
   function was documented to do X but actually did Y, fixing it to do X is
   not a breaking change in our policy, even if some user was relying on Y.
   Document the fix in `CHANGELOG.md` under `### Fixed` and ship it. When in
   doubt about whether something is a bug or a feature someone relied on,
   ask in the PR.

## 3. Common API change patterns

These are the changes we have actually had to make. Each one shows the
moving parts.

### Adding a new function to `prelude`

Non-breaking. This is the most common API change.

```rust
// crates/guroku-core/src/prelude.rs
pub use crate::resolver::resolve_lockfile;
pub use crate::resolver::resolve_manifest;
pub use crate::resolver::resolve_workspace; // new
```

Then update the prelude stability test so the new name is locked in:

```rust
// tests/api_stability_prelude.rs
#[test]
fn prelude_contains_expected_names() {
    use guroku::prelude::*;
    let _: fn(&Path) -> Result<Lockfile, GurokuError> = resolve_lockfile;
    let _: fn(&Manifest) -> Result<Lockfile, GurokuError> = resolve_manifest;
    let _: fn(&Workspace) -> Result<Lockfile, GurokuError> = resolve_workspace;
}
```

The test does not assert behavior; it asserts that the names exist with
the expected signatures. If a future PR accidentally renames or removes
`resolve_workspace`, the test fails at compile time.

### Adding a new variant to `GurokuError`

Non-breaking, because `GurokuError` is `#[non_exhaustive]`.

```rust
// crates/guroku-core/src/error.rs
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum GurokuError {
    #[error("manifest not found at {0}")]
    ManifestNotFound(PathBuf),

    #[error("lockfile is out of date")]
    LockfileStale,

    // new
    #[error("workspace member {member} declares dependency {dep} not in lockfile")]
    WorkspaceMemberMissingDep { member: String, dep: String },
}
```

Add a kind classification arm so the variant is grouped correctly:

```rust
// tests/error_kind_classification.rs
#[test]
fn workspace_member_missing_dep_is_user_error() {
    let err = GurokuError::WorkspaceMemberMissingDep {
        member: "core".into(),
        dep: "serde".into(),
    };
    assert_eq!(err.kind(), ErrorKind::User);
}
```

If `GurokuError` were not `#[non_exhaustive]`, adding a variant would break
every external `match` that did not have a wildcard arm. This is why every
error enum we expose must be `#[non_exhaustive]`.

### Adding a new field to a struct

Depends on the struct.

If the struct is `#[non_exhaustive]`, adding a field is non-breaking, but
external code cannot construct it with field-literal syntax. We typically
provide a constructor or builder.

If the struct is plain `pub` and derives `Debug + Clone`, adding a field
breaks the field-literal constructor for downstream users:

```rust
// before
let cfg = ResolverConfig { offline: true, frozen: false };

// after we add `network_timeout`, this no longer compiles downstream
```

Three options, in order of preference:

1. Mark new structs `#[non_exhaustive]` from the start. Build a constructor
   or builder for them.
2. If the struct is already plain `pub`, defer the new field to v2.0 and
   add it to the `### BREAKING` list in `CHANGELOG.md` (see section 8).
3. Shadow the struct with a builder type that owns the new field, leaving
   the old struct alone. This is sometimes the only option for types that
   are deeply embedded in user code.

### Renaming a function

Breaking unless you go through a deprecation cycle.

```rust
// crates/guroku-core/src/resolver.rs

// new name
pub fn resolve_lockfile(path: &Path) -> Result<Lockfile, GurokuError> {
    // ... real implementation
}

// old name kept around for one minor version
#[deprecated(since = "1.4.0", note = "use `resolve_lockfile` instead")]
pub fn read_lockfile(path: &Path) -> Result<Lockfile, GurokuError> {
    resolve_lockfile(path)
}
```

Remove `read_lockfile` in v2.0, not before. The deprecation cycle is the
whole point: users get a compile-time warning, fix their code at their
pace, and are never caught off guard by a removed name.

Do not use `pub use new_name as old_name;` to satisfy the deprecation —
see anti-patterns in section 5.

### Changing a function signature

Breaking. The two acceptable paths:

- Deprecate-and-add-new. Keep the old function, add a new one with the
  new signature, mark the old one `#[deprecated]`.

  ```rust
  #[deprecated(since = "1.5.0", note = "use `install_with_options` instead")]
  pub fn install(pkg: &Package) -> Result<(), GurokuError> {
      install_with_options(pkg, &InstallOptions::default())
  }

  pub fn install_with_options(
      pkg: &Package,
      opts: &InstallOptions,
  ) -> Result<(), GurokuError> {
      // ...
  }
  ```

- Wait for v2.0. If the old signature has no reasonable wrapper around
  the new one, the change has to wait. Add it to the `### BREAKING` list.

## 4. Pre-merge checklist for an API-touching PR

Every PR that adds, removes, or changes a public item must satisfy this
list before merge. The `api_change.md` PR template embeds it.

- [ ] Used the `api_change.md` PR template (not the default template).
- [ ] `cargo-semver-checks` workflow is green. If it flags a change you
      believe is intentional, document the justification in the PR
      description and request a second reviewer.
- [ ] The relevant `tests/api_stability_*.rs` files are updated. Adding a
      name to `prelude` without updating `api_stability_prelude.rs` is a
      common mistake; the test does not catch missing names by itself.
- [ ] `CHANGELOG.md` `[Unreleased]` section has an entry under the right
      heading: `### Added`, `### Changed`, `### Deprecated`, `### Removed`,
      `### Fixed`, or `### BREAKING`.
- [ ] Rustdoc for the changed item has been read. New items have a doc
      comment with at least one example. Changed items have updated
      examples. Deprecated items have a `# Deprecated` section pointing
      to the replacement.

## 5. Anti-patterns

These are mistakes we have caught in review. They are easy to make and
easy to miss.

- **Do not write `pub use NewName as OldName;` for renames.** It compiles,
  it works, but it hides the new name from rustdoc — users searching the
  docs see only `OldName`, defeating the purpose of the rename. Use a
  separate `#[deprecated]` wrapper instead.
- **Do not add `pub(crate)` to a previously-`pub` item.** That is a
  removal, even if the item is still defined. Go through the deprecation
  policy or wait for v2.0.
- **Do not break a doctest just to "tighten" a signature.** A doctest is a
  promise that the example in the documentation works. If a signature
  change makes the doctest fail, users with similar code will also fail.
  Either keep the old signature, or update the doctest *and* deprecate the
  old signature.
- **Do not refactor for "cleanliness" if it changes a public function's
  parameter list.** Renaming a parameter is fine (parameter names are not
  part of the SemVer contract for free functions), but reordering or
  retyping is breaking. Internal cleanups should not leak through the
  public surface.
- **Do not add a new required argument to a public function.** Even if you
  think no one uses it, you do not know that. Add an optional argument via
  an `Options` struct, or add a new function.
- **Do not change the bound on a public generic.** `fn foo<T: Read>` to
  `fn foo<T: Read + Send>` is breaking — code that called `foo` with a
  non-`Send` reader stops compiling.

## 6. The `#[non_exhaustive]` escape hatch

`#[non_exhaustive]` is the single most useful tool we have for keeping the
API extensible. It tells the compiler that an external user must write a
wildcard arm or use struct-update syntax, which means we can add things
later without breaking them.

Use it for:

- **Any error enum.** `GurokuError`, `ResolverError`, `LockfileError`, and
  every other `*Error` we expose are `#[non_exhaustive]`. New error
  conditions appear all the time; we must be able to add variants without
  a major version bump.
- **Any "kind" enum.** `ErrorKind`, `DependencyKind`, `PackageKind`. These
  exist precisely to be matched on; new kinds will appear.
- **Structs that are likely to grow fields.** `ResolverConfig`,
  `InstallOptions`, anything with the shape of "a bag of knobs". The cost
  is real: external code cannot use field-literal construction. We pair
  these with a `Default` impl and either a builder or a `with_*` method
  per field.

Do not use it for:

- Simple data types where the set of fields is the type's identity.
  `Version` has `major`, `minor`, `patch`. It will not grow. Marking it
  `#[non_exhaustive]` would be cargo-culting.
- Sealed traits or marker types where extension is explicitly not
  desired.

A note on testing: `#[non_exhaustive]` on a struct prevents external
field-literal construction, which can be inconvenient when writing
integration tests in another crate. Provide a `pub(crate)` constructor
that bypasses the restriction for our own test crates, or expose a
`for_testing` constructor under a `testing` feature flag.

## 7. The `#[doc(hidden)]` strategy

`#[doc(hidden)]` is for items that are technically `pub` (because some
internal mechanism, often a macro, needs them to be) but that we do not
want users to rely on. They are not part of the SemVer commitment.

Typical uses:

- Macro support code. A `macro_rules!` macro that expands to a call to
  `$crate::__internal::do_thing` needs `do_thing` to be `pub`, but the
  user should never call it directly. Mark it `#[doc(hidden)]`.
- Re-exports needed for trait method resolution that we do not want to
  document.

Rules of thumb:

- Prefer `pub(crate)` whenever you can. If the item does not need to leave
  the crate, do not let it.
- Use `#[doc(hidden)]` when `pub(crate)` is not enough — for example,
  when a macro needs to expand to a path that resolves in a downstream
  crate.
- Document, in a comment next to the item, why it is `pub` rather than
  `pub(crate)`. Future contributors will thank you.
- Never put a stable, useful item behind `#[doc(hidden)]`. If users
  discover it (and they will), they will use it, and we will end up
  supporting it anyway.

## 8. Coordinating with v2.0

Some changes have to wait for a major version. Rather than scattering
TODOs through the codebase, we keep a running list in `CHANGELOG.md`:

```markdown
## [Unreleased]

### BREAKING

- Remove `GurokuError::read_lockfile` (deprecated since 1.4.0).
- Make `ResolverConfig` `#[non_exhaustive]` and require construction via
  `ResolverConfig::builder()`.
- Replace `Lockfile::version()` `-> u32` with `-> LockfileVersion`.
```

Two effects fall out of this:

1. The migration guide for v2.0 writes itself. When it is time to cut the
   release, copy the `### BREAKING` section into a migration document and
   flesh out each entry with before/after code.
2. The list serves as a forcing function for design conversations. If the
   list is getting long, it is time to talk about scheduling v2.0. If it
   is empty, we have no reason to bump the major.

When you add an entry to `### BREAKING`, link it to a tracking issue. The
issue is where the actual implementation discussion happens.

## 9. Tooling

The CI pipeline runs all of these. You can run them locally before
pushing.

- **`cargo doc --no-deps`.** The single most important command. It builds
  the same documentation that ends up on docs.rs. If you are adding a new
  public item, run this and read what it produces. Pay attention to:
  examples that fail to compile, broken intra-doc links, and items that
  appear in the docs but should be `#[doc(hidden)]`.

  ```bash
  cargo doc --no-deps --workspace --open
  ```

- **`cargo public-api`.** Third-party tool. Lists every public item in
  the crate, one per line, in a stable form. Diffing the output before
  and after a change is the clearest way to see what the API change
  actually is.

  ```bash
  cargo install cargo-public-api
  cargo public-api --simplified > /tmp/before.txt
  # apply your change
  cargo public-api --simplified > /tmp/after.txt
  diff /tmp/before.txt /tmp/after.txt
  ```

- **`cargo-semver-checks`.** Compares the current public API against the
  last published version on crates.io and flags SemVer violations. This
  runs in CI on every PR; you should also run it locally before opening
  a PR that touches the API.

  ```bash
  cargo install cargo-semver-checks --locked
  cargo semver-checks check-release
  ```

  The tool is not perfect — it sometimes misses changes that depend on
  generic bounds or trait coherence — but it catches the obvious
  mistakes, and "cargo-semver-checks is green" is a precondition for
  merge.

- **`cargo +nightly rustdoc -- -Z unstable-options --output-format json`.**
  For when you need to inspect the API surface programmatically. We use
  this in `tests/api_stability_*.rs` to assert structural properties of
  the public API.

If a tool flags something you believe is wrong, do not silence it. Open
an issue, write a failing test that captures the case, and discuss in
review. The tools exist because we are not careful enough on our own.
