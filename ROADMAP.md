# Guroku Roadmap

This document is the long-form companion to the roadmap bullet list in the
project README. The README answers "where is guroku going?" in a glance; this
file answers "what does it actually mean to land milestone X, and what should I
pick up if I want to help?"

Each milestone below contains:

- a one-paragraph theme statement that frames the goal
- a bullet list of features lifted from the README and elaborated with the
  engineering substance behind them
- entry and exit criteria — what we expect to be true before code merges into
  `main`, and the bar a tag has to clear before we publish it
- a current status line so you can tell at a glance whether the milestone is
  shipped, in progress, or untouched

The version numbers here are not promises about calendar dates. They are an
ordering of problems we want to solve and the order we plan to solve them in.
If a milestone gets reshuffled, this file gets updated alongside the README so
the two never disagree.

---

## v0.1 — "It installs something"

### Theme

The first milestone exists to prove the end-to-end pipeline works at all: a
user types `guroku install`, and a tree of packages they can `require` shows
up in `node_modules`. Correctness, performance, and ergonomics are explicitly
out of scope. The goal is a vertical slice that touches every layer — CLI,
manifest parsing, registry HTTP, tarball extraction, on-disk layout — so we
have something concrete to harden in later milestones.

### Features

- **CLI surface: `install`, `add`, `remove`.** A minimal command set that
  mirrors what users already expect from npm. `install` reads `package.json`
  and materializes a `node_modules`. `add <pkg>` writes a new dependency entry
  and installs it. `remove <pkg>` does the inverse. No flags beyond the bare
  minimum required to make the commands behave.
- **`package.json` parser.** A serde-backed reader that understands
  `dependencies`, `devDependencies`, `peerDependencies`, and
  `optionalDependencies` well enough to enumerate them. We do not yet act on
  the distinction between dep kinds — that is v0.2 — but we record them.
- **npm registry client.** A thin HTTPS client over `reqwest` (or equivalent)
  that fetches packument JSON from `https://registry.npmjs.org` and resolves a
  given name + version range to a concrete tarball URL. No caching, no ETag,
  no retries beyond what the HTTP library gives us for free.
- **`.tgz` extraction.** Stream a gzipped tar from the registry, strip the
  leading `package/` prefix, and write files to disk with the correct mode
  bits. Symlinks in tarballs are rejected for now.
- **Naive flat `node_modules`.** Every resolved package is written to
  `node_modules/<name>` at the top level. We do not deduplicate, hoist, or
  nest. If two packages disagree on a dependency version, last writer wins.
  This is wrong, and we know it; v0.2 fixes resolution and v0.3 fixes layout.
- **SHA-512 integrity checks.** The `integrity` field from the registry is
  parsed and the downloaded tarball is verified before extraction. A mismatch
  aborts the install with a non-zero exit code.

### Entry criteria

- Cargo workspace exists with at least a `guroku-cli` binary crate and a
  `guroku-core` library crate.
- A CI job runs `cargo test` and `cargo clippy -- -D warnings` on Linux.
- An integration test fixture installs a small real-world package
  (`is-odd`-tier, no native build) end-to-end against the live registry.

### Exit criteria for merging into `main`

- All of the features above behave on at least Linux and macOS.
- `cargo test` passes on CI.
- The README install instructions match what the binary actually does.

### Exit criteria for tagging v0.1

- A user who has never used guroku before can `cargo install --git` it,
  `cd` into a fresh directory with a `package.json`, run `guroku install`,
  and have a `node_modules` they can `node -e "require('the-pkg')"` against.
- The CLI prints a clearly-marked "experimental" banner so nobody mistakes
  this for a stable tool.

### Current status

Shipped 2026-05-06. This is what is on `main` today.

---

## v0.2 — "It resolves correctly"

### Theme

v0.1 installs whatever the registry hands back for the latest matching
version, and silently picks a winner when packages conflict. v0.2 replaces
that with a real solver. The goal is that two different machines running
`guroku install` against the same `package.json` always produce the same
tree, and that the tree is provably consistent with every package's declared
constraints.

### Features

- **Semver parser.** A node-compatible semver implementation: caret, tilde,
  hyphen ranges, `x` and `*` wildcards, prerelease ordering, build metadata.
  Behavior is matched against the npm reference, not the Rust `semver` crate
  (which is stricter than node-semver in several places).
- **PubGrub-based resolver.** PubGrub gives us correct, fast, and — most
  importantly — explainable resolution. When a solve fails, the error message
  walks the user through the conflict instead of dumping a stack trace. The
  resolver is decoupled from the registry client behind a trait so it can be
  unit-tested with synthetic packages.
- **`guroku.lock`.** A textual lockfile that captures the resolved tree:
  every package, version, integrity hash, registry URL, and the dependency
  edges that justify it. The format is versioned (`lockfile_version = 1`) and
  designed to be diff-friendly. v1.0 will freeze this format; until then
  expect breaking changes between minor versions.
- **Frozen-lockfile mode.** `guroku install --frozen-lockfile` (and an
  implicit frozen mode in CI) refuses to update `guroku.lock`. If the
  lockfile and `package.json` disagree, the install errors instead of
  silently fixing things up. This is the mode we want CI to use.
- **dev / peer / optional dependency handling.** `--production` skips
  `devDependencies`. `peerDependencies` participate in resolution and produce
  warnings (not errors, yet) when unsatisfied. `optionalDependencies` that
  fail to resolve or install do not abort the run.

### Entry criteria

- v0.1 is tagged and the integration test for the v0.1 happy path still
  passes.
- A semver test corpus is in tree, ideally lifted from the node-semver test
  suite, so we can pin our parser against a known-good oracle.

### Exit criteria for merging into `main`

- The resolver has unit tests covering: trivial linear deps, diamond deps
  with a unique solution, diamond deps with no solution (must produce a
  human-readable explanation), prerelease selection, and peer-dep warnings.
- `guroku install` followed by `guroku install --frozen-lockfile` is a no-op
  on a clean checkout.
- `guroku.lock` round-trips through write-then-parse without churn.

### Exit criteria for tagging v0.2

- We can install ten popular real-world packages (express, react, vite,
  webpack, eslint, typescript, jest, lodash, axios, next) and the resulting
  tree passes each package's own smoke test.
- The lockfile produced on Linux is byte-identical to the one produced on
  macOS for the same input.

### Current status

Shipped 2026-05-06.

---

## v0.3 — "It's fast"

### Theme

By v0.2 guroku is correct but slow. Every install re-downloads, re-extracts,
and re-writes everything from scratch. v0.3 is the performance milestone: a
content-addressed store, hardlinked package files, a strict pnpm-style
layout, and a parallel pipeline. The bar is "noticeably faster than npm on a
warm cache, competitive with pnpm."

### Features

- **Global content-addressed store at `~/.guroku/store`.** Files are stored
  by their SHA-512, deduplicated across every project on the machine. The
  store is the source of truth; project `node_modules` directories are
  hardlink views into it. Garbage collection is deferred to v0.4 or later;
  for now the store grows monotonically.
- **Hardlinks (with copy fallback).** Package files are linked into
  `node_modules` instead of copied. On filesystems that don't support
  hardlinks (some Docker overlays, some Windows configurations) we fall back
  to copy and warn once per run.
- **Strict pnpm-style layout.** Packages live at
  `node_modules/.guroku/<pkg>@<ver>/node_modules/<pkg>/`, and the top-level
  `node_modules/<pkg>` is a symlink into that path. A package can only see
  the dependencies it actually declared, which catches the "works on my
  machine because of phantom deps" class of bugs at install time.
- **Parallel resolve + download pipeline.** Resolution and downloading run
  concurrently: as soon as a package is resolved its tarball fetch is
  enqueued, and extraction happens on a thread pool. A single tokio runtime
  with bounded channels coordinates the stages. The number of concurrent
  network requests is capped and configurable.
- **ETag / `If-None-Match` HTTP cache.** Packument requests cache the
  registry's ETag and revalidate on subsequent installs. A 304 means we skip
  the JSON parse entirely. Tarballs are content-addressed so they never need
  revalidation.

### Entry criteria

- v0.2 is tagged.
- A benchmark harness exists that can run `guroku install` against a fixed
  set of fixtures and report wall-clock time, peak RSS, and bytes
  transferred. Numbers are recorded in tree so regressions are visible.

### Exit criteria for merging into `main`

- Cold-cache install of a medium project (next.js scaffold, ~1k deps) is at
  least 2x faster than the v0.2 baseline.
- Warm-cache install of the same project is at least 5x faster than v0.2 and
  transfers zero tarball bytes.
- The strict layout actually rejects phantom deps in a regression test.

### Exit criteria for tagging v0.3

- Benchmarks vs npm and pnpm on the same hardware are published in the repo,
  with the methodology, and guroku is at worst within 20% of pnpm on warm
  installs.
- Store layout is documented well enough that an outsider could write a
  third-party tool against it.

### Current status

Shipped 2026-05-06.

---

## v0.4 — "It's usable"

### Theme

A package manager that can only install dependencies is a toy. v0.4 is about
making guroku a viable day-to-day replacement for npm/pnpm in real projects.
That means running scripts, supporting workspaces, executing local binaries,
and respecting the configuration users already have in `.npmrc`.

### Features

- **Lifecycle scripts.** `preinstall`, `install`, `postinstall`, `prepare`,
  and the `pre`/`post` siblings around `guroku run <script>`. Scripts run in
  a shell with `node_modules/.bin` on PATH and the standard npm env vars
  (`npm_package_*`, etc.) populated for compatibility. By default install
  scripts from third-party packages are gated behind a confirmation or an
  explicit allowlist; the recent supply-chain incidents make this
  non-negotiable.
- **Workspaces / monorepo support.** A `workspaces` field in the root
  `package.json` declares child packages. Cross-workspace dependencies are
  resolved to local symlinks, hoisted dependencies live at the root, and
  `guroku run -r <script>` runs a script across every workspace in
  topological order.
- **`guroku run`.** Executes a script defined in `package.json` with the
  same PATH and env setup as npm. Forwards stdio, propagates exit codes,
  handles signals.
- **`guroku exec` and `guroku dlx`.** `exec` runs a binary from the local
  `node_modules/.bin`. `dlx` (npx-style) installs a package into a temporary
  prefix and runs its bin, then cleans up. The temp prefix is itself stored
  in the CAS, so repeated `dlx` calls of the same package are nearly free.
- **`.npmrc` compatibility.** We read `.npmrc` from the standard locations
  (project, user home, global) and honor at least: registry, scoped registry
  overrides, `_authToken`, `cafile`, `strict-ssl`, `proxy`, `https-proxy`,
  `noproxy`. Unknown keys are ignored with a debug log, not an error.

### Entry criteria

- v0.3 is tagged.
- We have telemetry from a few real projects' install runs (via volunteers
  or our own dogfooding) so we know which scripts and `.npmrc` features
  matter most in practice.

### Exit criteria for merging into `main`

- Workspaces test fixture with three packages and one cross-dep installs
  cleanly and `guroku run -r build` builds them in the correct order.
- `guroku dlx create-vite my-app` works end-to-end.
- An `.npmrc` with a scoped registry plus auth token successfully installs
  a package from that scope.

### Exit criteria for tagging v0.4

- A sample of five real OSS monorepos (turbo, nx, etc.) installs and builds
  with guroku as a drop-in replacement, modulo documented exceptions.
- Lifecycle script gating defaults are reviewed by someone with security
  background and the policy is documented.

### Current status

Shipped 2026-05-06.

---

## v0.5 — "It plays nice"

### Theme

v0.5 is the "fits into existing infrastructure" milestone. By this point
guroku works for greenfield projects; this milestone makes it work for the
projects people actually have, which means private registries, git deps,
local path deps, version overrides, and a security audit story.

### Features

- **Private registries and scoped tokens.** Per-scope registry URLs and auth
  tokens, sourced from `.npmrc`, environment variables (`NPM_TOKEN`-style),
  or a future `guroku login` flow. Auth headers are scrubbed from logs and
  error messages by default.
- **Git dependencies.** `"foo": "github:user/repo#tag"` and full git URL
  forms (`git+https`, `git+ssh`). Resolution pins to a commit SHA in
  `guroku.lock`, and the package is treated as if it had been published with
  whatever `package.json` is at that SHA.
- **File and local dependencies.** `"foo": "file:../foo"` symlinks (or
  copies, on platforms where symlinks are awkward) to a local path.
  Workspaces are the preferred mechanism for monorepos; `file:` is for the
  one-off cases workspaces don't cover.
- **Overrides / resolutions.** A top-level `overrides` field in
  `package.json` lets a project pin a transitive dependency to a specific
  version. Behavior matches npm's `overrides`; we will not implement Yarn's
  `resolutions` field as a separate thing, only as an alias.
- **`guroku audit`.** Runs the installed tree against a vulnerability
  database (GitHub Advisory Database to start) and prints a report. Exits
  non-zero on findings at or above a configurable severity. No automatic
  fixing in this milestone — that is harder than it looks and we would
  rather ship a correct read-only audit than a flaky `audit fix`.

### Entry criteria

- v0.4 is tagged.
- We have a private-registry test target — either a self-hosted Verdaccio
  instance in CI, or a sandbox account on a commercial registry.

### Exit criteria for merging into `main`

- Installing from a private registry with token auth works and is covered
  by an integration test.
- A git dep pinned by tag installs reproducibly: the same tag on two
  machines produces the same lockfile entry.
- `guroku audit` produces output equivalent to `npm audit --json` for a
  fixture project with known vulnerabilities.

### Exit criteria for tagging v0.5

- At least one team external to the maintainers reports a successful
  migration of a private monorepo to guroku.
- Audit data source is configurable so users behind firewalls can point at
  internal mirrors.

### Current status

Shipped 2026-05-08.

---

## v1.0 — "It's stable"

### Theme

v1.0 is the commitment milestone. The lockfile format stops changing under
users' feet, the public Rust API gets compatibility guarantees, the binary
runs everywhere people expect it to run, and we have numbers — real,
defensible benchmarks — backing up our claims.

### Features

- **Stable lockfile.** `lockfile_version = 1` is frozen. Future versions
  will be additive or gated behind a new version number with a documented
  migration path. We will not silently rewrite a v1 lockfile in a v1.x
  release.
- **Documented public Rust API.** `guroku-core` exposes a stable API for
  embedding the resolver, the lockfile, and the store. Types in this surface
  follow Rust API guidelines and SemVer. Anything outside the documented
  surface is `#[doc(hidden)]` and may change.
- **Cross-platform CI.** Linux (gnu and musl), macOS (x86_64 and aarch64),
  and Windows (x86_64) are tier-1: every PR runs the full test suite on
  every platform, and a release blocks on all of them being green.
- **Prebuilt binaries.** Each tag publishes static binaries for the tier-1
  targets via GitHub Releases, plus an install script and homebrew/scoop
  recipes. The Cargo install path keeps working for people who want to
  build from source.
- **Benchmark suite vs npm / pnpm / bun / yarn.** A reproducible benchmark
  harness with documented methodology, run on every release, comparing cold
  install, warm install, and lockfile-only install across a fixed corpus of
  projects. Results are committed alongside the release notes.

### Entry criteria

- v0.5 is tagged.
- A v1 lockfile RFC has been open for at least four weeks and merged.
- Every public item in `guroku-core` either has rustdoc and tests, or is
  marked private before the freeze.

### Exit criteria for merging into `main`

- A semver-checking tool (e.g. `cargo-semver-checks`) gates `guroku-core`
  PRs.
- All tier-1 platforms are green on `main`.

### Exit criteria for tagging v1.0

- The CHANGELOG has a clear "what stability means" section and a list of
  what we are explicitly not committing to.
- The benchmark suite has been run on a known reference machine and the
  results published in the release notes.
- At least three downstream projects have signed off on the v1 API.

### Current status

Shipped 2026-05-08.

---

## v1.1 — "It resolves better"

### Theme

v1.0 froze the surface; v1.1 starts adding the features people actually asked
for, on top of that frozen surface. The theme is "make resolution itself
smarter and more controllable" — npm-style aliases so two versions of the
same registry package can live side-by-side under different local names,
path-keyed and glob overrides so projects can pin transitives precisely, and
single-step backtracking so the resolver no longer dies on common diamond
conflicts. PubGrub-the-crate is intentionally NOT in this milestone.

### Features

- **`DepSpec::Alias` and `npm:` parsing.** `"react-old": "npm:react@^16"`
  classifies as `Alias { real_name: "react", inner: Range("^16") }`. Splits
  on the LAST `@` so scoped real names work. `unparse` round-trips. Aliased
  entries land in `node_modules/<local_name>/`, with `Resolved.aliased_from
  = Some(real_name)` for downstream tooling.
- **Path-keyed overrides.** `"a > b > c": "1.0.0"` in `package.json#overrides`
  pins `c` only when reached through `a → b → c`. Whitespace tolerant.
  Implemented as a suffix match on the resolution path.
- **Glob resolutions.** `"**/<name>": "1.0.0"` in `package.json#resolutions`
  pins any `<name>` anywhere in the dep tree. Honours the literal
  `**/<name>` shape only; richer globs are v1.x backlog.
- **Documented precedence ladder.** exact-path overrides → flat overrides →
  exact-path resolutions → flat resolutions → `**/<name>` resolutions. The
  v1.0 `lookup` shim survives, calling `lookup_with_path(&[name])` under
  the hood — strictly more permissive than v1.0 but never less so.
- **Single-step backtracking.** When a diamond conflict arrives — package X
  needs `dep@^1.2` but the resolver already chose `dep@1.1.0` for someone
  else — the resolver walks the candidate list highest-first looking for a
  version that satisfies BOTH ranges. If found, it substitutes. If not, it
  errors with a path-formatted `ResolutionConflict`. v1.0's BFS sticky-first
  would have died on the spot.
- **Path-formatted conflict errors.** `ResolutionConflict.requested_by` now
  carries `"a > b > c"` so users can see which dep chain wanted what.
- **Manifest-aware resolver entry point.** `resolver::resolve_with_manifest_overrides(client, roots, manifest)` is the preferred way to drive the
  resolver from a Manifest. The older `resolve_with_overrides` continues to
  work (and is what the new entry point ultimately calls into).

### Entry criteria

- v1.0 is tagged.
- The path-keyed override format has been dogfooded on at least one
  in-tree fixture.

### Exit criteria for merging into `main`

- Aliased deps install side-by-side with their non-aliased namesakes; the
  `tests/manifest_aliases_dont_collide.rs` fixture passes.
- Path-keyed and glob entries are honoured by `resolver::resolve_with_manifest_overrides`; `tests/overrides_path_keyed.rs` and
  `tests/overrides_glob.rs` cover the matching ladder.
- A diamond-conflict regression case (one transitive that two roots want at
  incompatible ranges) backtracks successfully when a satisfying version
  exists, errors cleanly when it doesn't.

### Exit criteria for tagging v1.1

- All v1.0 tests still pass (`tests/api_stability_*`, `tests/lockfile_v1_compat.rs`, `tests/cli_help_v1.rs`, etc.). v1.1 is strictly additive on the
  documented v1.0 surface.
- A v1.0 lockfile is consumed by v1.1 with no migration.
- `docs/migration/v1.0-to-v1.1.md` is published.
- Release notes call out the deferred PubGrub work explicitly so users
  aren't surprised when their pathological case still needs an override.

### Current status

Shipped 2026-05-08.

---

## v1.2 — "It backtracks properly" — Shipped



### Theme

v1.1 ships single-step backtracking, which is enough for the common diamond
case but not enough for cascading conflicts where one substitution forces
another. v1.2 brings in PubGrub for real.

### Features

- **PubGrub integration.** Wire `pubgrub-the-crate` behind the existing
  resolver entry points. Convert npm-semver Ranges to pubgrub Ranges; bridge
  the sync trait surface to the async metadata client (probably with a
  prefetch step that hydrates everything pubgrub needs in advance).
- **Explainable conflicts.** When pubgrub fails, surface the human-readable
  conflict trace it produces, formatted with our `>`-style path syntax.
- **Path-aware backtracking.** Path-keyed overrides participate in
  pubgrub's incompatibility tracking so they're not invisible to the
  conflict explainer.
- **Resolver determinism guarantees.** Two runs against the same registry
  metadata produce byte-identical lockfiles, independent of network
  ordering. v1.1 inherits this from sticky-first; v1.2 keeps it under
  pubgrub.

### Entry criteria

- v1.1 is tagged.
- A test corpus of diamond and cascade conflicts (both solvable and
  insoluble) is in tree.

### Exit criteria for tagging v1.2

- The corpus solves where pubgrub-the-crate says it should and fails with
  a readable explanation where it shouldn't.
- Lockfile bytes for the v1.1 test fixtures are unchanged (or the diff is
  documented).

### Current status

Not started.

---

## v1.3 — "Workspaces, properly"

### Theme

v0.4 added a `guroku workspaces` subcommand and discovery; v1.3 makes
workspaces a first-class peer of registry deps in the resolver and linker.

### Features

- **Cross-workspace symlinks.** A workspace package depending on a sibling
  workspace package gets a symlink to the sibling's source dir, no CAS
  detour.
- **Topological `guroku run -r <script>`.** Runs scripts across workspaces
  in dependency order, with stream-prefixed output and a final summary.
- **Workspace-scoped lockfile.** A single root lockfile captures the
  resolution for all workspaces. Per-workspace `node_modules` is wired so
  each workspace sees only its declared deps (plus hoisted ones).
- **Workspace protocols.** `"@scope/sibling": "workspace:^"`, `"workspace:*"`,
  `"workspace:~"` are recognised. The `workspace:` prefix bypasses registry
  fetching and points at the local workspace.

### Current status

Not started.

---

## v1.4 — "It runs offline"

### Theme

Make the CAS portable so an air-gapped install is one `tar` extract away.

### Features

- **`guroku store export <out.tar>`.** Packs the CAS plus the project's
  lockfile entries into a portable archive.
- **`guroku store import <in.tar>`.** Hydrates the CAS from such an archive.
- **`--offline` flag for `install`.** Refuses any network call. Errors if
  the CAS doesn't already cover the lockfile.
- **CAS GC.** `guroku store gc` walks every project's lockfile under a
  configurable scan list and removes CAS entries no longer referenced.

### Current status

Not started.

---

## v1.5 — "It publishes"

### Theme

Closes the loop. Today guroku only consumes the registry; v1.5 lets you
publish to it.

### Features

- **`guroku publish`.** Produces an npm-compatible tarball, computes
  integrity, signs (provenance attestation, dist-tag setting), uploads
  to the configured registry.
- **2FA / OTP flows.** `--otp <code>` and an interactive prompt fallback.
- **Provenance.** Sigstore-backed `--provenance` flag matching npm's
  provenance format so guroku-published packages are interoperable.
- **`guroku version`.** Bumps the manifest version, optionally tags git,
  optionally pre-releases. Mirrors `npm version`.

### Current status

Not started.

---

## v1.6 — "Plugins"

### Theme

A stable extension point so third parties can add commands, registry
backends, or lifecycle hooks without forking. Pinned to v1.6 because
plugins need a stable enough core to extend; v1.0 froze the surface but
the resolver and linker have moved enough since then that v1.6 is the
realistic earliest.

### Features

- **WASM-component plugin host.** Plugins compile to WASM components and
  are loaded at startup. Capabilities are explicit (filesystem,
  registry-call, exec).
- **Plugin manifest in `.gurokurc.json`.** Per-project plugin enablement.
- **CLI extension points.** `guroku <plugin-name> <args>` routes to the
  plugin's exported handler.
- **Lifecycle hook extension points.** Plugins can subscribe to
  `pre-resolve`, `post-resolve`, `pre-link`, `post-link`, `pre-script`,
  `post-script`.

### Current status

Not started.

---

## Future / maybe

These are ideas we like but have not committed to. They live below the v1.0
line because we want to ship the core experience before we expand the
surface area, and because each of these is a project-sized chunk of work in
its own right.

- **Plugins.** A stable extension point so third parties can add commands,
  registry backends, or lifecycle hooks without forking. The shape of this
  is unclear; dynamic loading in Rust is awkward, and a JSON-RPC / WASM
  approach has its own tradeoffs. We will not start on this until we have
  concrete use cases that the core doesn't already cover.
- **Offline-mode store snapshots.** A way to pack the CAS plus a lockfile
  into a single archive that can be moved to an air-gapped machine and used
  to install without network. Useful for CI, reproducible builds, and the
  "build it on my laptop, ship it to a tin can" use case.
- **Deno / JSR registry support.** First-class resolution from
  `jsr.io`-style registries, treated as a peer of the npm registry rather
  than a wrapper around it. Depends on JSR's protocol staying stable enough
  to target.
- **`guroku publish`.** Today guroku only consumes the registry. Adding
  publish closes the loop, but it also pulls in OTP flows, provenance
  attestation, dist-tag management, and a long tail of edge cases. Worth
  doing, but not before v1.0.

---

## Non-goals

These are deliberate limits on the project's scope. They will not become
goals in a later milestone; if you want them, guroku is the wrong tool.

- **Not a runtime.** Guroku does not execute JavaScript. It installs files
  on disk and runs lifecycle scripts in the user's shell. Node, Deno, Bun,
  and the browser stay out of scope.
- **Not a bundler.** Guroku does not transform, tree-shake, or pack
  application code. The output of an install is a `node_modules` tree and a
  lockfile, full stop. Bundlers like Vite, esbuild, and Rollup are
  consumers of guroku, not features of it.
- **Not aiming for npm CLI parity.** We implement the npm commands and
  flags we believe are useful for modern workflows. We do not commit to
  matching every npm subcommand or every flag. If you need a command we
  haven't implemented, open an issue with the use case; we may add it, or
  we may point at an alternative.
- **Not chasing every historical edge case.** Some npm behaviors exist for
  packages published a decade ago and never updated. We will support what
  the modern ecosystem actually depends on. If a quirk only matters for an
  unmaintained 2014 package, we are likely to skip it and document the
  decision.

---

## How to influence the roadmap

The fastest way to make something on this list happen sooner — or to put
something new on the list — is to open a GitHub issue using the
`feature_request` template. Describe the use case, what you've tried, and
what success would look like. Concrete proposals beat abstract ones.

If you want to talk through an idea before filing an issue, GitHub
Discussions on the guroku repository is the right venue. Discussions are a
better fit for "I'm thinking about X, does it make sense?" and issues are a
better fit for "X is the plan, here is the spec." Either way, every change
to this roadmap starts as a public conversation.
