# A Tour of the guroku Codebase

Welcome. This document is a guided walk through the guroku source tree, written
for new contributors who have just cloned the repository and are wondering
where to look first. It is deliberately narrative: read it top-to-bottom with
the source open in another window, and by the end you should have a working
mental model of where every major piece lives and why.

guroku is an npm-style package manager written in Rust. This tour reflects
the v0.3 layout. Where v0.3 differs materially from earlier revisions, that is
called out inline.

---

## 1. Start here

The two files to open first, side by side, are:

- `src/lib.rs` — the entry point for the public API. Everything that is
  importable from outside the crate (or from the integration tests) is
  re-exported from here. If you cannot find a type by name in `lib.rs`, it is
  not part of the public surface and you should not depend on it from a test.
- `src/main.rs` — the binary entry point. It is intentionally tiny: parse
  CLI args, set up logging, dispatch to a command handler, translate errors
  into a process exit code.

The split is the conventional Rust pattern: `lib.rs` is the library, `main.rs`
is a thin shell over the library. Most of the work happens in the library;
the binary exists so the library can be invoked from a terminal.

If you are about to add a feature, start by asking: does this belong in the
library, or only in the binary? Answer is almost always "library."

---

## 2. The CLI

```
src/cli.rs   <-- clap definitions
src/main.rs  <-- dispatch
```

`src/cli.rs` is pure declarative: a top-level `Cli` struct deriving
`clap::Parser`, plus a `Command` enum deriving `clap::Subcommand`. Each
variant carries the args for one subcommand and nothing else. There is no
business logic here.

`src/main.rs` matches on the parsed `Command` and calls into
`src/commands/*`. Adding a new subcommand therefore touches both files:

1. Add a variant to `Command` in `src/cli.rs`, with its arg fields and clap
   attributes.
2. Add a match arm in `src/main.rs` that destructures the variant and calls
   the corresponding `commands::<name>::run(...)`.

If you only edit one of the two, you will either have a CLI flag with no
implementation or an implementation that nothing can reach.

---

## 3. The manifest

`src/manifest.rs` parses and writes `package.json`. The core type is
`Manifest`, a serde-deserialised struct.

The conventions here are strict and worth internalising:

- Every field in `Manifest` (and its nested types) uses `#[serde(rename =
  "...")]` to map between Rust's `snake_case` and JSON's `camelCase` /
  `kebab-case`. Do not skip this attribute even when the names happen to
  match.
- Optional fields are `Option<T>`, not `T` with a `Default`. This matters
  because we round-trip manifests on disk and must not synthesise fields the
  user did not write.
- Unknown fields are preserved via a catch-all map so that running
  `guroku add` against a manifest with custom keys does not silently strip
  them.

Adding a new manifest field therefore means:

1. Add the struct field with the appropriate `#[serde(rename = ...)]` and
   `Option<T>` wrapper.
2. Re-export or expose it on `Manifest` if external callers need to read it.
3. If the field affects resolution or installation, thread it through the
   resolver or commands as appropriate (a separate concern from the parse
   layer).

---

## 4. The registry client

`src/registry.rs` wraps `reqwest` and is the only place in the crate that
talks HTTP to a registry. There are three things you will touch most often
when working in this module:

- `RegistryClient::fetch_metadata` — issues the HTTP request, threads the
  ETag through `http_cache`, and parses the response body into
  `PackageMetadata`. This is the public entry point for "get me the record
  for package X."
- `PackageMetadata::resolve` — given a parsed semver range, returns the
  best-matching `VersionInfo`. Keep this function pure: no I/O, no logging
  beyond `debug!`. It is exercised heavily in unit tests.
- `VersionInfo` and `Dist` — the registry-record shape. `VersionInfo` is
  the per-version object, `Dist` is the nested tarball descriptor with the
  URL and integrity string. If you find yourself wanting to plumb a new
  registry-side field through the resolver, this is where it enters the
  type system.

Do not introduce a second HTTP client elsewhere. Anything that needs network
access goes through `RegistryClient`.

---

## 5. The HTTP cache

`src/http_cache.rs` is the small, focused module that backs the conditional
GETs in the registry client. For each package name `<name>` it owns two
files inside a cache directory:

```
<dir>/<name>.json   -- the cached response body
<dir>/<name>.etag   -- the ETag string from the previous response
```

The public functions come in two flavours:

- The default forms (`read`, `write`, etc.) use the user's real cache
  directory.
- The `_in` variants (`read_in`, `write_in`, ...) take the directory as an
  explicit argument. These exist so that integration tests can hand in a
  `tempfile::TempDir` and exercise the real code paths without polluting
  `~/.guroku/`.

When you add a new cache operation, add both the user-facing form and the
`_in` variant. The default form should be a one-line wrapper that supplies
the real directory.

---

## 6. Versions and ranges

`src/version.rs` is a thin module on top of the `node-semver` crate. It
re-exports `Version` and `Range` and adds three convenience functions:

- `parse_range` — parses a string into `node_semver::Range`, with our
  preferred error wrapping.
- `parse_version` — same for `Version`.
- `max_satisfying` — given a range and a slice of versions, returns the
  maximum that satisfies it (or `None`).

The rule for the rest of the crate is: **do not import `node_semver`
directly.** Always go through `crate::version`. This gives us a single
chokepoint to swap implementations, normalise error types, and add
guroku-specific behaviour (such as range coercion for tag specifiers)
without rewriting every call site.

If you find yourself reaching for `node_semver` in another module, add the
helper to `version.rs` instead.

---

## 7. The resolver

`src/resolver.rs` turns a manifest plus a registry client into a
`Resolution`: the full graph of packages and versions that need to be on
disk for `node_modules` to be coherent.

Key design points:

- The walk is a breadth-first traversal of the dependency graph.
- It is **sticky-first**: once a version of a package has been chosen, the
  same version is reused for any later occurrence whose range allows it.
  This avoids gratuitous duplication and matches the lockfile's intent.
- v0.3 added parallel root prefetch: the direct dependencies are fetched
  concurrently before the main BFS begins. This is purely a latency
  optimisation; the resolution result is identical to the serial version.

The output, `Resolution`, is the only thing the linker cares about. Treat
it as the contract between resolution and installation: if you change its
shape, you are changing the contract and need to update both sides.

---

## 8. The CAS

The content-addressed store lives in two files:

- `src/store.rs` — the on-disk layout, atomic writes, and lookup by
  integrity hash.
- `src/cache.rs` — higher-level helpers around the store, including the
  metadata cache that lives next to the CAS.

CAS entries are laid out as:

```
~/.guroku/cas/<sha[0:2]>/<sha[2:]>/
```

The two-character prefix avoids putting hundreds of thousands of entries
in a single directory. Each entry is a directory containing the unpacked
tarball contents; this is what the linker hardlinks from.

Atomicity is via tmp-then-rename: writes happen into
`~/.guroku/cas/tmp/<random>/`, and only after the bytes are verified and
the directory is fully populated do we `rename(2)` it into its final
location. A crash mid-write therefore leaves an orphaned tmp directory but
never a half-populated CAS entry.

---

## 9. The integrity verifier

`src/integrity.rs` is small and load-bearing. We support **SHA-512 only**;
weaker algorithms (SHA-1 in particular) are rejected at parse time so a
malicious registry response cannot downgrade us.

The flow is:

1. Stream the tarball bytes from `reqwest`.
2. Hash them as they arrive.
3. Compare the final digest against the integrity string from the
   registry's `Dist`.
4. Only on match do the bytes proceed into the CAS.

The verification happens **before** the bytes touch the CAS, not after.
This is deliberate: the CAS is the trust boundary, and nothing
unverified gets to live there even briefly.

---

## 10. The tarball extractor

`src/tarball.rs` unpacks the verified bytes into a CAS staging directory.
Two behaviours to know about:

- The `package/` prefix that npm tarballs ship with is stripped. CAS
  entries should look like the package's root, not like
  `package/lib/index.js`.
- Path traversal is rejected. Any entry whose normalised path escapes the
  destination root (`..` segments, absolute paths, symlinks pointing
  outside) causes extraction to abort with an error. Do not relax this
  check.

---

## 11. The linker

`src/linker.rs` writes `node_modules`. It exposes three public functions:

- `link_flat` — the v0.1 flat layout. Retained for tests and for the
  `--flat` debug flag, but no longer the default.
- `link_hardlink_tree` — recursive hardlink of one CAS entry into a
  destination directory. The primitive used by both layouts.
- `populate_node_modules` — the v0.3 strict-layout writer. This is the
  default path for `install`/`add`/`remove`. It produces a layout where
  each package's transitive dependencies live under that package's own
  `node_modules`, matching Node.js resolution semantics for the strict
  case.

If you are adding a new layout strategy, add a new public function rather
than re-purposing an existing one. The commands pick a layout explicitly.

---

## 12. The lockfile

`src/lockfile.rs` is JSON read/write for `guroku.lock`. The format is
deliberately simple: a top-level version field, a packages map keyed by
`name@version`, and per-entry resolved URL and integrity.

The single rule that matters: **bump `LOCKFILE_VERSION` for any
incompatible schema change.** "Incompatible" means anything that an older
guroku binary would mis-read. Adding an optional field that older readers
can ignore is fine; renaming a key, changing a type, or removing a field is
not, and needs the version bump plus a migration path.

---

## 13. The commands

`src/commands/` is where the user-visible verbs live. Each verb is its
own file and is intentionally a thin glue layer on top of the resolver,
the store, and the linker:

- `commands/install.rs` — read manifest, resolve, fetch into CAS, link.
- `commands/add.rs` — same, plus mutate the manifest with the new
  dependency entry.
- `commands/remove.rs` — same, plus remove the entry from the manifest.

Shared helpers live in `commands/mod.rs`. The three you will reuse most:

- `fetch_into_cas` — given a `VersionInfo`, ensures the tarball is
  present in the CAS (downloading and verifying if not).
- `into_linked_packages` — converts a `Resolution` into the input shape
  that the linker expects.
- `parse_spec` — parses a CLI argument like `lodash@^4` into a
  `(name, range)` pair.

A new command should follow the same shape: a `run` function in
`commands/<name>.rs`, helpers lifted into `commands/mod.rs` as soon as a
second command needs them.

---

## 14. Errors

`src/error.rs` holds the single `Error` enum with all error variants in
one place. Variants use `thiserror` for the `Display` and `From`
implementations.

To add a new error kind:

1. Add a variant to the enum.
2. Give it a `#[error("...")]` template that reads well in a terminal.
3. If it wraps another error type, derive `#[from]`.
4. Surface it from the module that originally produced the failure, so
   the call sites do not need to manually re-wrap.

Resist the temptation to add per-module error enums. The flat enum is a
deliberate choice: it makes the binary's top-level error handler exhaustive
and keeps the matrix of "which command can produce which error" visible in
one file.

---

## 15. Logging

`src/logging.rs` initialises `tracing-subscriber`. The filter is read
from `GUROKU_LOG`, falling back to `RUST_LOG` if the former is unset.

Throughout the rest of the crate, use `tracing` macros directly:

```rust
tracing::info!(package = %name, version = %v, "fetched");
tracing::debug!(?range, "resolving");
tracing::warn!(?path, "skipped non-package entry");
```

Conventions:

- `info!` for things a user running with default verbosity should see
  (resolution complete, install finished, etc.).
- `debug!` for the per-package and per-step detail.
- `warn!` for recoverable surprises.
- Errors that are returned from a function should not also be logged at
  the throw site. Log them once at the top of the call stack.

---

## 16. Tests

`tests/` holds the integration tests. Every `*.rs` file in that directory
is compiled as its own test binary by Cargo, which is the convention we
lean into:

- One tested behaviour per file. If a test file is growing past a few
  hundred lines, that is a signal to split it.
- Anything that touches disk uses `tempfile::TempDir`. No test writes to
  `~/.guroku/`, ever. The `_in` variants in `http_cache` and friends exist
  precisely so this is easy.
- Network is mocked with `wiremock`. A test that requires the real
  registry is not an integration test, it is a manual probe and does not
  belong in `tests/`.
- Test files share helpers via `tests/common/mod.rs`. Cargo only treats
  files at the top of `tests/` as test binaries, so `common` is safe from
  being compiled as one.

---

## Where to go next

This tour is intentionally a map, not a manual. For more depth:

- `ARCHITECTURE.md` at the repo root has the broader overview: the
  high-level design decisions, the data flow diagram, and the rationale for
  the strict layout.
- The `docs/internals/*` pages drill into individual subsystems (the
  resolver, the CAS, the linker) at a level of detail that would have
  buried this tour.
- `CONTRIBUTING.md` covers the mechanics: how to set up the dev
  environment, what we expect from a pull request, and how the review
  process works.

If you find a part of the codebase that this tour misrepresents, that is a
bug in the tour. Edits welcome.
