# embedding-rust

A worked example of consuming `guroku` as a Rust library from another binary
crate. If you are building a tool that needs npm-compatible dependency
resolution, this is the smallest realistic shape it can take.

## What this example shows

A small Rust binary that depends on guroku as a library. It:

1. Takes a project directory as a CLI argument.
2. Reads that project's `package.json` from disk.
3. Builds a `RegistryClient` pointed at the default public npm registry.
4. Calls `resolver::resolve` to walk the dependency graph.
5. Prints the resolved dependency tree to stdout.

The example deliberately stops before any installation work happens, so you
can see the resolver output without touching `node_modules` or the CAS.

## Files

- `Cargo.toml` --- declares a path dependency on the in-repo guroku.
- `src/main.rs` --- the runnable example.
- `README.md` --- this file.

## Build it

```sh
cd examples/embedding-rust
cargo build
```

The first build will compile guroku and its transitive dependencies, so
expect it to take a minute or two on a cold target directory. Subsequent
builds are incremental.

## Run it against a project

```sh
# against a real project that has package.json:
cargo run -- /path/to/your/project

# or against the in-repo sample:
cargo run -- ../sample-project
```

The argument is a path to a directory containing a `package.json`. The
example does not walk upward to find one --- pass the project root
explicitly.

## What you should see

```
project root: /path/to/your/project
project: my-app
declared 5 root packages
resolved 12 packages total:
  ms@2.1.3
  lodash@4.17.21
  ...
```

The exact package count depends on your project's manifest. If the resolver
fails (network error, version conflict, missing package on the registry),
you'll see the error chain instead, formatted by `anyhow`.

## What this example skips

The actual install pipeline. After `resolver::resolve`, you'd:

- Call `commands::install::install_from_resolution(&client, &resolution, &node_modules, &direct_dep_names).await?`
  to download and link.
- Or write your own loop using `commands::fetch_into_cas` and
  `linker::populate_node_modules`.

See `docs/embedding-guroku.md` for the full pipeline, including how to
configure the CAS location, customize the lockfile path, and hook into
lifecycle scripts.

## Library version

This example uses a path dependency so it always tracks the in-repo
sources. When you publish your own crate that depends on guroku, replace
the path dependency with a version pin:

```toml
[dependencies]
guroku = "1"
```

guroku follows semver across the 1.x line, so `"1"` will pick up patch and
minor releases automatically. If you need to lock to a specific patch,
write `"=1.0.3"` instead.

## Logging

guroku emits structured events through the `tracing` crate. The example
installs a default subscriber that reads its filter from the `GUROKU_LOG`
environment variable. Set it to see what the resolver is doing:

```sh
GUROKU_LOG=debug cargo run -- /path/to/your/project
```

Useful values:

- `info` --- one-line-per-phase progress.
- `debug` --- per-package fetch and resolution events.
- `trace` --- everything, including registry HTTP traffic. Very verbose.

You can also scope the filter to a single module, e.g.
`GUROKU_LOG=guroku::resolver=debug`.

## MSRV

Rust 1.75. The example inherits guroku's MSRV; if your own crate goes
lower, you'll need a newer guroku release that drops MSRV requirements
(see `docs/MSRV.md`). MSRV bumps land in minor releases on the 1.x line
and are always called out in the changelog.

## Related docs

- `docs/embedding-guroku.md` --- the long-form embedding guide, covering
  install, link, and audit flows.
- `docs/api-overview.md` --- a tour of the public types (`RegistryClient`,
  `Resolution`, `LockfileV1`, the `commands` module).
- `docs/STABILITY.md` --- which APIs are stable, which are
  `#[doc(hidden)]`, and which are gated behind unstable feature flags.
