# Deprecation Policy

This document describes how guroku deprecates and eventually removes things
after the 1.0 release. It is a companion to `STABILITY.md`, which defines
*what* guroku promises to keep stable; this document defines *how* we walk
back from those promises when we need to.

The short version: nothing covered by SemVer disappears in a minor release.
Anything we want to remove goes through a deprecation cycle that lands across
two minor releases minimum and culminates in a major version bump.

## 1. The promise

The following surfaces are covered by SemVer and therefore cannot be removed
in a minor release:

- The lockfile schema documented in `docs/lockfile-format.md`.
- The Rust prelude API: every item re-exported by `guroku::prelude`.
- The documented CLI surface: every subcommand and every flag listed in
  `docs/cli-reference.md`.

Removing any of these requires one of the following:

- A major version bump on its own. This is allowed but unfriendly: users get
  no warning, and we expect to do this only for security or correctness
  reasons that make a deprecation cycle infeasible.
- A deprecation cycle that culminates in a major version bump. This is the
  default and the rest of this document describes it.

Adding to a stable surface is always allowed and never requires a deprecation
cycle. See section 9.

## 2. The cycle

Every planned removal goes through three stages: Announce, Hold, Remove.

### 2.1 Announce

Deprecation lands in `vX.(Y+1).0`, i.e. the next minor release. Concretely:

- Rust API items get a `#[deprecated]` attribute (see section 5).
- CLI subcommands and flags get a `flag-aliases` entry that prints a warning
  to stderr the first time the deprecated flag is used in an invocation
  (see section 6).
- Lockfile schema fields get an explicit "deprecated as of v..." note in
  `docs/lockfile-format.md`, and the field is marked optional in the schema
  (see section 7).

The CHANGELOG entry for that release lists the deprecation under a
`# Deprecations` section at the top of the file (see section 8). The
deprecation notice always names the replacement, if there is one, and the
target removal version.

### 2.2 Hold

Subsequent minor releases keep the deprecated item working. The minimum
hold time is one minor release; for major API surface (anything in
`prelude`), the minimum is two minor releases.

So:

- A deprecated CLI flag announced in v1.3.0 may be removed in v2.0.0.
- A deprecated prelude item announced in v1.3.0 must continue to exist in
  v1.4.0 and v1.5.0 at minimum, and may be removed in v2.0.0.

The hold gives downstream users time to migrate without being forced into a
major upgrade just to keep their build green. During the hold the deprecated
item must continue to behave exactly as it did before deprecation; we do not
"soft-break" deprecated items.

### 2.3 Remove

Removal lands in the next major release (`v(X+1).0.0`) along with a
migration note in `docs/migration/v(X)-to-v(X+1).md`. The migration note
describes:

- What was removed.
- What replaces it (or, if nothing replaces it, why it was removed).
- A before/after example for the most common use case.

The CHANGELOG entry for the major release moves the item out of the
`# Deprecations` section and into the `# Breaking changes` section.

## 3. What can be deprecated

Anything covered by SemVer. Specifically:

- **Prelude items.** Anything re-exported by `guroku::prelude`: types,
  functions, traits, modules, macros, constants. If it can be named with a
  `use guroku::prelude::*;` it is in scope.
- **CLI subcommands and flags.** Every subcommand listed in
  `docs/cli-reference.md`, and every flag of those subcommands. This
  includes both long forms (`--registry`) and short forms (`-r`).
- **Lockfile schema fields.** Every field documented in
  `docs/lockfile-format.md`. Deprecation here means marking the field
  optional and ignored on read; see section 7.

If you find yourself wanting to deprecate something that does not fit one
of these three categories, it probably falls under section 4 instead.

## 4. What can't be "deprecated" because it isn't promised in the first place

The following surfaces are explicitly not covered by SemVer and can change
in any release without ceremony:

- **Internals.** Anything not re-exported by `guroku::prelude`. This
  includes the entire contents of `guroku::internal`, anything marked
  `#[doc(hidden)]`, and any module path not mentioned in the API overview.
- **Error message Display strings.** The exact wording of an error's
  `Display` output is not stable. The `ErrorKind` discriminant and the
  documented error code are stable; the prose around them is not.
- **Log line wording.** Log messages emitted via `tracing` may change at
  any time. Consumers should match on structured fields, not on the log
  message string.
- **Performance characteristics.** Wall-clock time and memory usage are
  not part of the contract. A patch release may make something faster or
  slower; a minor release may make a large change to either. We try not to
  regress, but we do not promise it.

Changing any of these is not a deprecation and does not need to go through
the cycle in section 2. It still belongs in the CHANGELOG so users can
correlate behaviour changes with releases, but it does not need a
deprecation announcement.

## 5. The `#[deprecated]` attribute

When applied to a Rust item, `#[deprecated]` causes the Rust compiler to
emit a warning at the call site in user code. This is the primary mechanism
for announcing Rust API deprecations.

Always include `since` and `note`:

```rust
#[deprecated(since = "1.3.0", note = "use FooBar instead")]
pub fn old_thing() {
    // ...
}
```

Guidelines:

- `since` is the version in which the deprecation was announced, not the
  version in which the item will be removed. Removal is communicated via
  the migration note and CHANGELOG.
- `note` always names the replacement if there is one. If there is no
  direct replacement, `note` should briefly describe what the user should
  do instead (for example, "use the builder API on `RegistryClient`").
- Pure-rename refactors should add a re-export of the old name with
  `#[deprecated]`, not remove the old name outright. For example:

  ```rust
  pub fn new_name() -> Foo { /* ... */ }

  #[deprecated(since = "1.3.0", note = "renamed to new_name")]
  pub use self::new_name as old_name;
  ```

  This keeps existing user code compiling (with a warning) for the duration
  of the hold.
- For deprecated trait methods, prefer adding a default implementation that
  forwards to the replacement, so downstream impls do not break.

## 6. CLI flag deprecation

Deprecated CLI flags must continue to work for the duration of the hold.
The first time a deprecated flag is used in a single invocation, guroku
prints a warning to stderr in the following format:

```
warning: --foo is deprecated since 1.3.0; use --bar instead
```

Specifics:

- The warning is printed once per invocation, not once per flag usage.
  If the user passes `--foo` three times, they see one warning.
- The warning goes to stderr, not stdout, regardless of any
  `--quiet`/`--verbose` setting. It is suppressed only by `--silent`.
- The warning text is not localised and is not part of the CLI's stable
  output (it falls under section 4: error/log wording is not promised).
  The fact that *some* warning is emitted is part of the contract; the
  exact wording is not.
- Subcommand deprecation works the same way: the warning is printed when
  the deprecated subcommand is invoked, before the subcommand executes.

If a flag is renamed, the old form should continue to behave identically
to the new form during the hold. We do not change the semantics of a
deprecated flag.

Example: deprecating `--registry-url` in favour of `--registry`:

```bash
guroku install --registry-url https://example.test
# warning: --registry-url is deprecated since 1.3.0; use --registry instead
# (install proceeds normally with the given registry)
```

## 7. Lockfile field deprecation

Schema additions are non-breaking; field removal is. Adding a new field is
always safe because older guroku versions are documented to ignore unknown
fields (see `docs/lockfile-format.md`). Removing a field is a breaking
change because older code that reads the field will start seeing it absent.

To deprecate a lockfile field:

1. Mark the field optional in the schema and document the deprecation in
   `docs/lockfile-format.md` with an explicit "deprecated as of vX.Y.0"
   note next to the field.
2. Continue to write the field out for the duration of the hold, with the
   same value the field had before deprecation. This keeps lockfiles
   readable by both new and old guroku versions during the transition.
3. Accept the field's absence on read. Code that consumes the field must
   tolerate it being missing, falling back to whatever the new behaviour
   is (typically a default value or a derived value).
4. In the major release that removes the field, stop writing it out and
   stop reading it. The migration note explains what users with old
   lockfiles should do.

A lockfile field deprecation does not require a `#[deprecated]` attribute
on any Rust type, because the lockfile schema is not directly exposed as
a stable Rust API. The schema is the contract; the Rust types that
implement it are internal.

## 8. Tracking deprecations

A `# Deprecations` section is maintained at the top of `CHANGELOG.md`,
above the per-release sections. It lists every currently active
deprecation along with:

- The item being deprecated (Rust path, CLI flag, or lockfile field).
- The version in which it was deprecated.
- The target removal version.
- The replacement, if any.

Example:

```markdown
# Deprecations

- `RegistryClient::with_default_registry` (deprecated 1.3.0, removed 2.0.0):
  use `RegistryClient::default_url` instead.
- `--registry-url` flag on `guroku install` (deprecated 1.3.0, removed
  2.0.0): use `--registry` instead.
- `lockfile.metadata.legacy_hash` field (deprecated 1.4.0, removed 2.0.0):
  no replacement; the integrity field carries the same information.
```

When the major release lands, items move from this section into the
`# Breaking changes` section under the new version.

## 9. What changes are NOT deprecations

The following changes are explicitly not deprecations and do not require
the cycle in section 2:

- **Bug fixes that change observable behaviour.** A bug is, by definition,
  the program not doing what it's documented to do. Fixing a bug is a
  patch release, not a deprecation. If the bug is severe and the fix
  itself is risky, we may include a flag to opt back into the buggy
  behaviour for one release; that flag is then itself deprecated in the
  next release.
- **Performance improvements that change wall-clock or memory.**
  Performance is not part of the SemVer contract (section 4), so changing
  it is not a deprecation. Note this in the CHANGELOG so users can
  correlate, but no announcement is needed.
- **Adding a new variant to a `#[non_exhaustive]` enum.** The whole point
  of `#[non_exhaustive]` is that consumers must already handle unknown
  variants. Adding a new one is not a breaking change and is therefore
  not a deprecation. We document new variants in the CHANGELOG under the
  release that adds them.
- **Adding a new CLI subcommand or flag.** Additions to the CLI are not
  breaking. They land in a minor release and are documented in
  `docs/cli-reference.md` as of that release.
- **Adding a new lockfile field.** Same reasoning as the CLI: additions
  are non-breaking because older guroku versions ignore unknown fields.

## 10. Worked example

Suppose we want to rename `RegistryClient::with_default_registry` to
`RegistryClient::default_url`. (This is purely illustrative; the real
method is staying put.)

The cycle plays out across three releases:

**v1.3.0 (Announce).** Add the new method. Mark the old method
deprecated, forwarding to the new one:

```rust
impl RegistryClient {
    pub fn default_url(url: Url) -> Self {
        // new canonical implementation
        Self { url, /* ... */ }
    }

    #[deprecated(since = "1.3.0", note = "use default_url")]
    pub fn with_default_registry(url: Url) -> Self {
        Self::default_url(url)
    }
}
```

The CHANGELOG entry for v1.3.0 includes:

```markdown
# Deprecations

- `RegistryClient::with_default_registry` (deprecated 1.3.0, removed 2.0.0):
  use `RegistryClient::default_url` instead.
```

User code that calls `with_default_registry` now compiles with a warning.

**v1.4.0 / v1.5.0 (Hold).** Both methods continue to exist. The old one
keeps emitting its deprecation warning. We do not change either's
behaviour.

Because `RegistryClient` is in `guroku::prelude`, the minimum hold is two
minor releases (section 2.2), so we cannot remove the method until v2.0.0.

**v2.0.0 (Remove).** The deprecated method is deleted:

```rust
impl RegistryClient {
    pub fn default_url(url: Url) -> Self {
        Self { url, /* ... */ }
    }
}
```

A migration note lands at `docs/migration/v1-to-v2.md`:

```markdown
## RegistryClient::with_default_registry removed

Replace:

```rust
let client = RegistryClient::with_default_registry(url);
```

with:

```rust
let client = RegistryClient::default_url(url);
```

The two methods were equivalent from v1.3.0 onward.
```

The CHANGELOG entry for v2.0.0 moves the item from `# Deprecations` to
`# Breaking changes`, and the deprecation entry is removed from the top
of the file.

That is the entire cycle, end to end. The same shape applies to CLI flag
removals and lockfile field removals; only the announcement mechanism
differs.
