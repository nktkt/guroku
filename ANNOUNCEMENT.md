# guroku 1.0

It's stable.

After five preview releases, guroku 1.0 is here. This release ships the
first stability commitment for the project: the lockfile schema, the
Rust API surface (rooted at `guroku::prelude`), and the CLI are now
covered by SemVer. Within the 1.x line, your lockfiles will keep
parsing, your `Cargo.toml` dependency on `guroku = "1"` will keep
compiling, and your scripts and CI invocations will keep working.

This is the moment guroku stops being an experiment and becomes a
library you can depend on.

## The journey

guroku went from empty repository to 1.0 in six iterations:

- **v0.1 (2026-05-06)** — install pipeline. The first end-to-end
  install: parse `package.json`, talk to the registry, fetch tarballs,
  unpack them into `node_modules`. Nothing fancy, but it ran.
- **v0.2 (2026-05-06)** — real resolver and lockfile. A proper
  dependency resolver replaced the placeholder, and `guroku.lock`
  arrived as a deterministic, content-addressed record of the resolved
  graph.
- **v0.3 (2026-05-06)** — content-addressable store, hardlinks, strict
  layout. Packages began landing in a global CAS and getting hardlinked
  into a strict, pnpm-style `node_modules` layout. Disk usage dropped;
  installs got dramatically faster on warm caches.
- **v0.4 (2026-05-06)** — lifecycle scripts and the `run` / `exec` /
  workspaces trio. npm-compatible lifecycle hooks (`preinstall`,
  `install`, `postinstall`, etc.) landed alongside `guroku run`,
  `guroku exec`, and first-class workspace support.
- **v0.5 (2026-05-06)** — private registries, file and git
  dependencies, overrides, and `guroku audit`. The release that closed
  the gap with the incumbents on day-to-day workflows: scoped registry
  auth, `file:` and `git:` specs, version overrides, and a vulnerability
  audit pipeline.
- **v1.0 (2026-05-08)** — stability commitment. No new headline
  features; instead, a public, documented, SemVer-bound surface that
  downstream tooling can build on.

## What 1.0 promises

The 1.0 stability commitment covers three surfaces:

- **Lockfile schema.** `guroku.lock` files written by 1.x will be
  readable by every later 1.x release. New optional fields may be
  added; existing fields will not change meaning or disappear.
- **Rust API.** Anything reachable from `guroku::prelude` is covered.
  Items outside the prelude or marked `#[doc(hidden)]` are not part of
  the stable surface. `GurokuError` is `#[non_exhaustive]` so we can
  add error variants without a major bump.
- **CLI surface.** Documented subcommands, flags, and exit codes are
  stable. Output meant for humans may be polished; output behind
  `--json` is treated as a contract.

The full policy lives in [`docs/STABILITY.md`](docs/STABILITY.md).

## What's new in 1.0

Beyond the stability promise, 1.0 ships the polish that makes guroku
pleasant to embed and operate:

- Comprehensive crate-level rustdoc, including end-to-end embedding
  examples for using guroku as a library inside other Rust tools.
- A `guroku::prelude` module that re-exports the canonical public
  types, so most embedders only need a single `use` line.
- `GurokuError` is now marked `#[non_exhaustive]`. Match on it with a
  catch-all arm; new variants will not break your build on a minor
  bump.
- Pre-built Windows binaries on every release, alongside the existing
  macOS and Linux artifacts.
- Criterion benchmark scaffolding under `benches/`, so performance
  changes are measurable rather than anecdotal.

## Where to find things

- [`README.md`](README.md) — quick start, install, basic usage.
- [`docs/api-overview.md`](docs/api-overview.md) — a tour of the public
  Rust API and how the pieces fit together.
- [`docs/embedding-guroku.md`](docs/embedding-guroku.md) — worked
  examples for using guroku as a library: programmatic install,
  resolution-only flows, custom registry clients.
- [`docs/v1.0-release-notes.md`](docs/v1.0-release-notes.md) — the
  full release notes for 1.0, including the complete changelog since
  0.5.
- [`docs/STABILITY.md`](docs/STABILITY.md) — the SemVer policy, in
  detail.

## What's next

1.0 is a commitment, not a stopping point. Work continues on the 1.x
line; none of the items below are breaking, and all are planned to
land in minor releases:

- **PubGrub-based resolver.** The current BFS sticky-first solver is
  fast and correct for the workloads we have, and it remains in
  guroku 1.x for the foreseeable future. A PubGrub-based resolver is
  in development and will land in a 1.x minor release behind a flag
  before becoming the default; both will produce the same lockfile
  format.
- **Workspace inter-dependency linking.** Better support for
  workspace packages that depend on each other, including symlink
  layouts that match the expectations of bundlers and TypeScript's
  project references.
- **Macrobench harness with published comparisons.** A reproducible
  macrobenchmark suite, plus regularly updated comparisons against
  npm, pnpm, bun, and yarn on a fixed corpus of real-world projects.

## Thanks

<!-- TODO: maintainer to acknowledge contributors here before publishing. -->

## Try it

```sh
git clone https://github.com/nktkt/guroku.git
cd guroku
cargo build --release
./target/release/guroku --version
```

## Reach out

Bug reports and feature requests belong in the issue tracker:
<https://github.com/nktkt/guroku/issues>.

For questions, design discussion, and showing off what you've built,
join us in GitHub Discussions:
<https://github.com/nktkt/guroku/discussions>.

Thanks for trying guroku. Onward to 1.x.
