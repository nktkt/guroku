## Summary
(1-3 sentences.)

## Affected modules
- [ ] `src/resolver.rs`
- [ ] `src/specs.rs`
- [ ] `src/overrides.rs`
- [ ] `src/manifest.rs` (resolution-relevant fields)
- [ ] `src/registry.rs` (resolve / metadata)
- [ ] `src/version.rs` (range / version)

## Behaviour preserved
- [ ] The lockfile produced for an existing project's `package.json` is bit-for-bit identical (or the diff is documented in the PR description).
- [ ] `guroku install --frozen-lockfile` still installs from the existing lockfile unchanged.
- [ ] `ResolutionConflict.requested_by` still uses `>` as the path separator.
- [ ] The override precedence ladder (path-overrides → flat-overrides → path-resolutions → flat-resolutions → glob-resolutions) is unchanged.
- [ ] `Resolved::aliased_from` is None for non-aliased entries and Some(real_name) for aliased entries.

## SemVer impact
- [ ] No change to items in `guroku::prelude`.
- [ ] No change to documented public function signatures in `resolver`/`specs`/`overrides`.
- [ ] If adding a new `DepSpec` variant: confirmed `DepSpec` is still `#[non_exhaustive]`.
- [ ] If adding a new `OverrideKind` variant: confirmed `OverrideKind` shape is intentional (it's currently NOT marked non_exhaustive — bumping it would be breaking).

## Resolver behaviour
- [ ] If you changed the backtracking path, `tests/resolver_no_backtrack.rs` still passes.
- [ ] If you changed alias handling, `tests/resolved_aliased_from.rs` and `tests/resolution_alias_lookup.rs` still pass.
- [ ] If you changed override precedence, `tests/overrides_path_keyed.rs`, `tests/overrides_glob.rs`, and `tests/overrides_v1_simple_compat.rs` all still pass.

## Performance
- [ ] No new O(n²) or worse loops in the hot path.
- [ ] If touching `try_backtrack`, sanity-check that the candidate-list iteration is still descending (highest-first).

## Testing
- [ ] Tests added under `tests/specs_*`, `tests/overrides_*`, or `tests/resolver_*`.
- [ ] `cargo test --all` passes locally.
- [ ] `cargo clippy --all-targets -- -D warnings` clean.
- [ ] `cargo fmt --check` clean.

## Documentation
- [ ] CHANGELOG.md `[Unreleased]` updated.
- [ ] `docs/internals/{npm-aliases,path-keyed-overrides,glob-resolutions,single-step-backtrack}.md` updated when implementation changed.
- [ ] `docs/{aliases,path-keyed-overrides,glob-resolutions}.md` updated when user-visible behaviour changed.

## Risk and rollback
(1-3 sentences. The resolver is a hot path; reverting needs the lockfile to be rebuildable from the older code.)
