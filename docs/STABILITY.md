# Stability

This document is the stability commitment for guroku v1.0. It defines, in
plain language, what is covered by Semantic Versioning (SemVer) and what is
not. If you depend on guroku, either as a CLI in scripts and CI or as a Rust
library in another tool, this is the contract you can rely on.

The short version: the lockfile, the CLI surface, and the `guroku::prelude`
are stable. Internals, log wording, and on-disk fluff are not. The rest of
this document spells out the details.


## 1. What v1.0 promises

The v1.0 release establishes three stability boundaries. Within the v1.x
series, guroku promises the following:

- The lockfile schema (`lockfileVersion: 1`) is SemVer-stable: a v1.x guroku
  will load any v1 lockfile written by any other v1.x guroku.
- The Rust public API exposed via `guroku::prelude` is SemVer-stable.
- The CLI surface (every subcommand, flag, and exit code documented in
  `docs/cli-reference.md` as of v1.0) is SemVer-stable.

Everything outside these three boundaries is, by default, not part of the
stability contract. The remainder of this document elaborates on each, and
on the deliberate carve-outs and processes around them.


## 2. What v1.0 does NOT promise

It is just as important to be explicit about what is _not_ covered. We have
seen too many projects assert "1.0 is stable" and then quietly let the
definition of "stable" expand until every user-visible string is a
breaking-change minefield. We refuse that trap.

The following are explicitly excluded from the stability contract:

- **Internals.** Anything not re-exported from `guroku::prelude` and any
  item marked `#[doc(hidden)]` is internal. It may change in any minor
  release, including breaking changes to its signature, its behaviour, or
  its very existence. If you find yourself reaching past `prelude` into a
  submodule path, you have left the stable surface and you are on your own.
- **Behaviour beyond what's documented.** The exact wording of log lines,
  the order of independent operations, the precise byte sequences printed
  to stderr during a successful install: none of these are guaranteed. If
  it is not in `docs/cli-reference.md`, `docs/lockfile-format.md`, or the
  rustdoc for `guroku::prelude`, it is not promised.
- **On-disk layout under `~/.guroku/`** outside of the lockfile and the
  `.npmrc`-driven config. The cache structure, the temp file naming, the
  layout of the content-addressed store: all of this is an implementation
  detail. Users should never script against these paths. The lockfile
  (typically `guroku.lock` in the project root) and the `.npmrc` config
  (which guroku reads for compatibility) are the only on-disk surfaces
  that participate in the contract.
- **Performance.** A minor release can be slower (or faster) without
  bumping the major version. We will not regress dramatically without a
  reason, and we track benchmarks, but a 5% slowdown in a particular
  workload does not constitute a SemVer break. Conversely, a 10x speedup
  is not a major-version event either.
- **Error message text.** The error _variants_ - the enum members of
  `GurokuError` - are stable; new variants may be added (the enum is
  `#[non_exhaustive]`), but existing ones will not be removed or renamed
  without a major bump. The `Display` strings on those variants, however,
  are best-effort. We may rephrase them, add context, or translate them.
  If you are matching on the textual output of an error, you have built
  on sand.

If you are unsure whether a particular behaviour is in or out, check the
relevant documented surface. If it is not documented as stable, treat it
as unstable.


## 3. The lockfile commitment in detail

The lockfile is the single most important on-disk artefact guroku
produces. It is what makes builds reproducible, what CI systems compare
against, and what your colleagues' machines must agree with. Stability
here is non-negotiable.

The commitment, in detail:

- `lockfileVersion: 1` is permanent within the v1.x series. There will
  never be a v1.x release that writes `lockfileVersion: 2`, and there will
  never be a v1.x release that refuses to read a `lockfileVersion: 1`
  written by another v1.x release. If you upgrade guroku from v1.0.0 to
  v1.7.3, your existing lockfile continues to work without rewriting.
- New OPTIONAL fields may be added in minor releases. Older guroku must
  tolerate unknown fields, and does so structurally: the lockfile
  deserializer uses `#[serde(default)]` for new optional fields, and
  pointedly does NOT use `#[serde(deny_unknown_fields)]`. An older v1.x
  guroku reading a lockfile written by a newer v1.x guroku will simply
  ignore fields it does not recognize.
- Removing a field, renaming a field, or making a previously optional
  field required is a breaking change. It requires either a major version
  bump (v2.0) or a `lockfileVersion: 2`, and in practice the two go
  together.
- Bumping `lockfileVersion` is itself a major bump. There is no scenario
  in which `lockfileVersion: 2` ships in a v1.x release. If you see such
  a thing in the wild, it is a bug, and we want to hear about it.

Note that "tolerate" does not mean "round-trip." An older v1.x guroku that
reads a newer lockfile, modifies it, and writes it back may drop fields it
did not understand. This is by design: we will not invent semantics for
fields we do not know. If round-tripping unknown fields matters to your
workflow, ensure all collaborators run a guroku at least as new as the
newest field in use.


## 4. The Rust API commitment

guroku is primarily a CLI, but it also ships as a library crate so that
build tools, IDE plugins, and meta-package-managers can integrate with it
directly. The library surface is not a kitchen sink: it is intentionally
narrow, and it lives entirely under `guroku::prelude`.

The commitment:

- Every item re-exported by `guroku::prelude` is stable. Names and
  signatures will not change in incompatible ways across minor releases.
  If a function has type `fn install(&self, manifest: &Manifest) -> Result<Lockfile>`
  in v1.0.0, it has that same type, modulo additive non-breaking changes,
  in every v1.x release.
- `GurokuError` is `#[non_exhaustive]`. New variants may be added in
  minor releases without breaking external matchers, because external
  matchers are required by the compiler to include a wildcard arm. If you
  see your `match` on `GurokuError` failing to compile after a minor
  upgrade and you do not have a `_` arm, your code was always wrong.
- The crate root re-exports `Result` and `GurokuError`. Those re-exports
  are stable. `guroku::Result<T>` is and will remain
  `core::result::Result<T, guroku::GurokuError>`.

What is _not_ stable on the Rust side:

- Anything reachable through a non-prelude path, even if it is `pub`.
  The `pub` visibility is necessary for `prelude` to re-export the item;
  it does not constitute an additional stability promise on the original
  path.
- Anything marked `#[doc(hidden)]`. These items exist for macro hygiene,
  for internal cross-crate use, or for tests. They will move and change
  freely.
- Anything in a submodule whose docs say "internal" or which is named
  `internal`, `private`, `__macros`, or similar. These names are
  conventionally unstable.

If you want a stable API surface added to `prelude`, open an issue. We
are conservative about additions: once it is in, we cannot easily take
it out.


## 5. The CLI commitment

The CLI is what most users interact with. Scripts, CI configurations,
Makefiles, container builds: all of these depend on the CLI behaving the
same way today as it did yesterday.

The commitment:

- Every documented subcommand, flag, and behaviour in
  `docs/cli-reference.md` as of v1.0 is stable. `guroku install`,
  `guroku add`, `guroku remove`, `guroku run`, `guroku audit`,
  `guroku why`, and the rest: their argument parsing, their effects, and
  their exit codes are part of the contract.
- New flags may be added in minor releases. A minor release that adds
  `--frozen-lockfile-strict` is still SemVer-compatible, even if older
  guroku rejects that flag. Scripts that do not pass the new flag are
  unaffected.
- New subcommands may be added in minor releases. They will not collide
  with existing subcommand names.
- Removing a flag is a major bump. Renaming a subcommand is a major bump.
  Changing what an exit code means is a major bump. Changing the default
  behaviour of an existing flag is a major bump.

Exit codes deserve a special note. The full table is in
`docs/cli-reference.md`, but the rule is: a given exit code, once
documented, has a fixed meaning. We will not, for instance, silently
broaden exit code 3 from "lockfile out of date" to "lockfile out of date
or registry unreachable." If we need a new exit code, we add a new one
in a minor release; we do not redefine an old one.

Output format is a more delicate matter. Human-readable output is
considered behaviour-beyond-what's-documented for the purposes of
stability: do not grep its exact wording. Machine-readable output (JSON
formats produced under `--json` flags) IS stable. Its schema is
versioned in the same spirit as the lockfile.


## 6. Deprecation process

We do not remove things on a whim, and we do not remove them silently.
The full deprecation policy is in `docs/deprecation-policy.md`. Briefly:

- Deprecation is announced in a minor release, e.g. v1.x.0.
- The deprecated item continues to function, and emits a warning, in
  every subsequent release for at least one minor cycle.
- Removal happens at the next major bump (v2.0) at the earliest.

Concretely: if we decide in v1.4.0 that a flag should go away, the flag
keeps working through v1.4, v1.5, and so on until v2.0, with a warning
printed each time it is used. We will not remove it in v1.5 or v1.6 just
because we feel like it.

Deprecation warnings are themselves not part of the strict-stability
contract on log wording, but their _existence_ is: a deprecated thing
WILL warn, even if the exact phrasing of the warning evolves.


## 7. MSRV

The Minimum Supported Rust Version (MSRV) is documented in
`docs/MSRV.md`. As of v1.0, the MSRV is Rust 1.75.

MSRV bumps are handled with a 6-month deprecation window: when we decide
to require a newer Rust, we announce it, and we do not actually bump for
six months. Within those six months, releases continue to compile on the
old MSRV.

An MSRV bump is not, on its own, a SemVer-major event for guroku. It is
a SemVer-minor event with a known, advertised compatibility implication.
Downstream tools that care about MSRV should track `docs/MSRV.md` and the
release notes.


## 8. Forward-compatibility tests

The promises above are not just words. They are enforced by tests that
will fail loudly if anyone, including us, tries to break them.

The relevant tests live in the test suite and exist for exactly this
reason:

- `tests/lockfile_unknown_field.rs` constructs a synthetic lockfile that
  contains a field guroku does not know about, and asserts that guroku
  loads it without error. If anyone adds `#[serde(deny_unknown_fields)]`
  to the lockfile types, this test catches it before release.
- `tests/manifest_unknown_field.rs` does the same for manifests
  (`package.json` files), guarding against accidental strictness in the
  manifest parser.
- `tests/api_stability_*.rs` is a family of tests that import items from
  `guroku::prelude` and assert their signatures by use. They will not
  necessarily fail to compile if a signature changes (Rust does not have
  built-in API surface diffing), but they exercise enough of the surface
  that obvious breakages are caught.

These tests are part of the standard CI run. A pull request that breaks
one of them is, by default, rejected. Breaking one of them deliberately
requires explicit sign-off and either a major-version branch or, in rare
cases, a documented carve-out with a clear rationale.

If you find a way to break a stability promise that these tests do not
catch, that is a bug in the tests, and we want a report. See section 10.


## 9. What "unstable" looks like in our docs

Stability is not the default in documentation; it is opt-in. We mark
unstable surfaces explicitly so that users can tell at a glance.

Unstable, in our docs and our code, means:

- Anything labelled "internal" in prose. If a doc paragraph says
  "Internally, guroku does X," that is a description of current
  behaviour, not a promise about future behaviour.
- Anything under `docs/internals/`. This subtree is for contributors and
  curious users; it documents implementation details that can and do
  change between releases. Do not script against it.
- Anything marked TODO. A `TODO` in the docs or in the code means the
  thing is not finished, the design is not settled, or the author wants
  to revisit it. Treat it as a strong signal that the surrounding
  behaviour is in flux.
- Anything in a release note explicitly marked "experimental" or
  "preview." We use these labels for features that ship behind a flag
  and whose final shape is still being negotiated with users. They live
  outside SemVer until they are promoted.

If you see something that is not in `prelude`, not in
`docs/cli-reference.md`, not in `docs/lockfile-format.md`, and not
explicitly labelled "stable," default to assuming it is unstable.


## 10. Reporting stability bugs

If you believe a stability promise has been broken, we want to know. The
process:

1. Open an issue in the guroku repository.
2. Apply the "stability" label.
3. Cite the exact promise in this document that you believe was
   violated. Quote the bullet, name the section. We have intentionally
   structured this doc as a list of citable claims.
4. Provide a reproduction: the version of guroku you upgraded from, the
   version you upgraded to, the input that triggers the issue, and the
   observed and expected behaviour.

Stability bugs are treated as high-priority. We may respond by:

- Reverting the offending change in a patch release, if the breakage
  was unintentional.
- Issuing a clarification or correction to this document, if the promise
  was ambiguous.
- Acknowledging the breakage and scheduling it for the next major, if it
  was deliberate but missed our own deprecation process. (This last case
  is rare, and we will explain ourselves.)

We do not consider a stability complaint resolved until either the
behaviour is restored or the promise is publicly amended. Silent
narrowing of stability claims is, itself, a kind of stability bug.


## Closing note

This document will evolve. It will not, however, narrow: the promises in
section 1, the carve-outs in section 2, and the detailed rules in
sections 3 through 5 are themselves stable. We may add new promises, we
may clarify existing ones, but we will not retroactively shrink them
within the v1.x series.

If you read this document carefully and you can build your tooling
within the documented surface, you can upgrade guroku within v1.x with
confidence. That is the entire point of v1.0.
