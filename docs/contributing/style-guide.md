# Style guide

This is the canonical style reference for guroku contributors. It covers Rust
code, prose in `docs/`, tests, commit messages, and pull requests. If something
here conflicts with `rustfmt.toml`, `clippy.toml`, or `.editorconfig`, the
config files win — update this document instead.

For testing specifics, see [`docs/contributing/testing-guide.md`](./testing-guide.md).
For architectural conventions, see [`ARCHITECTURE.md`](../../ARCHITECTURE.md).

## Rust style

1. **rustfmt is canonical.** The repo's `rustfmt.toml` is the source of truth.
   Run `cargo fmt --all` before committing. CI will reject diffs that don't
   round-trip through rustfmt.

   ```sh
   cargo fmt --all
   cargo fmt --all -- --check   # what CI runs
   ```

2. **Clippy is enforced.** CI runs:

   ```sh
   cargo clippy --all-targets -- -D warnings
   ```

   Fix the warnings. Don't slap `#[allow(...)]` on a lint without a comment
   explaining why the lint is wrong for this site:

   ```rust
   // clippy::needless_collect: we need to materialise the iterator
   // before locking the cache, otherwise we deadlock on re-entry.
   #[allow(clippy::needless_collect)]
   let pkgs: Vec<_> = resolver.iter().collect();
   ```

3. **Module organisation.** One module per `src/<name>.rs`. Tests for that
   module either go in a `#[cfg(test)] mod tests` at the bottom of the file,
   or in `tests/<name>_*.rs` for integration-style tests.

   ```text
   src/
     resolver.rs        # module body + #[cfg(test)] mod tests
     lockfile.rs
   tests/
     resolver_cycles.rs # integration test, one behaviour
     lockfile_round_trip.rs
   ```

4. **Public surface.** Anything `pub` is API. Mark internal-only items
   `pub(crate)`. Don't leak third-party types into the public API except
   through the dedicated wrapper modules — for example, `version` re-exports
   `node_semver::{Range, Version}` so callers depend on `guroku::version`,
   not on `node_semver` directly.

   ```rust
   // src/version.rs
   pub use node_semver::{Range, Version};
   ```

5. **Errors.** Every error path returns `crate::error::Result<T>`. New error
   kinds get a variant in `GurokuError` with a `thiserror` Display template.
   Avoid stringly-typed `Other` unless the failure is genuinely a one-off.

   ```rust
   #[derive(Debug, thiserror::Error)]
   pub enum GurokuError {
       #[error("lockfile {path} is corrupt: {reason}")]
       LockfileCorrupt { path: PathBuf, reason: String },

       #[error("resolution failed for {name}@{range}")]
       Unresolvable { name: String, range: Range },

       // Last resort. Prefer a typed variant.
       #[error("{0}")]
       Other(String),
   }
   ```

6. **Logging.** Use `tracing::info!` / `debug!` / `warn!`.

   - `info!` is for things the user wants to see by default.
   - `debug!` is for diagnostics, surfaced behind `GUROKU_LOG=debug`.
   - `warn!` is for recoverable problems the user should know about.
   - Never `println!` from library code. CLI entry points may use `println!`
     for stdout output that is part of the contract (e.g. `guroku why`).

   ```rust
   tracing::info!(package = %name, version = %v, "installed");
   tracing::debug!(?cache_key, "cache miss");
   ```

7. **Comments explain why, not what.** Don't restate the function name in a
   doc comment. Document the surprising bit: invariants, ordering, why this
   loop can't be an iterator, why we hold the lock across the await.

   ```rust
   // Bad
   /// Resolves dependencies.
   fn resolve(...) {}

   // Good
   /// Resolves dependencies in BFS order so that diamond conflicts are
   /// reported at the shallower depth, matching npm's error output.
   fn resolve(...) {}
   ```

8. **No `unwrap` / `expect` outside tests** and "this can never happen" code.
   Use `?`. If you must `expect`, the message should describe why the panic
   is impossible — not what was being unwrapped.

   ```rust
   // Bad
   let cfg = Config::load().expect("load config");

   // Good
   let cfg = Config::load()?;

   // Acceptable
   let regex = Regex::new(r"^\d+$")
       .expect("regex literal is known-valid at compile time");
   ```

9. **Async: tokio only.** Use `tokio` types throughout. Don't mix
   `futures-cpupool`, `async-std`, or other runtimes. For CPU-bound work,
   `tokio::task::spawn_blocking`. For parallel I/O, `futures::future::try_join_all`
   on `tokio` futures.

10. **Dependencies require justification.** Adding a new crate to `Cargo.toml`
    needs a paragraph in the PR description: why we need it, what we
    considered instead, and the licence. Prefer the standard library.
    Prefer crates already in the tree.

## Prose style (docs)

1. **Active voice.** "guroku writes the lockfile" beats "the lockfile is
   written". Name the actor.

2. **Concrete examples.** Every concept gets a fenced shell or Rust snippet.
   If you can't write an example, the concept probably isn't ready to
   document.

3. **No emoji** in source files or docs. Use markdown bullets, tables, and
   `**bold**` for emphasis. This matches the project-wide CLAUDE-style
   preference and keeps grep output clean.

4. **Heading capitalization is sentence case** for H2 and H3.

   ```md
   ## Disk usage          <!-- yes -->
   ## Disk Usage          <!-- no  -->
   ### Resolving versions <!-- yes -->
   ```

   H1 (page title) follows the same rule.

5. **Cross-link aggressively.** Every doc that references a concept should
   link to the canonical doc for that concept. A mention of "strict layout"
   in `troubleshooting.md` links to
   [`docs/internals/strict-layout.md`](../internals/strict-layout.md). A
   mention of `cargo fmt` in this file links to [the rustfmt section](#rust-style)
   above. Broken links are caught by `cargo xtask check-docs`.

6. **Update prose and tests together.** If you change a public API, update
   its doc page in the same PR. A PR that ships an API change without a
   docs change will be sent back.

7. **Trailing newline on every file.** `.editorconfig` enforces this; most
   editors do it for free. Files ending mid-line will fail
   `cargo xtask check-docs`.

8. **Code fences need a language tag.**

   ````md
   ```sh
   guroku install
   ```

   ```rust
   let v = Version::parse("1.2.3")?;
   ```

   ```json
   { "name": "guroku" }
   ```
   ````

   Untagged blocks render without highlighting and break some renderers'
   copy-button behaviour. Use ```text``` if there's genuinely no language.

## Test style

This is the short version. The full guide lives in
[`docs/contributing/testing-guide.md`](./testing-guide.md).

1. **One behaviour per test file.** The filename describes the behaviour:
   `tests/resolver_handles_peer_cycles.rs`, not `tests/resolver.rs`.
2. **Tempfile for any disk side effects.** Never write to the repo, the home
   directory, or `/tmp` without a randomised path. Use the `tempfile` crate.
3. **No network in tests.** Stub the registry. Tests must pass on a
   disconnected laptop.

## Commit messages

1. **Subject line ≤ 50 chars, imperative mood.** "Add lockfile v3 support",
   not "Added lockfile v3 support" and not "Adding lockfile v3 support".
2. **Body explains the why** when it isn't obvious from the diff. Wrap at
   72 chars. Reference issues with `Fixes #123` on its own line.
3. **Co-author trailers on collaborative commits**:

   ```text
   Co-authored-by: Alice <alice@example.com>
   ```

4. **No emoji** in commit messages, including no `:sparkles:`-style aliases
   and no Conventional-Commits-with-emoji prefixes.

Example:

```text
Cache resolved version ranges per session

Re-resolving the same range against the same registry response was
showing up as 8% of `guroku install` wall time on the monorepo
benchmark. The cache is keyed on (name, range, registry-etag) and
lives for the life of the CLI invocation.

Fixes #412
```

## Pull requests

1. **One topic per PR.** Don't bundle a refactor with a feature. Reviewers
   should be able to say yes or no to a single coherent change.
2. **Update `CHANGELOG.md`** under the `[Unreleased]` section. Add a bullet
   under `Added`, `Changed`, `Fixed`, or `Removed`.
3. **Update `ARCHITECTURE.md`** if you add, remove, or rename a module, or
   change the data flow between modules.
4. **Re-run the full local check** before pushing:

   ```sh
   cargo fmt --all
   cargo clippy --all-targets -- -D warnings
   cargo test --all
   ```

   If any of these fail in CI but pass locally, your toolchain is probably
   ahead of `rust-toolchain.toml`. Update the toolchain pin in a separate
   PR.

## When in doubt

Match the surrounding code. If the surrounding code is wrong, fix it in a
follow-up PR rather than mixing the fix into an unrelated change.
