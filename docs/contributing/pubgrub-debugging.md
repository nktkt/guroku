# Debugging the pubgrub resolver

This page is for contributors patching the v1.2 resolver. Users hitting pubgrub conflicts in their projects should read `docs/pubgrub-resolver.md` instead.

## Reproducing a user's failure
1. Get the user's `package.json` and (if they have one) `guroku.lock`.
2. `cd` into a fresh dir and copy the manifest.
3. `GUROKU_LOG=debug guroku install` — capture the failing output.
4. `GUROKU_LOG=debug GUROKU_RESOLVER=bfs guroku install` — does the BFS path also fail? If yes, the bug isn't pubgrub-specific.
5. If BFS succeeds and pubgrub fails, the bug is in our pubgrub integration (range translation, prefetch closure, error translation). Move to the next sections.

## Common pubgrub failure modes
- **Pubgrub picks wrong version**: usually a range translation bug. Add a candidate that should match but doesn't appear in the chosen set.
- **Pubgrub fails to find a solution that exists**: usually the candidate-set translation is missing a version. Check that `prefetch_closure` actually fetched the package.
- **Pubgrub takes too long**: rare, but if a project has thousands of transitives, the prefetch closure can be slow. Profile with `cargo flamegraph`.
- **Pubgrub's error report mentions the wrong packages**: the error translation surface in `translate_pubgrub_error`. Add a structured-field extractor in v1.x.

## Tools
- `GUROKU_LOG=debug` — verbose logging from `tracing`.
- `GUROKU_LOG=trace` — even more, including HTTP fetches.
- `GUROKU_RESOLVER=bfs` — force the v1.1 path for comparison.
- `cargo test --test pubgrub_resolver_simple -- --nocapture` — run the public-surface checks.
- `cargo test --test pubgrub_npm_version -- --nocapture` — run the Version trait checks.

## Adding a regression test
1. Reduce the failing project to a minimal package.json + minimal registry metadata.
2. Add a fixture under `tests/fixtures/` if needed.
3. Write a test under `tests/pubgrub_*.rs`. For tests that need the network, mark them `#[ignore]` and document why.
4. For tests that just need a Manifest + classifier: keep them in-process. Use `Manifest::default()` + manual field population.

## Reading pubgrub's source
- `pubgrub::solver::resolve` in pubgrub 0.2.1 is the entry point. Trace through `next_decision` and `propagation`.
- The DependencyProvider trait is in `pubgrub::solver`. Our impl is in `src/pubgrub_resolver.rs`.
- Range arithmetic lives in `pubgrub::range`. Range::union, Range::intersection are the most-touched.
- Reporting is in `pubgrub::report`. DefaultStringReporter is the simplest.

## When to bump pubgrub
- 0.3 (or later) is released and stable.
- Our test suite passes against the new version.
- Conflict reports are at least as good as 0.2's.
- Migration is documented in `docs/migration/`.

## What to avoid
- Don't add a new `DepSpec` variant without a `_` arm in the BFS resolver. The non_exhaustive attribute is the right tool here.
- Don't change `NpmVersion::bump()` semantics without bumping at least the patch version of guroku itself. Lockfile determinism depends on bump being stable.
- Don't fetch metadata inside the DependencyProvider. The async/sync bridge means you'd need block_on, which complects the runtime.
- Don't extend `GurokuError` for pubgrub-specific reasons. The existing variants (ResolutionConflict, Other) cover the surface.

## Where to ask
- GitHub Discussions for questions.
- Issues for bug reports.
- The `pubgrub_resolver.rs` source has comments explaining each non-obvious decision; read them first.

## Related docs
- `docs/internals/pubgrub-integration.md` — the deep-dive.
- `docs/internals/range-conversion.md` — npm Range -> pubgrub Range.
- `docs/internals/two-phase-resolver.md` — async/sync bridge.
- `docs/internals/pubgrub-version-trait.md` — Version trait impl.
- `docs/internals/pubgrub-conflict-explainer.md` — error translation.
- `docs/internals/v1.2-architecture-decisions.md` — ADRs.
