## Summary
(1-3 sentences.)

## Affected modules
- [ ] `src/scripts.rs`
- [ ] `src/commands/run.rs`
- [ ] `src/commands/exec.rs`
- [ ] `src/commands/install.rs` (lifecycle hooks)
- [ ] `src/manifest.rs` (scripts/bin fields)
- [ ] `src/linker.rs` (.bin shims)

## Behaviour preserved
- [ ] Lifecycle hook order still: preinstall -> resolve+download+link -> per-pkg postinstall -> install -> postinstall -> prepare.
- [ ] `--ignore-scripts` still skips both root and per-pkg scripts.
- [ ] Per-pkg postinstall failures still warn-and-continue (root failures still abort).
- [ ] `guroku run` argument forwarding via `-- args` still works.
- [ ] `guroku exec` precedence (.bin first, PATH second) unchanged.
- [ ] `node_modules/.bin/` shims still relative-targeted.

## Cross-platform
- [ ] Linux (CI).
- [ ] macOS (CI).
- [ ] Windows behaviour considered (sh->cmd shell switching, symlinks needing Developer Mode).
- [ ] No bashisms in test scripts.

## Security
- [ ] No new path that runs arbitrary script bodies without going through `scripts::run_in` or `commands::exec`.
- [ ] PATH augmentation order unchanged (project bin first).
- [ ] No env-var leakage from guroku's parent process beyond what's documented.

## Testing
- [ ] New tests under `tests/scripts_*` or `tests/cli_*`.
- [ ] `cargo test --all` passes locally.
- [ ] `cargo clippy --all-targets -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.

## Documentation
- [ ] CHANGELOG.md `[Unreleased]` updated.
- [ ] `docs/scripts.md` / `docs/lifecycle.md` updated if user-visible behaviour changed.
- [ ] `docs/internals/scripts.md` / `docs/internals/lifecycle.md` updated if implementation changed.

## Risk and rollback
(1-3 sentences.)
