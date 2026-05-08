## Summary
(1-3 sentences.)

## API surface affected
- [ ] Item re-exported by `guroku::prelude` (SemVer-covered).
- [ ] Crate-root re-export (`guroku::Result` / `guroku::GurokuError`).
- [ ] `pub` item NOT in `prelude` (semi-stable; minor releases can rename).
- [ ] `#[doc(hidden)]` item (internal; minor releases can do anything).
- [ ] Lockfile schema (`lockfileVersion: 1`).
- [ ] CLI subcommand or flag.

## SemVer impact
- [ ] No SemVer impact (additive change, fully backward-compatible).
- [ ] Additive change to a `#[non_exhaustive]` type (allowed in minor).
- [ ] Renames that ship with a `#[deprecated]` re-export of the old name.
- [ ] Breaking change targeting v2.0 (justify below).

## If breaking
- [ ] Listed in `docs/migration/v1-to-v2.md` (create if not present).
- [ ] CHANGELOG.md `[Unreleased]` notes the break under `### BREAKING`.
- [ ] An RFC-ish issue exists referencing this PR.

## If renaming with deprecation
- [ ] Old name still re-exported with `#[deprecated(since = "X.Y.Z", note = "...")]`.
- [ ] CHANGELOG.md notes the deprecation under `### Deprecated`.
- [ ] Removal target version specified in the deprecation note.

## Testing
- [ ] `cargo test --all` passes.
- [ ] `cargo clippy --all-targets -- -D warnings` clean.
- [ ] `cargo fmt --check` clean.
- [ ] `cargo doc --no-deps --all-features` with `RUSTDOCFLAGS=-D warnings` clean.
- [ ] `tests/api_stability_*.rs` updated if a `prelude` re-export changed.
- [ ] `tests/cli_help_v1.rs` / `tests/cli_subcommand_inventory.rs` updated if a CLI subcommand or flag changed.
- [ ] `tests/lockfile_v1_compat.rs` / `tests/lockfile_format_stability.rs` updated if the lockfile schema changed (and `lockfileVersion` bumped if needed).
- [ ] `cargo-semver-checks` workflow passes (or its result is overridden with a v2.0 justification).

## Documentation
- [ ] Rustdoc updated on every changed public item.
- [ ] `docs/STABILITY.md` updated if a stability promise was added/relaxed.
- [ ] `docs/api-overview.md` / `docs/embedding-guroku.md` updated for embedder-visible changes.
- [ ] `docs/cli-reference.md` updated for CLI changes.
- [ ] `docs/lockfile-format.md` updated for lockfile schema changes.

## Risk and rollback
(1-3 sentences. Be honest.)
