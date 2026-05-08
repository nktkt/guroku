# Guroku API Overview (v1.0)

A user-facing tour of the public Rust API surface that ships with the
`guroku` crate. If you are integrating guroku into a build tool, a CI
runner, an editor extension, or building an alternative front-end on top
of the library, start here.

This document is distinct from `docs/internals/api-design.md`, which is
the contributor-facing design rationale describing why the API looks the
way it does. This page is the embedder's tour.

## 1. Audience

This guide is for Rust developers who want to drive guroku
programmatically. Typical embedders include:

- Build tools that need to install or verify dependencies as part of a
  larger pipeline.
- CI runners that want to inspect a lockfile, fetch packages, and
  report on the resolution graph without invoking the `guroku` binary.
- Alternative front-ends (TUIs, GUIs, language-server-style daemons)
  that need fine-grained control over fetch, resolve, and link steps.
- Tooling that consumes resolution data (auditors, license scanners,
  dependency visualisers) and wants the same view of the graph that
  the CLI sees.

If you only want to install dependencies from the command line, you
want `docs/cli-reference.md` and `docs/getting-started.md` instead.

## 2. The prelude

The canonical entry point for embedders is the prelude module. A single
glob import gives you the names you will reach for most often:

```rust
use guroku::prelude::*;
```

The prelude is intentionally small and stable. It exposes:

- `GurokuError` and `Result` for error handling.
- `Lockfile` and `PackageLock` for reading and writing `guroku.lock`.
- `Manifest` for reading a `package.json`.
- `RegistryClient` for talking to npm-compatible registries.
- `PackageMetadata` and `VersionInfo` as the shape returned by the
  registry client.
- `Resolution` and `Resolved` as the output of the resolver.
- `DepSpec` and `GitRef` for classified dependency specifications.
- `classify_spec`, `parse_range`, `parse_version`, `max_satisfying`
  as the small set of pure helpers you will reach for most often.
- `Range` and `Version` re-exported from `node-semver` so that callers
  do not have to depend on it explicitly.
- The constants `LOCKFILE_NAME`, `LOCKFILE_VERSION`, and
  `DEFAULT_REGISTRY`.

If you prefer not to glob, every item is also available at its
fully-qualified path under `guroku::`. The prelude is the supported
surface; items reachable only through deeper paths are best-effort and
may move between minor versions until promoted into the prelude.

## 3. Reading a manifest

A `Manifest` is an in-memory view of a `package.json`. Construct one
from a path:

```rust
use guroku::prelude::*;

let m = Manifest::read_from(std::path::Path::new("./package.json"))?;
for (name, spec) in m.all_dependencies() {
    println!("{name} -> {spec}");
}
```

`all_dependencies` yields the union of `dependencies`,
`devDependencies`, and `optionalDependencies` in a deterministic order.
If you need to distinguish those groups, the `Manifest` exposes
`dependencies()`, `dev_dependencies()`, and `optional_dependencies()`
accessors that each return a borrowed map. `peer_dependencies()` is also
available; peer entries are reported but not auto-installed, matching
npm semantics.

`Manifest::read_from` performs only the parsing and lightweight
validation that guroku itself relies on. It does not chase
`workspaces`; for that, see the workspace helpers under
`guroku::workspace`.

## 4. Building a registry client

`RegistryClient` is the type that knows how to talk to the registry.
Two constructors cover the common cases:

```rust
use guroku::prelude::*;

// Simplest: hit the public registry directly.
let client = RegistryClient::with_default_registry();

// Honour project + user .npmrc, including registry override and
// _authToken for private scopes.
let cwd = std::env::current_dir()?;
let client = RegistryClient::from_npmrc(&cwd)?;
```

`with_default_registry` is hard-coded to `DEFAULT_REGISTRY` and is
useful for tests, examples, and tools that explicitly do not want
ambient configuration.

`from_npmrc` walks up from `cwd` looking for a project `.npmrc`,
merges it with the user-level `.npmrc`, and applies the standard set
of keys that guroku understands: `registry`, scoped registry
overrides, and `_authToken` for both default and per-registry auth.
Unknown keys are ignored. See `docs/npmrc.md` for the precise list.

If you need to construct a client by hand, `RegistryClient::builder()`
exposes a builder where you can set the base URL, default headers, an
explicit auth token, and the underlying timeout.

## 5. Resolving

The resolver lives in `guroku::resolver`. The two entry points are:

```rust
use guroku::prelude::*;
use guroku::resolver;

let roots: Vec<(String, DepSpec)> = m
    .all_dependencies()
    .map(|(name, spec)| (name.to_string(), classify_spec(spec)))
    .collect();

let resolution = resolver::resolve(&client, &roots).await?;

for (name, resolved) in resolution.iter() {
    println!("{name}@{}", resolved.version());
}
```

`resolve` returns a `Resolution`, which is a flat, deterministic view
of the resolved graph. Iterate it with `resolution.iter()`, which
yields `(name, &Resolved)` pairs. `Resolved` exposes `version()`,
`integrity()`, `tarball()`, and the resolved dependency edges.

If you need overrides (analogous to npm's `overrides` field, or to
patch a transitive version for security reasons), use:

```rust
let overrides: Vec<(String, DepSpec)> = vec![/* ... */];
let resolution =
    resolver::resolve_with_overrides(&client, &roots, &overrides).await?;
```

Override entries are matched by name and applied transitively. See
`docs/overrides.md` for the matching rules.

The resolver is deterministic given the same inputs, the same
registry state, and the same client configuration. Two runs in the
same process will produce identical `Resolution` values.

## 6. Lockfiles

Lockfiles are read and written through `Lockfile`:

```rust
use guroku::prelude::*;

let path = std::path::Path::new(LOCKFILE_NAME);
let mut lock = Lockfile::read_from(path).unwrap_or_default();

lock.insert(
    "lodash".to_string(),
    "4.17.21".to_string(),
    PackageLock {
        resolved: "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz".into(),
        integrity: "sha512-...".into(),
        dependencies: Default::default(),
    },
);

lock.write_to(path)?;
```

`Lockfile::read_from` returns `Result<Lockfile>` and surfaces both IO
and parse errors as `GurokuError` variants. `write_to` writes
atomically: it stages the new content beside the target path and
renames into place, so partial writes cannot corrupt an existing
lockfile.

The on-disk format is documented in `docs/lockfile-format.md`. The
constants `LOCKFILE_NAME` and `LOCKFILE_VERSION` are the canonical
filename and the format version that this build of guroku writes.
When `LOCKFILE_VERSION` is incremented, older guroku binaries reject
the file rather than mis-reading it.

## 7. Dep specs

Dependency specifiers in `package.json` are not all the same shape.
`classify_spec` does the dispatch:

```rust
use guroku::prelude::*;

match classify_spec("^1.2.3") {
    DepSpec::Range(range) => {
        // Resolve from the registry against this semver range.
    }
    DepSpec::File(path) => {
        // Local path dependency, resolved relative to the manifest.
    }
    DepSpec::Git(GitRef { url, revision }) => {
        // Git dependency. `revision` is the resolved sha or ref.
    }
    _ => {
        // Forward-compatible: see Errors below for why this is needed.
    }
}
```

`classify_spec` is pure and synchronous. It does not touch the
network or the filesystem. `GitRef::url` is the upstream URL after
shorthand expansion (e.g. `github:user/repo` becomes a full HTTPS
URL); `GitRef::revision` is the requested ref, which may be a tag,
branch, or commit sha.

For file dependencies, see `docs/file-deps.md`. For git, see
`docs/git-deps.md`.

## 8. Versions

The version helpers wrap `node-semver` so that callers do not need to
take a direct dependency on it:

```rust
use guroku::prelude::*;

let range: Range = parse_range("^1.2.3")?;
let v: Version = parse_version("1.2.5")?;
assert!(range.satisfies(&v));

let candidates = vec![
    parse_version("1.2.3")?,
    parse_version("1.2.5")?,
    parse_version("2.0.0")?,
];
let best: Option<&Version> = max_satisfying(&candidates, &range);
assert_eq!(best, Some(&candidates[1]));
```

`Range` and `Version` are re-exports from `node-semver`. They are part
of guroku's public API: pinning `node-semver` is guroku's job, not
yours.

## 9. Errors

Every fallible function in the public API returns
`Result<T> = std::result::Result<T, GurokuError>`. There is no second
error type; embedders only need to handle `GurokuError`.

`GurokuError` is annotated `#[non_exhaustive]`. New variants may be
added in minor releases without breaking SemVer. As a result, every
pattern match on `GurokuError` must include a `_` arm:

```rust
use guroku::prelude::*;

fn explain(e: &GurokuError) -> &'static str {
    match e {
        GurokuError::Io(_) => "filesystem error",
        GurokuError::Network(_) => "network error",
        GurokuError::Manifest(_) => "package.json was invalid",
        GurokuError::Resolution(_) => "could not resolve a dependency",
        GurokuError::Integrity(_) => "tarball integrity check failed",
        _ => "unknown guroku error",
    }
}
```

The error type implements `std::error::Error`, `Display`, and `Debug`.
`source()` chains through to the underlying cause where one exists.
For machine-readable error codes (useful in CI), see
`docs/error-codes.md`.

## 10. Async

Anything that touches the network or runs potentially long IO is
async over Tokio:

- `RegistryClient::fetch_metadata`, `fetch_tarball`, and friends.
- `resolver::resolve` and `resolver::resolve_with_overrides`.
- The download helpers under `guroku::download`.

Anything that is purely CPU-bound is sync:

- `Manifest::read_from` (it does block on a file read, but the work
  itself is parsing and is fast).
- `classify_spec`.
- `parse_range`, `parse_version`, `max_satisfying`.
- All `Lockfile` reading and writing. Writing is atomic, and we
  considered making it async, but the file is small and the rename is
  cheap; staying sync keeps the API simple.

guroku does not pin a specific Tokio runtime flavour. A current-thread
runtime is sufficient; a multi-threaded runtime works equally well and
will give you concurrent fetches when the resolver explores siblings
of the graph in parallel.

## 11. A complete example

Putting it all together: read a manifest, build a registry client
from `.npmrc`, resolve, and print the resolved tree.

```rust
use guroku::prelude::*;
use guroku::resolver;

#[tokio::main]
async fn main() -> Result<()> {
    let manifest_path = std::path::Path::new("./package.json");
    let m = Manifest::read_from(manifest_path)?;

    let cwd = std::env::current_dir()?;
    let client = RegistryClient::from_npmrc(&cwd)?;

    let roots: Vec<(String, DepSpec)> = m
        .all_dependencies()
        .map(|(name, spec)| (name.to_string(), classify_spec(spec)))
        .collect();

    let resolution = resolver::resolve(&client, &roots).await?;

    println!("Resolved {} packages:", resolution.len());
    for (name, resolved) in resolution.iter() {
        println!("  {name}@{}", resolved.version());
    }

    Ok(())
}
```

To extend this example into a full install, write the resolution to
disk via `Lockfile::write_to` and feed the resolved tarball URLs into
`guroku::download`. A worked end-to-end version is in
`docs/embedding-guroku.md`.

## 12. Threading and Send/Sync

`RegistryClient` is `Send + Sync + Clone`. The underlying `reqwest`
client is held behind an `Arc`, so cloning a `RegistryClient` is cheap
and the clones share connection pools. Pass clones into spawned tasks
freely; do not wrap a `RegistryClient` in your own `Arc` or `Mutex`.

`Resolution` and `Resolved` are `Send + Sync`. You can return a
`Resolution` from a spawned task, share an `Arc<Resolution>` across
workers that each consume a slice of the graph, or stash one in
application state. `Resolution` is also `Clone`, although the clone
walks the internal map, so prefer `Arc<Resolution>` when sharing.

`Lockfile` is `Send` but not `Sync` while you are mutating it; treat
it like any other owned value. Once written, the on-disk file is the
shared source of truth.

`Manifest` and `DepSpec` are both `Send + Sync + Clone`.

In short: pass `RegistryClient` and `Resolution` across threads
freely, and treat everything else as ordinary owned data.

## 13. Where to go next

- `docs/embedding-guroku.md` walks through a complete embedding
  example end-to-end, including download, integrity verification,
  and on-disk layout.
- `docs/STABILITY.md` describes the SemVer commitment for the public
  API: what is covered, what is not, and how deprecations are
  signalled.
- `docs/lockfile-format.md`, `docs/npmrc.md`, `docs/overrides.md`,
  and `docs/error-codes.md` are the deep dives for the corresponding
  surfaces touched on above.
- The full rustdoc, including every item not listed in the prelude,
  will be published on crates.io once v1.0 ships. Until then, build
  it locally with `cargo doc --open` from a checkout of the guroku
  source tree.

If you find a hole in this guide, the public API, or the prelude,
please open an issue. The whole point of v1.0 is that this surface
becomes one we are willing to support.
