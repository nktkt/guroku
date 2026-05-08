# Pubgrub Conflict Explainer

How resolution failures are surfaced to users as `GurokuError::ResolutionConflict`,
covering both the legacy v1.1 BFS resolver and the v1.2 pubgrub resolver.

## 1. Two flavours of conflict

Guroku has two resolvers, each with its own conflict representation:

- **v1.1 BFS path**: a simple `(name, range, path)` conflict. `requested_by` is the
  dep-graph path expressed as a string like `"a > b > c"`. It tells you exactly
  which chain of dependencies led to the offending requirement.
- **v1.2 pubgrub path**: a cascading conflict that pubgrub describes as a
  derivation tree. `requested_by` is the human-readable derivation report, which
  may span many lines and reference multiple incompatibilities.

Both flavours funnel into the same error variant. The shape of `requested_by`
is the only thing that differs between them.

## 2. The GurokuError::ResolutionConflict shape (unchanged from v1.1)

The variant is intentionally unchanged so that downstream consumers (the CLI
formatter, JSON error output, and existing test suites) continue to work:

- `name: String` - the package the conflict centres on.
- `chosen: String` - the version we'd already chosen, or `<unsolvable>` for
  pubgrub conflicts where no single version was ever picked.
- `requested: String` - the conflicting range, or `<see report>` for pubgrub
  conflicts where the conflict is between multiple constraints.
- `requested_by: String` - the dep-graph path (BFS) or the rendered derivation
  report (pubgrub).

## 3. For pubgrub conflicts

When the pubgrub resolver returns `PubGrubError::NoSolution(tree)`,
`translate_pubgrub_error` does the following:

- Collapses the derivation tree's "no versions" terminals via
  `tree.collapse_no_versions()` so the report focuses on the meaningful
  derivation steps. The raw tree contains a lot of "package@version has no
  versions matching X" leaves that obscure the real story.
- Renders the collapsed tree via `DefaultStringReporter::report(&tree)`. The
  output is multi-line plain text designed for human reading.
- Stuffs the rendered string verbatim into `requested_by`.

The structured `name`, `chosen`, and `requested` fields are filled with
placeholder strings (`<resolver>`, `<unsolvable>`, `<see report>`) because real
pubgrub conflicts can span multiple packages.

## 4. Why placeholders, not "best-effort structured fields"

It's tempting to walk the derivation tree and pick "the package" at the centre
of the conflict, but:

- The derivation tree is shaped like nested incompatibilities. There is no
  canonical "the package" to extract. A conflict between `lib-a` and `lib-b`
  via `core` could plausibly be attributed to any of the three.
- Picking one would be misleading. Users would naturally trust the structured
  fields and miss the broader context in the report.
- The placeholder strings (`<resolver>`, `<unsolvable>`, `<see report>`) are
  scannable and self-documenting. A user who sees `chosen: <unsolvable>` knows
  immediately to read `requested_by`.
- The issues template `pubgrub_resolution_failure.yml` invites users to paste
  the `requested_by` content directly, so the report ends up where it's useful.

## 5. Example conflict report

A typical rendered report looks like this:

```
Because lib-a@1.2.3 depends on core@>=2.5
and lib-b@1.0.0 depends on core@<2.5,
lib-a@1.2.3 and lib-b@1.0.0 are incompatible.
And because root depends on lib-a@^1
and root depends on lib-b@^1,
no version solves.
```

The `DefaultStringReporter` handles all the formatting (line breaks, "Because"
phrasing, conjunctions). We don't post-process the output.

## 6. For v1.1 BFS conflicts (unchanged)

The v1.1 resolver tracks a `Vec<String>` for each dep's path back to the root
and formats it with `format_path`, producing strings like `"a > b > c"`. When
a conflict is detected during BFS expansion, the resolver emits
`ResolutionConflict { name, chosen, requested, requested_by: format_path(&path) }`
with concrete values for every field.

The v1.1 contract is preserved exactly so existing test suites pass without
modification, and users who haven't opted into pubgrub still see the friendly
single-package conflict format they're used to.

## 7. Reading a pubgrub conflict report

Some practical advice for users:

- Read bottom-up. The last lines of the report describe the highest-level
  incompatibility (typically between roots). Earlier lines explain how each
  terminal incompatibility was derived.
- Look for the package name and version that appears in the leaf
  incompatibilities. The conflict is rooted there.
- Cross-reference with the project's `package.json` to find the constraint
  that's too restrictive. The report names the ranges; your manifest names
  the dependents.
- Add an `overrides` entry pinning the conflicting transitive dep, **or**
  loosen one of the offending constraints in your direct deps.

## 8. What pubgrub doesn't do

The derivation tree is a precise description of *why* resolution failed, but
it deliberately stops short of:

- Suggesting a fix. The report describes what's wrong, not how to fix it.
- Telling you which constraint to relax. Multiple roots may all be reasonable;
  picking one to blame is a policy decision pubgrub doesn't make.
- Showing a happy-path "if you change X to Y this resolves." That's on our
  v1.x backlog, not something pubgrub provides out of the box.

## 9. Future

- v1.3 may parse the derivation tree and populate the structured `name`,
  `chosen`, and `requested` fields when the conflict centres on a single
  package. The placeholders are a stepping stone, not a permanent answer.
- Later v1.x may add an "explain why pubgrub picked X@Y" mode via resolver
  instrumentation, surfacing the decision trace alongside conflict reports.

## 10. Source

- `src/pubgrub_resolver.rs:translate_pubgrub_error`
- `src/error.rs:GurokuError::ResolutionConflict`
- `tests/pubgrub_conflict_report_format.rs`
