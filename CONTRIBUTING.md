# Contributing to guroku

Thanks for your interest in guroku, a Rust-based, npm-style package manager.
This document covers how to get a working build, what we expect from patches,
and where to start if you want to take on something larger.

## Welcome and scope

guroku is in **pre-alpha**. The CLI surface, on-disk layout, lockfile format,
and internal module boundaries are all still in flux. Breaking changes land
without a deprecation cycle for the time being.

Because the design is moving, please **open an issue before sending non-trivial
PRs**. A short note describing the problem you want to solve and the rough
shape of the fix is enough — it lets us flag overlap with in-flight work and
saves you from rewriting a patch against a moving target. Small fixes (typos,
obvious bugs, doc clarifications) do not need a preflight issue; just send
the PR.

## Prerequisites

- **Rust 1.75 or newer.** Install via [rustup](https://rustup.rs). The MSRV
  is tracked in `Cargo.toml` and bumped deliberately.
- **`cargo`** — comes with the standard Rust toolchain.
- **A Unix-like environment.** Development is done on macOS and Linux.
  Windows is **not yet exercised**; patches that improve Windows support are
  welcome, but CI does not currently cover it, so you should expect to do
  your own validation there.

No other system dependencies are required to build the core crate.

## Building and testing

The standard cargo workflow applies:

```sh
# Build the workspace.
cargo build

# Run the full test suite (unit + integration).
cargo test

# Lint. We treat clippy warnings as errors in CI.
cargo clippy --all-targets -- -D warnings

# Format. rustfmt is the source of truth for style.
cargo fmt --all
```

Tests under `tests/` are **integration tests** — each file is its own crate
and exercises guroku through its public API or CLI. To run a single one:

```sh
cargo test --test <name>
```

For example, `cargo test --test install` runs only `tests/install.rs`.

If you are iterating on a single unit test, `cargo test <pattern>` filters
by test name across the whole suite.

## Project layout

The high-level design lives in [`ARCHITECTURE.md`](ARCHITECTURE.md). Read it
before making changes that cross module boundaries — it explains how the
resolver, fetcher, store, and CLI fit together, and which invariants each
layer is responsible for.

The source tree under `src/` is organized by responsibility (resolver,
manifest parsing, lockfile, store, CLI entry points, etc.). When in doubt,
follow existing patterns rather than introducing a new layout.

## Code style

- **rustfmt is the ground truth.** Run `cargo fmt --all` before committing.
  CI will reject unformatted code.
- **Prefer small, focused PRs.** A patch that does one thing is easier to
  review, easier to revert, and more likely to land quickly. If you find
  yourself touching unrelated code, split it.
- **Comments explain WHY, not WHAT.** The code already says what it does.
  Use comments to record the reason a particular approach was chosen, edge
  cases that motivated a check, or links to upstream discussions.
- **No magic numbers.** Promote literals with non-obvious meaning to named
  `const`s with a doc comment. Buffer sizes, timeouts, retry counts, and
  protocol limits all qualify.
- Prefer `?` over manual `match` on `Result` when the error path is just
  propagation. Use `thiserror` for library errors and `anyhow` only at the
  binary boundary.
- Avoid `unwrap` and `expect` outside of tests and provably-infallible
  paths. When you do use `expect`, the message should describe the invariant
  that makes the call safe.

## Commit messages

- **Imperative mood**: "Add resolver cache" rather than "Added" or "Adds".
- **Subject line: 50 characters or fewer**, no trailing period.
- A blank line, then a body that explains *why* the change is needed and
  any context a future reader will want. Wrap the body at 72 columns.
- **Reference issue numbers** when relevant: `Fixes #42`, `Refs #17`.

A good message looks like:

```
Cache resolved versions per registry response

The resolver was re-parsing the same registry document for every
transitive dependency, which dominated wall time on large graphs.
Memoize per (package, version-req) within a single resolution pass.

Fixes #57.
```

Squashing or rebasing into a clean history before review is appreciated
but not required — we can squash on merge if needed.

## Pull request checklist

Before requesting review, please confirm:

- [ ] `cargo fmt --all` is clean.
- [ ] `cargo clippy --all-targets -- -D warnings` is clean.
- [ ] Tests added or updated for the behavior you changed. Bug fixes should
      come with a regression test that fails without the fix.
- [ ] [`ARCHITECTURE.md`](ARCHITECTURE.md) updated if you added or renamed
      a public type, module, or subsystem boundary.
- [ ] [`CHANGELOG.md`](CHANGELOG.md) updated under the `## Unreleased`
      heading. One bullet per user-visible change; internal refactors do
      not need entries.
- [ ] PR description explains the motivation and links the relevant issue.

CI runs the same `fmt`, `clippy`, and `test` invocations listed above. If
CI is red, the PR is not ready for review.

## Reporting bugs

Use the **bug-report issue template** and include:

- The output of `guroku --version`.
- Your OS and version (e.g. `macOS 14.4`, `Ubuntu 22.04`).
- The exact command you ran, including any flags and the relevant
  `package.json` / lockfile excerpts.
- The full error output. If guroku panicked, include the backtrace
  (`RUST_BACKTRACE=1`).
- A minimal reproduction if you can produce one. A failing test case is
  ideal; a small example repository is the next best thing.

If you are unsure whether something is a bug or intended behavior, file
the issue anyway — at worst we close it with an explanation, and that
explanation often becomes documentation.

## Roadmap

The high-level roadmap lives in the [README](README.md). The natural next
contributions, in rough order, are:

- **v0.2 — resolver.** A real version resolver with proper backtracking,
  replacing the placeholder logic currently in `src/resolver`.
- **v0.3 — content-addressed store with hardlinks.** A global CAS shared
  across projects, with hardlinked `node_modules` for fast, deduplicated
  installs.

If you want to take on a roadmap item, please comment on the tracking
issue first so we can coordinate.

## Non-goals

guroku is a **package manager**. It is not, and will not become:

- A JavaScript runtime.
- A bundler, transpiler, or build tool.
- A monorepo task runner.

PRs that try to add these capabilities will be closed. Tooling that lives
*alongside* a package manager (workspaces, scripts, lifecycle hooks) is in
scope; tooling that *replaces* a separate ecosystem tool is not.

## Code of conduct

Participation in this project is governed by the
[Code of Conduct](CODE_OF_CONDUCT.md). By contributing, you agree to abide
by its terms.

## Security

Please do **not** report security vulnerabilities through public GitHub
issues. See [SECURITY.md](SECURITY.md) for the disclosure process and
contact details.

---

Thanks again for contributing. guroku is a small project today; well-aimed
patches make a real difference.
