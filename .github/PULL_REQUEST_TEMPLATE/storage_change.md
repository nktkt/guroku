## Summary
(1-3 sentences describing the change.)

## Affected modules
- [ ] `src/store.rs`
- [ ] `src/cache.rs`
- [ ] `src/linker.rs`
- [ ] `src/integrity.rs`
- [ ] `src/http_cache.rs`
- [ ] `src/tarball.rs`

## Storage compatibility
- [ ] On-disk layout under `~/.guroku/cas/` is unchanged (or backwards-compatible).
- [ ] Lockfile schema (`lockfileVersion`) is unchanged (or bumped, with migration plan).
- [ ] No change to the SHA-512 keying scheme.

## Atomicity and concurrency
- [ ] No new race conditions on concurrent `guroku install` against the same `~/.guroku`.
- [ ] CAS writes still go through the tmp-then-rename path.
- [ ] The `.guroku-cas-ready` marker is still written/checked correctly.

## Cross-platform
- [ ] Tested on Linux (Ubuntu CI).
- [ ] Tested on macOS (CI).
- [ ] Windows behaviour considered (or explicitly out of scope; see `docs/internals/strict-layout-windows.md`).
- [ ] Hardlink fallback path still works (tested by hand or covered by tests).

## Performance impact
- [ ] Wall-clock impact on warm install measured (or not applicable).
- [ ] Disk-usage impact analysed.
- [ ] No new unnecessary allocations or copies.

## Testing
- [ ] Tests added/updated for the changed behaviour.
- [ ] `cargo test --all` passes locally.
- [ ] `cargo clippy --all-targets -- -D warnings` is clean.
- [ ] `cargo fmt --all -- --check` is clean.

## Documentation
- [ ] CHANGELOG.md updated under `[Unreleased]`.
- [ ] `docs/internals/<relevant-page>.md` updated.
- [ ] User-facing impact noted in `docs/storage.md` or `docs/troubleshooting.md` if applicable.
- [ ] ARCHITECTURE.md updated if a module was added/renamed.

## Risk and rollback
(1-3 sentences: how risky is this change, and how would we revert?)
