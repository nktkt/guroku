# guroku CLI Reference

`guroku` is an npm-style package manager written in Rust. This document
describes the current command-line surface: every flag, every subcommand,
their inputs and outputs, and the limitations you should expect at this
stage of the project.

> **v0.2 changes:** `install` gained a `--frozen-lockfile` flag for CI;
> range specs (`^1.2.3`, `~1.0`, `1.x`, `^1 || ^2`) now resolve correctly
> instead of silently falling back to `latest`; every install reads and
> writes `guroku.lock`. See `docs/migration/v0.1-to-v0.2.md` if you're
> upgrading from v0.1.

## Synopsis

```
guroku [OPTIONS] [COMMAND]
```

If `COMMAND` is omitted, `guroku` runs `install` against the project in
the current working directory (or whatever `--cwd` points at).

## Global options

| Flag | Description |
| ---- | ----------- |
| `-C, --cwd <CWD>` | Path to the project directory. Defaults to the current working directory. All path resolution (`package.json`, `node_modules`, etc.) is performed relative to this value. |
| `-h, --help` | Print help for `guroku` or for a specific subcommand and exit. |
| `-V, --version` | Print the binary version and exit. |

## Environment

| Variable | Purpose |
| -------- | ------- |
| `GUROKU_LOG` | Controls log verbosity. Accepts the standard `tracing`/`env_logger` syntax (`error`, `warn`, `info`, `debug`, `trace`, or per-target filters such as `guroku::resolver=debug`). |
| `RUST_LOG` | Used as a fallback when `GUROKU_LOG` is not set. |
| (default) | If neither variable is set, `guroku` logs at `info`. |

## Filesystem layout

```
<project>/package.json        # the manifest guroku reads and writes
<project>/node_modules/       # install target
~/.guroku/store/<name>/<version>/   # global content cache
```

The cache is shared across projects on the same machine. v0.1 always
copies from the cache into `node_modules`; see "Known limitations" below.

## Exit codes

| Code | Meaning |
| ---- | ------- |
| `0`  | Success. |
| `1`  | Runtime error (network failure, manifest parse error, I/O error, registry returned an unexpected response, etc.). |
| `2`  | CLI usage error (unknown flag, missing required argument, malformed package spec at the argv layer). Produced by clap before any work is done. |

---

## `guroku install`

### Synopsis

```
guroku [-C <CWD>] install
guroku [-C <CWD>] i
guroku [-C <CWD>]
```

### Description

Reads `package.json` in the project directory and installs every entry
under `dependencies` into `node_modules`. This is the default subcommand:
running `guroku` with no arguments is equivalent to `guroku install`.

`install` is idempotent in v0.1: it always re-resolves the versions
listed in `package.json`, fetches anything missing from the cache, and
re-populates `node_modules`. There is no lockfile, so two runs separated
by a registry update may produce different trees.

### Examples

Install in the current directory:

```sh
guroku install
```

The short alias:

```sh
guroku i
```

Install for a project that lives somewhere else:

```sh
guroku -C ./packages/web install
```

Run with debug logging:

```sh
GUROKU_LOG=debug guroku install
```

### Exit codes

- `0` — every dependency installed.
- `1` — a dependency could not be fetched, the manifest is invalid, or `node_modules` could not be written.
- `2` — invalid CLI invocation.

### Notes

- `install` only reads the top-level `dependencies` field. `devDependencies`, `peerDependencies`, and `optionalDependencies` are ignored in v0.1.
- The transitive graph is walked, but resolution is naive (see "Known limitations").

---

## `guroku add`

### Synopsis

```
guroku [-C <CWD>] add <PACKAGES>...
```

### Description

Adds one or more packages to the `dependencies` field of `package.json`
and installs them. `package.json` is rewritten in place after a
successful resolve. If any package in the list fails to resolve or
fetch, no changes are persisted.

Each `<PACKAGES>` argument is a package spec. The accepted forms are:

| Spec | Example | Behavior in v0.1 |
| ---- | ------- | ---------------- |
| `name` | `lodash` | Installs `latest`. |
| `name@version` | `lodash@4.17.21` | Installs that exact version if it exists in the registry. |
| `name@latest` | `lodash@latest` | Installs `latest`. |
| `@scope/name` | `@types/node` | Installs `latest`. |
| `@scope/name@version` | `@types/node@20.11.0` | Installs that exact version. |
| Anything else (`^1.2.3`, `~1.2`, `>=1`, tag names other than `latest`, git URLs, tarballs, file paths) | `lodash@^4` | Falls back to `latest` and emits a warning. |

The recorded version in `package.json` is the exact version that was
installed, written without a range specifier (e.g. `"lodash": "4.17.21"`).

### Examples

Add a single package at `latest`:

```sh
guroku add lodash
```

Pin an exact version:

```sh
guroku add lodash@4.17.21
```

Add a scoped package:

```sh
guroku add @types/node@20.11.0
```

Add several at once:

```sh
guroku add react react-dom@18.2.0 @types/react
```

A spec that v0.1 does not understand still works, but is downgraded:

```sh
guroku add 'lodash@^4'
# warning: range "^4" not supported in v0.1, falling back to latest
```

### Exit codes

- `0` — all packages added and installed; `package.json` updated.
- `1` — at least one package failed to resolve or install. `package.json` is left untouched.
- `2` — no package specs were supplied, or a spec was syntactically invalid at the argv layer.

### Notes

- Adding a package that is already listed in `dependencies` overwrites the existing entry with the newly resolved version.
- The on-disk `package.json` is rewritten with 2-space indentation and a trailing newline. Other formatting (key ordering of unrelated fields, comments in JSON5, etc.) is not preserved beyond what `serde_json` round-trips.

---

## `guroku remove`

### Synopsis

```
guroku [-C <CWD>] remove <PACKAGES>...
guroku [-C <CWD>] rm     <PACKAGES>...
```

### Description

Removes one or more packages from `dependencies` in `package.json` and
deletes the corresponding directories from `node_modules`. The cache at
`~/.guroku/store/` is not touched, so re-adding the same version later
does not require a new download.

### Examples

Remove a single package:

```sh
guroku remove lodash
```

Using the alias:

```sh
guroku rm lodash
```

Remove several at once, including a scoped package:

```sh
guroku remove react react-dom @types/react
```

### Exit codes

- `0` — every named package was removed (or was already absent).
- `1` — `package.json` could not be read or written, or `node_modules` could not be modified.
- `2` — no package names were supplied.

### Notes

- Packages that are not present in `dependencies` are skipped silently. They are not an error.
- `remove` does not prune transitive dependencies that were only kept alive by the removed package. Run `guroku install` afterwards if you want a clean tree.
- Only the top-level entries in `node_modules/<name>` and `node_modules/@scope/<name>` are removed. Nested `node_modules` directories belonging to other packages are left alone.

---

## Known limitations in v0.1

These are tracked in the CHANGELOG and will be addressed in later
releases. They are listed here so that the surface above is not
mistaken for the eventual feature set.

- **No semver resolver.** Version specs other than an exact version or `latest` are downgraded to `latest`. Ranges (`^1.2.3`, `~1.2`, `>=1 <2`) and dist-tags other than `latest` are not honored.
- **No lockfile.** There is no `guroku.lock` (or equivalent). Two runs of `install` against the same `package.json` may install different transitive versions if the registry has moved in between.
- **No hardlinks or symlinks.** Packages are copied from the global cache into each project's `node_modules`. Disk usage scales linearly with the number of projects.
- **No lifecycle scripts.** `preinstall`, `install`, `postinstall`, and any other script hooks declared in `package.json` are not executed. Native modules that rely on a build step will not work out of the box.

## See also

- `CHANGELOG.md` for the per-release feature list.
- `README.md` for a quick-start guide and the project roadmap.
- `CONTRIBUTING.md` for how to file bugs against any of the limitations above.
