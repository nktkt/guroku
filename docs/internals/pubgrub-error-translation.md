# Pubgrub Error Translation

How `pubgrub::error::PubGrubError` is mapped onto `guroku::error::GurokuError`
at the boundary between the resolver crate and the rest of guroku.

## 1. Why translate

- guroku has its own error type, exposed via `Result<T, GurokuError>`.
- Pubgrub returns `PubGrubError<P, V>` from `pubgrub::solver::resolve`.
- Embedders (CLI, library users, language bindings) should not have to know
  about pubgrub's internal error variants in order to react to a failed
  resolution.

Keeping the translation in one place means we can change pubgrub versions
without rippling type changes through the rest of the codebase.

## 2. The translation surface

- Function: `translate_pubgrub_error(err: PubGrubError<String, NpmVersion>) -> GurokuError`.
- Lives in `src/pubgrub_resolver.rs`.
- Called once: at the top of the entry point's solve step, immediately after
  `pubgrub::solver::resolve` returns an `Err`.

There is exactly one call site. New call sites should be reviewed; the goal
is for this function to be the only thing that pattern-matches on
`PubGrubError`.

## 3. Variant by variant

- `NoSolution(tree)` -> `GurokuError::ResolutionConflict` with the rendered
  tree placed in the `requested_by` field.
- `ErrorRetrievingDependencies { source, .. }` -> `GurokuError::Other` with
  the source's `Display` impl as the message.
- `DependencyOnTheEmptySet { .. }` -> `GurokuError::Other`. This means a
  package declared a dependency with an empty range; it is a metadata bug,
  not user error.
- `SelfDependency { .. }` -> `GurokuError::Other`. Also a metadata bug.
- `ErrorChoosingPackageVersion(source)` -> `GurokuError::Other`.
- `ErrorInShouldCancel(source)` -> `GurokuError::Other`.
- `Failure(msg)` -> `GurokuError::Other`. Pubgrub's generic failure surface.

## 4. Why so many `Other`

`ResolutionConflict` is the user-facing error. Everything else means
"we got an inconsistent state from the resolver." The user cannot fix those
by editing their `package.json`; they are guroku bugs, pubgrub bugs, or
registry bugs.

Lumping them into `Other` avoids `GurokuError` growing N variants for a
transient internal failure mode. We can split them later if telemetry shows
the lump is too coarse.

## 5. NoSolution: the interesting case

The tree returned by `NoSolution` is a `pubgrub::report::DerivationTree<P, V>`.
Translation steps:

1. Call `tree.collapse_no_versions()` to remove redundant
   "no version available" terminals. These pollute the report when a registry
   simply has no matching version, which is almost always already implied
   by another node in the tree.
2. Call `pubgrub::report::DefaultStringReporter::report(&tree)` to render to
   `String`.
3. The resulting multi-line text is placed into
   `GurokuError::ResolutionConflict.requested_by`.
4. Structured fields are filled with placeholders:
   - `name = "<resolver>"`
   - `chosen = "<unsolvable>"`
   - `requested = "<see report>"`

## 6. Why placeholders

Pubgrub conflicts can span multiple packages and incompatibilities. There
is no canonical "the package" that the conflict is about, so any single
value we put in `name` would be misleading.

The placeholders are obviously placeholders, so users (and tests) can
detect "this is a pubgrub-shaped error" by spotting them. The actual story
lives in `requested_by`.

## 7. Round-trip stability

`format!("{err}")` for a translated error includes both the placeholders
AND the `requested_by` content. The output is multi-line but plain text:
no ANSI escapes, no JSON. Tests in
`tests/pubgrub_conflict_report_format.rs` pin the contract so that
downstream tooling can grep for the placeholder strings without breaking
on a pubgrub upgrade.

## 8. What we don't translate

- The pubgrub `Cancel` signal. We don't use `ShouldCancel` today, so
  `ErrorInShouldCancel` is unreachable in practice; we still map it for
  type-completeness.
- The pubgrub `Failure` variant. This is pubgrub's own catch-all for
  "something went wrong"; it is exceedingly rare and treated as a generic
  `Other`.

## 9. Future work (v1.3+)

- Parse the derivation tree at translation time and populate the structured
  fields when the conflict centres on a single `(package, version)` pair.
- Add a `--explain` flag (CLI v1.3+) that re-runs the resolution with
  verbose tracing enabled and dumps the raw tree alongside the rendered
  report.
- Internationalised messages: today the report is English-only, since
  pubgrub's `DefaultStringReporter` is English-only.

## 10. Source pointers

- `src/pubgrub_resolver.rs:translate_pubgrub_error` — the implementation.
- `src/error.rs:GurokuError` — the target enum.
- `tests/pubgrub_conflict_report_format.rs` — round-trip tests pinning the
  rendered format.
