## Summary
(1-3 sentences.)

## Affected modules
- [ ] `src/pubgrub_resolver.rs`
- [ ] `src/version.rs` (Range / Version exposed types)
- [ ] `src/registry.rs` (PackageMetadata shape)
- [ ] `src/commands/install.rs` (resolver dispatch)
- [ ] `Cargo.toml` (pubgrub dep version)

## Behaviour preserved
- [ ] `GUROKU_RESOLVER=bfs guroku install` still uses the v1.1 BFS path.
- [ ] `Resolved.aliased_from` is None for non-aliased entries and Some(real_name) for aliases.
- [ ] file:/git: roots still fall back to BFS internally.
- [ ] Lockfile bytes for the example fixtures are unchanged.
- [ ] The v1.0 stability surface (`guroku::prelude`, lockfile schema, CLI flags) is unchanged.

## SemVer impact
- [ ] No change to items in `guroku::prelude`.
- [ ] No change to documented public function signatures in `pubgrub_resolver`/`resolver`/`version`.
- [ ] If bumping `pubgrub` minor: confirmed `NpmVersion`/`DependencyProvider` impls still satisfy the upstream traits.

## Pubgrub specifics
- [ ] If you changed `npm_range_to_pubgrub`: at least one fixture covering the new shape lands in tests.
- [ ] If you changed `prefetch_closure`: the new fetch surface is bounded (not O(every package on npm)).
- [ ] If you changed `translate_pubgrub_error`: the `requested_by` payload is still valid plain text.
- [ ] If you changed `plan_roots`: the file:/git: fallback decision is still correct.

## Pubgrub bump (only if upgrading the dep)
- [ ] All `DependencyProvider` trait method signatures still match.
- [ ] `Range`, `Version`, `Dependencies`, `PubGrubError` enums still match (or you've updated the imports + match arms).
- [ ] `DefaultStringReporter` API still matches (or you've updated the call site).

## Performance
- [ ] No new O(n²) loops in the prefetch closure or solve loop.
- [ ] If you changed candidate-set translation: the per-call cost is still O(|candidates|).

## Testing
- [ ] Tests added under `tests/pubgrub_*` or `tests/cli_help_v1_2_*`.
- [ ] `cargo test --all` passes locally.
- [ ] `cargo clippy --all-targets -- -D warnings` clean.
- [ ] `cargo fmt --check` clean.

## Documentation
- [ ] CHANGELOG.md `[Unreleased]` updated.
- [ ] `docs/internals/{pubgrub-integration,range-conversion}.md` updated when implementation changed.
- [ ] `docs/v1.2-release-notes.md` (or successor for the current minor) updated when user-visible behaviour changed.

## Risk and rollback
(1-3 sentences. Pubgrub is the new default; reverting needs the BFS path on `main`. The escape hatch is `GUROKU_RESOLVER=bfs`.)
