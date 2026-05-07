# guroku architecture

This document describes the internal organisation of `guroku`, a Rust-based,
npm-style package manager. It is aimed at contributors and at readers who want
to understand the codebase before changing it. It is deliberately concrete: it
describes what the v0.1 code actually does, not what an idealised package
manager should do. Where the implementation takes a short-cut, that short-cut
is named explicitly and pointed at the roadmap entry that supersedes it.

If you are looking for user-facing documentation, see `README.md`. If you are
looking for the public Rust API, see the re-exports in `src/lib.rs`.

## 1. Overview

`guroku` is a command-line package manager that consumes the public npm
registry and installs JavaScript packages into a `node_modules/` directory next
to a `package.json`. It is written in Rust, distributed as a single binary, and
exposes three subcommands in v0.1:

- `guroku install` — install everything declared in `package.json`.
- `guroku add <pkgs...>` — add new dependencies to `package.json` and install
  them.
- `guroku remove <pkgs...>` — drop dependencies from `package.json` and unlink
  them from `node_modules`.

v0.1 is intentionally a thin slice. It does the following, and only the
following:

- Parses `package.json` into a `Manifest`.
- Talks to `https://registry.npmjs.org` over HTTPS to fetch package metadata.
- Picks a version using a trivial rule (exact version, or "latest" dist-tag).
  There is **no semver resolver**, no peer-dependency handling, and no
  deduplication beyond what the trivial rule produces.
- Downloads the `.tgz` tarball, verifies its SHA-512 against
  `dist.integrity`, and extracts it into a flat per-package store under
  `~/.guroku/store/<name>/<version>/`.
- Copies that directory into `node_modules/<name>/`. This is a real filesystem
  copy, not a hardlink and not a symlink.
- Does **not** write a lockfile. Reproducibility in v0.1 is bounded by what
  the registry returns at install time.
- Does **not** execute lifecycle scripts (`preinstall`, `postinstall`, etc.).
  Tarballs are extracted and that is all.

The code is structured so that the parts that will change in v0.2 and v0.3
(resolution, storage layout, linking strategy) are isolated behind small
modules. The shape of the install pipeline should survive those changes; only
the implementations behind `registry`, `cache`, and `linker` should need
substantial rework.

## 2. Crate layout

The crate is a single binary with a thin library crate alongside it. The
binary is `guroku`; the library exists so that integration tests in `tests/`
and any future embedders can call into the same code paths the CLI uses.

```
src/
  main.rs        — binary entry point; parses CLI, dispatches to commands
  lib.rs         — public re-exports
  cli.rs         — clap definitions for the `guroku` CLI (install / add / remove)
  error.rs       — `GurokuError` (thiserror) and crate-wide `Result`
  logging.rs     — tracing init from `GUROKU_LOG` env var
  manifest.rs    — `package.json` parser/writer (`Manifest`)
  registry.rs    — npm registry HTTP client and `PackageMetadata` / `VersionInfo` / `Dist`
  tarball.rs     — `.tgz` extraction (strips leading `package/` segment, rejects `..`)
  integrity.rs   — SHA-512 verification of `dist.integrity`
  linker.rs      — naive flat copy of a store package into `node_modules/<name>` (v0.1)
  cache.rs       — `~/.guroku` paths: store, tarball cache
  commands/
    mod.rs       — shared `install_one` / `parse_spec` helpers
    install.rs   — `guroku install`
    add.rs       — `guroku add <pkgs...>`
    remove.rs    — `guroku remove <pkgs...>`
```

`main.rs` is intentionally tiny. It calls `logging::init`, parses `cli::Cli`
with `clap`, and matches on the subcommand to dispatch into one of the
functions in `commands/`. It is also where the process exit code is decided: a
returned `GurokuError` is printed as `guroku: <error>` to stderr and the
process exits with `1`.

`lib.rs` re-exports the items that integration tests and external callers are
allowed to depend on. Anything not re-exported from `lib.rs` should be
considered an internal implementation detail and may move between modules
without notice.

`cli.rs` owns the `clap` derive structs. It does not implement any business
logic; its only job is to turn `argv` into a typed `Cli` value. Keeping `clap`
out of the rest of the crate means subcommand functions take plain Rust
arguments and are easy to call from tests.

`error.rs` defines `GurokuError`, a `thiserror`-derived enum, and a
`Result<T> = std::result::Result<T, GurokuError>` alias used throughout the
crate. See section 6 for the error model.

`logging.rs` initialises `tracing_subscriber` from the `GUROKU_LOG` environment
variable, which is parsed as an `EnvFilter` directive (so values like
`info`, `guroku=debug`, or `guroku::registry=trace` all work). If
`GUROKU_LOG` is unset, the default level is `warn`.

`manifest.rs` parses and re-serialises `package.json`. The `Manifest` struct
carries the fields guroku actually reads (`name`, `version`, `dependencies`,
`devDependencies`) and preserves the rest of the document via
`serde_json::Value` so that `add`/`remove` round-trip cleanly without
reformatting unrelated fields.

`registry.rs` is the HTTP client for the npm registry. It exposes
`PackageMetadata`, `VersionInfo`, and `Dist`, which mirror the subset of the
registry's JSON that guroku consumes: the per-package "packument"
(`/{name}`), the per-version metadata inside it, and the `dist` object that
carries the tarball URL and integrity hash. There is no auth and no scoped
registry support in v0.1.

`tarball.rs` extracts npm `.tgz` archives. npm's convention is that every
entry inside the tarball is prefixed with `package/`; this module strips that
prefix when writing files and rejects any entry whose normalised path tries to
escape the destination (`..` segments, absolute paths, symlinks pointing
outside the destination).

`integrity.rs` verifies tarballs against the SRI string in `dist.integrity`.
v0.1 only understands `sha512-<base64>`; anything else is a hard error. The
verifier is fed the raw tarball bytes — guroku always holds the whole tarball
in memory before extracting (see section 5).

`linker.rs` is the boundary between "package is in the store" and "package is
visible to Node". In v0.1 this is a naive recursive copy from
`~/.guroku/store/<name>/<version>/` into `<project>/node_modules/<name>/`.
This is the module that will be replaced wholesale in v0.3.

`cache.rs` centralises the on-disk paths guroku owns: `~/.guroku/` as the
root, `~/.guroku/store/` for extracted packages, and `~/.guroku/tarballs/`
for cached `.tgz` blobs keyed by integrity hash. Nothing else in the crate is
allowed to construct these paths — if you need a path under `~/.guroku`, ask
`cache`.

`commands/mod.rs` holds the helpers shared between subcommands. `install_one`
is the per-package pipeline (metadata → tarball → verify → extract → link)
called by both `install` and `add`. `parse_spec` turns a CLI argument like
`react@18.2.0` or `lodash` into a `(name, version_spec)` pair.

`commands/install.rs`, `commands/add.rs`, and `commands/remove.rs` are the
top-level subcommand entry points. They read the manifest, mutate it where
appropriate, drive the install pipeline, and write the manifest back.

## 3. Install pipeline

The install pipeline is the same for `install` and `add`; they differ only in
how the set of packages-to-install is computed. `install` takes the union of
`dependencies` and `devDependencies` from the manifest. `add` takes the
command-line arguments, parses them with `parse_spec`, and merges them into
the manifest before falling through to the same pipeline.

For each package to install, `install_one` runs the following sequence:

```
                       package.json
                            |
                            v
                   1. read manifest
                            |
                            v
              2. fetch packument from registry
                            |
                            v
                3. resolve(spec) -> version
                            |
                            v
                4. fetch tarball (cached?)
                  +---------+----------+
                  |                    |
            yes (in cache)        no (download)
                  |                    |
                  +---------+----------+
                            |
                            v
              5. verify SHA-512 vs dist.integrity
                            |
                            v
              6. extract tarball into store
                  ~/.guroku/store/<name>/<version>/
                            |
                            v
              7. link store -> node_modules/<name>
                            |
                            v
                   8. write manifest
```

Step by step:

1. **Read manifest.** `manifest::Manifest::load(path)` parses
   `package.json`. Failure here aborts the whole command with a
   `GurokuError::Manifest` carrying the path.
2. **Fetch packument.** `registry::client.get_metadata(name)` issues a single
   `GET /{name}` and deserialises into `PackageMetadata`. The response is not
   cached on disk in v0.1.
3. **Resolve.** v0.1's resolver is a one-liner: if the spec is a dist-tag (or
   missing, in which case it defaults to `latest`), look it up in
   `metadata.dist_tags`; otherwise treat the spec as an exact version and
   look it up in `metadata.versions`. Anything else — ranges, `^`, `~`, git
   URLs, file paths, scoped overrides — is rejected. The real resolver
   arrives in v0.2.
4. **Fetch tarball.** `cache::tarball_path(integrity)` is checked first. On a
   miss, `registry::client.fetch_tarball(dist.tarball)` downloads the bytes
   into memory and writes them to the cache atomically (write to a temp
   file in the same directory, then rename).
5. **Verify integrity.** `integrity::verify(bytes, &dist.integrity)` recomputes
   SHA-512 and compares against the SRI value. A mismatch deletes the
   cached tarball and returns `GurokuError::Integrity`.
6. **Extract.** `tarball::extract(bytes, store_dir)` streams a `GzDecoder`
   into a `tar::Archive`, stripping the leading `package/` segment and
   rejecting any unsafe path. The destination is the store path; extraction
   is staged in a sibling temp directory and renamed into place so a crashed
   extract leaves no half-populated store entry.
7. **Link.** `linker::link(store_dir, node_modules_dir, name)` copies the
   store directory into `node_modules/<name>`. If `node_modules/<name>`
   already exists, it is removed first.
8. **Write manifest.** Only `add` and `remove` reach this step;
   `install` is a no-op for the manifest.

## 4. Storage model (current vs. target)

The on-disk layout under `~/.guroku/` in v0.1 is:

```
~/.guroku/
  store/
    <name>/
      <version>/
        package.json
        ...the package's files...
  tarballs/
    <integrity>.tgz
```

Each `<name>/<version>/` directory is a complete, self-contained copy of one
package. `node_modules/<name>/` is then a byte-for-byte copy of that
directory. Two projects that depend on the same `lodash@4.17.21` will
extract once into the store, and then each project will hold its own full
copy inside its own `node_modules/`.

This is, intentionally, the dumbest layout that works. It is easy to reason
about, easy to debug (`ls` shows real files), and easy to throw away
(`rm -rf node_modules` and `rm -rf ~/.guroku` are both safe). It is also
wasteful: disk usage is `O(projects x packages)` instead of the
`O(packages)` that pnpm-style stores achieve.

The target layout, planned for v0.3, is a content-addressable store with
hardlinks and a pnpm-style isolated `node_modules/`:

```
~/.guroku/
  cas/
    <sha512-prefix>/<sha512>/    # one entry per file, by content hash
  store/
    <name>/<version>/            # tree of hardlinks into cas/
  tarballs/
    <integrity>.tgz

<project>/node_modules/
  .guroku/<name>@<version>/node_modules/<name>/   # hardlinks into store/
  <name> -> .guroku/<name>@<version>/node_modules/<name>   # symlink
```

In that model, identical files are stored exactly once on disk per filesystem,
and projects pay only the cost of an inode per file rather than a full copy.
Until that lands, the v0.1 copy-linker is a known short-cut, not a design
decision we intend to defend.

The `linker.rs` module is the only place that needs to change to switch
between these two models. The pipeline in section 3 does not care how
linking is implemented.

## 5. Concurrency

guroku runs on a Tokio multi-thread runtime, entered via `#[tokio::main]` in
`main.rs`. The runtime is shared across all subcommands; there is no
single-threaded fallback.

Inside `commands::install`, the set of packages to install is turned into a
`futures::stream` of futures, each of which is a call to `install_one`.
Those futures are driven through `StreamExt::buffer_unordered(N)`, where
`N` is a small concurrency knob (currently a constant in `commands/mod.rs`,
sized for typical home-internet links rather than CI machines). This gives
us bounded parallelism without a thread pool: at most `N` tarballs are
in-flight at once, and the rest queue up behind them.

Each tarball is treated as a single atomic blob. We download the entire
`.tgz` into a `Vec<u8>` before doing anything with it, then verify and
extract from that buffer. This costs memory proportional to the largest
tarball but makes integrity verification trivial (hash the buffer once) and
makes the extraction step a pure function of bytes-in to files-out, with no
partial-download recovery to worry about. Tarballs in the npm ecosystem are
small enough that this is not a real constraint.

There is no shared mutable state between concurrent `install_one` calls.
The store is laid out so that each call writes to its own
`<name>/<version>/` directory, and the rename-into-place pattern means two
processes installing the same package race harmlessly: whichever one renames
last wins, and the loser's temp directory is cleaned up.

`add` and `remove` mutate `package.json` once, after the install pipeline
has finished, on the main task. They are not concurrent with each other,
and the CLI does not attempt to lock the manifest against another `guroku`
process running in the same directory.

## 6. Error model

All fallible functions in the crate return `crate::Result<T>`, which is
`std::result::Result<T, GurokuError>`. `GurokuError` is a single
`thiserror`-derived enum with one variant per failure domain: manifest I/O,
registry HTTP, integrity mismatch, tarball extraction, linker I/O, and a
generic `Io` for everything else.

The rule for I/O errors is that the variant carries the path that was being
operated on, not just the underlying `io::Error`. A read failure on
`./package.json` should produce a message like
`failed to read manifest at ./package.json: No such file or directory`,
not the bare `No such file or directory` that `io::Error` would print on
its own. This is the single most useful thing the error model does and it
is worth the small amount of boilerplate at every I/O call site.

`main.rs` catches the top-level `GurokuError` and prints it to stderr in
the form:

```
guroku: <error>
```

followed by `Caused by:` lines for each link in the source chain (via the
`Display` impl, walked through `error.source()`). The process then exits
with status `1`. There is no panic-to-error translation; a panic is a bug
and is allowed to abort.

`tracing` is used for diagnostic output, not for user-facing errors. If a
user wants to see what guroku is doing — every HTTP request, every cache
hit, every extract — they set `GUROKU_LOG=guroku=debug` (or `=trace`) and
read stderr. The default level is `warn`, so a successful install is
silent.

## 7. Roadmap pointer

The roadmap lives in `README.md` and is the source of truth for what comes
next. In short:

- **v0.2 — resolver.** Replace the one-liner in step 3 of the pipeline
  with a real semver resolver. Introduce a lockfile (`guroku.lock`) so
  that resolution is reproducible and so that `install` on a clean
  checkout does not re-query the registry for every package. `package.json`
  range syntax (`^`, `~`, ranges, `||`) starts working in this release.
- **v0.3 — content-addressable store and hardlinks.** Replace the copy-
  based `linker.rs` with a hardlink-based linker over a CAS, and adopt
  a pnpm-style isolated `node_modules/` layout. This is where guroku
  stops being wasteful on disk and starts being competitive with pnpm
  on cold-install time.
- **v0.4 — scripts and workspaces.** Run lifecycle scripts
  (`preinstall`, `postinstall`, `prepare`) in a sandboxed shell, and
  support npm-style workspaces (`workspaces` field in the root
  `package.json`, hoisted installs across packages).

Each milestone is shaped so that the modules touched are the ones described
in section 2. v0.2 lands in `registry.rs`, a new `resolver.rs`, and
`commands/mod.rs`. v0.3 lands in `linker.rs` and `cache.rs`. v0.4 lands in
new modules (`scripts.rs`, `workspaces.rs`) and in the command layer.

## 8. Non-goals

guroku is a package manager. It is not, and is not trying to become, any of
the following:

- **A JavaScript runtime.** guroku does not execute JavaScript. It produces
  a `node_modules/` directory; running it is Node's job (or Bun's, or
  Deno's).
- **A bundler.** guroku does not transform, tree-shake, minify, or
  otherwise touch the contents of the packages it installs. What the
  registry served is what ends up on disk.
- **A drop-in npm CLI.** guroku does not aim for command-for-command,
  flag-for-flag parity with `npm`. It implements the subset of behaviour
  that the roadmap calls out and rejects the rest. If a script in your
  repository calls `npm install --legacy-peer-deps --foo --bar`, guroku
  will not understand it and will not pretend to.
- **A registry.** guroku is a client. Hosting packages, mirroring the
  registry, and proxying are out of scope.

These non-goals exist so that the scope stays small enough for the v0.1
codebase to actually do what it says it does.

## 9. Glossary

- **manifest** — a `package.json` file. In code, the parsed form is
  `manifest::Manifest`. The on-disk form is JSON; the in-memory form
  preserves unknown fields so that round-tripping does not lose data.
- **dist** — the `dist` object inside a registry version entry. Carries
  the tarball URL (`dist.tarball`), the integrity hash
  (`dist.integrity`), and a few other fields guroku ignores.
- **integrity** — an SRI-style string (`sha512-<base64>`) that the
  registry publishes for each tarball. guroku verifies every tarball
  against this string before extracting.
- **tarball** — the gzipped tar archive (`.tgz`) that the registry serves
  for a given package version. Inside, every entry is prefixed with
  `package/`; `tarball.rs` strips that prefix on extract.
- **store** — the directory under `~/.guroku/store/` that holds extracted
  packages. In v0.1 this is a flat `<name>/<version>/` layout; in v0.3
  it becomes a tree of hardlinks into a content-addressable store.
- **linker** — the component that makes a store entry visible to Node by
  populating `node_modules/<name>/`. In v0.1 this is a recursive
  filesystem copy; in v0.3 it becomes a hardlink-and-symlink construction
  over a pnpm-style isolated layout.
