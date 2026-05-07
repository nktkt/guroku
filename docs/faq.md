# Frequently Asked Questions

This page collects short answers to questions that come up often. For
deeper treatment, follow the links into the rest of the docs.

If your question is not answered here, please open an issue (see
[Contributing](#contributing)).

---

## Choosing guroku

### Q: How does guroku compare with npm, pnpm, bun, and yarn?

guroku borrows ideas from each. Like **pnpm**, it lays out
`node_modules` strictly so packages cannot reach undeclared
dependencies (see [strict-layout](internals/strict-layout.md)). Like
**pnpm** and **bun**, it stores package contents in a content-addressed
store (CAS) and links them into projects (see
[storage.md](storage.md)). Like **bun**, it is written in Rust for
speed and a small dependency footprint. Unlike all four, guroku is
pre-1.0 software with deliberately narrow scope: it is a package
manager, not a runtime, not a workspace orchestrator (yet), and not a
publishing tool.

### Q: Can I use guroku in production?

No. guroku is pre-alpha. The on-disk format, the lockfile schema, and
the CLI surface may all change without warning before v1.0. Use it for
experiments, learning, and small side projects only.

### Q: Why "guroku"?

It is just a name. It does not stand for anything and it does not
have a clever backronym. Pronounce it however feels natural.

### Q: Is guroku a runtime?

No. guroku installs JavaScript packages so that Node.js (or any
Node-compatible runtime) can find them. Running a runtime is an
explicit non-goal; see [ROADMAP.md](../ROADMAP.md) for the full
non-goals list.

---

## Installation and basic use

### Q: How do I install guroku?

For v0.3, build it from source with cargo:

```
git clone https://github.com/nktkt/guroku
cd guroku
cargo install --path .
```

Pre-built binaries and a Homebrew formula will arrive once the CLI is
more stable. The full walk-through lives in
[getting-started.md](getting-started.md).

### Q: Does it read my existing `package-lock.json`, `pnpm-lock.yaml`, or `yarn.lock`?

No. guroku only reads its own [`guroku.lock`](lockfile-format.md). An
importer that converts other lockfiles is on the roadmap, but it is
not in v0.3. For now, run `guroku install` against your `package.json`
and let guroku produce a fresh lockfile.

### Q: Should I commit `guroku.lock`?

Yes. Always commit it alongside `package.json`. The lockfile is the
sole source of truth for reproducible installs and includes content
hashes for every resolved dependency. See
[lockfile-format.md](lockfile-format.md) for the schema.

### Q: What about `node_modules` — should I commit it?

No, never. `node_modules` is a build artifact derived from
`package.json` and `guroku.lock`. Add it to your `.gitignore`.

---

## Disk and performance

### Q: How big is `~/.guroku/cas`?

It depends on how many distinct package versions you have installed
across all projects. A laptop that touches a dozen mid-size projects
will typically see a few hundred megabytes. Inspect the current size
with:

```
du -sh ~/.guroku/cas
```

For a breakdown by package and tips on reclaiming space, see
[storage.md](storage.md) (the "disk usage" section).

### Q: Can I share `~/.guroku/cas` across users on the same machine?

Not in v0.3. Permissions and concurrent-write safety for a shared
store are not yet in place. Each user account gets its own CAS under
`$HOME/.guroku/cas`.

### Q: Why is the first install slow?

The first install is a cold cache: every tarball must be downloaded,
verified, and extracted into the CAS. Subsequent installs of the same
versions hit both the on-disk CAS and the registry-metadata ETag
cache, so they finish in a small fraction of the time.

### Q: Can I move `~/.guroku` to another drive?

Not configurably in v0.3. The path is hard-coded relative to the
user's home directory. A symlink at `~/.guroku` pointing to your
target drive will work as a workaround, but it is unsupported. A
proper `--store-dir` flag and config key are planned for v0.4.

---

## Strict layout

### Q: Why is `node_modules/<name>` a symlink?

guroku uses a strict layout: each direct dependency is a symlink that
points into `~/.guroku/cas`, and transitive dependencies live under
`node_modules/.guroku/`. This prevents packages from importing
dependencies they did not declare. Read
[strict-layout.md](internals/strict-layout.md) for the diagram and
rationale.

### Q: Will my code break if `node_modules` is symlinked?

Almost certainly not. Node follows symlinks transparently when
resolving modules, and every modern bundler (webpack, esbuild, vite,
rollup, parcel) does the same. The handful of tools that have
historically struggled with symlinks usually expose a "preserve
symlinks" flag; turn it off if it is on.

### Q: Does `require.resolve` still work?

Yes. `require.resolve` walks the same symlink-aware path as
`require`, so it returns the resolved real path inside
`~/.guroku/cas`. If you depend on the resolved string for tooling,
verify it does not assume `node_modules/<pkg>` is a real directory.

---

## Resolver

### Q: Does guroku do PubGrub?

Not in v0.3. The current resolver is a straightforward depth-first
walk with first-resolved-wins semantics. PubGrub-style backtracking is
a candidate for v0.4 or later; see
[algorithm-notes.md](internals/algorithm-notes.md) for the design
sketch.

### Q: How does it handle diamond dependencies?

The first version that satisfies a constraint wins, and any later
incompatibility is reported as a hard error rather than silently
duplicated. This keeps the install graph predictable at the cost of
sometimes failing where npm would quietly nest two copies. The full
algorithm is documented in
[algorithm-notes.md](internals/algorithm-notes.md).

### Q: Are peer dependencies installed?

No. In v0.3, peer dependencies are read from `package.json`,
validated against the resolved graph, and warned about when missing,
but they are never auto-installed. See
[peer-dependencies.md](peer-dependencies.md) for the exact rules and
warning levels.

---

## Errors and debugging

### Q: How do I get debug logs?

Set the `GUROKU_LOG` environment variable:

```
GUROKU_LOG=debug guroku install
```

Valid levels are `error`, `warn`, `info`, `debug`, and `trace`.
`trace` is extremely verbose and is intended for resolver and CAS
debugging. The full list of error codes is in
[error-codes.md](error-codes.md).

### Q: Where are guroku's config files?

There are none in v0.3. Every option is either a CLI flag or an
environment variable. `.npmrc` parsing (registry URLs, auth tokens,
scopes) is on the v0.4 list.

---

## Networking

### Q: Can I use a private registry?

Not yet. v0.3 always talks to `https://registry.npmjs.org`. Private
registry support, per-scope registries, and `_authToken` handling are
scheduled for v0.5.

### Q: Does guroku respect `HTTP_PROXY` and `HTTPS_PROXY`?

It uses `reqwest`'s default proxy detection, which reads
`HTTP_PROXY`, `HTTPS_PROXY`, and `NO_PROXY` from the environment.
This works in most setups, but it is not part of guroku's test matrix
yet, so treat it as best-effort.

### Q: Is there an offline mode?

Partial. Once a package version is in the CAS and its registry
metadata is in the ETag cache, guroku can install it without network
access. There is no explicit `--offline` flag in v0.3; an attempt to
resolve something not yet cached will simply fail with a network
error.

---

## Roadmap

### Q: When will v0.4 ship?

When it is done. guroku is a one-person side project with no release
calendar. The current scope of v0.4 (lockfile importer, workspaces,
lifecycle scripts, `.npmrc`) is tracked in
[ROADMAP.md](../ROADMAP.md).

### Q: When will lifecycle scripts (`preinstall`, `postinstall`, etc.) work?

v0.4. Sandboxing and the script-allowlist design are still being
worked out, which is the main thing gating the milestone.

### Q: When will workspaces work?

v0.4. The lockfile already has a slot for workspace members; what is
missing is the resolver pass that hoists shared dependencies and the
CLI surface for running scripts across packages.

---

## Contributing

### Q: How do I run the tests?

```
cargo test
```

There are unit tests next to each module and integration tests under
`tests/`. The integration tests spin up a local fake registry, so
they do not need network access. The full contributor workflow,
including formatter and clippy expectations, lives in
[CONTRIBUTING.md](../CONTRIBUTING.md).

### Q: Where do I file an issue?

At <https://github.com/nktkt/guroku/issues>. Please use the bug
report template; it asks for the guroku version, the host OS, the
contents of `guroku.lock` (or a minimal reproduction), and the output
of the failing command with `GUROKU_LOG=debug`.
