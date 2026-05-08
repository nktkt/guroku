# Embedding guroku

This guide is for developers who want to use guroku as a Rust library inside
their own tool, rather than shelling out to the `guroku` binary. It walks
through a realistic embedding scenario: from adding the dependency, through
resolving and installing, to wiring up overrides, lockfiles, and custom
registries.

guroku is published as a regular crate on crates.io. The CLI you run from a
shell is a thin wrapper over the same public API documented here.

---

## 1. Motivation

Why embed guroku instead of running the binary?

- **You're a build tool that wants to install JS deps as part of a larger
  pipeline.** Maybe you bundle a frontend and a backend, or you produce
  container images that need `node_modules` populated before a `webpack`
  pass. Embedding lets you treat dependency installation as just another
  step in your tool's normal flow, with the same logging, the same progress
  reporting, and the same error model as the rest of your code.

- **You're writing a CI helper and want to avoid spawning the binary.**
  Spawning incurs `tokio::process::Command` overhead, complicates error
  capture (you have to scrape stderr), and forces you to ship a second
  binary alongside your tool. By linking guroku as a library, you get
  structured errors, no process boundary, and one statically-linked
  artifact.

- **You want to drive different parts of the pipeline independently.** A
  common case is "resolve here, but defer the actual fetch to my own
  bandwidth-limited downloader." Embedding lets you call `resolver::resolve`
  directly, persist or serialize the resolution, and then either feed it
  back into `commands::install::install_from_resolution` later, or take the
  raw tarball URLs and fetch them yourself with your own concurrency
  governor.

If none of those apply, the CLI is probably fine. Embedding is for when you
want guroku to be a feature of your tool rather than a sibling.

---

## 2. Add as a dependency

In your `Cargo.toml`:

```toml
[dependencies]
guroku = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

guroku's public API is async, so you need a Tokio runtime. The
`macros` feature gives you `#[tokio::main]`; `rt-multi-thread` is
recommended because resolution and fetching fan out across many tasks and
benefit from a real thread pool.

If you're already inside an async context (an Axum handler, an existing
`tokio::main`, etc.), you don't need to change anything else. If you're
embedding from a synchronous binary, the simplest pattern is to build a
runtime once and call `Runtime::block_on` at the boundary.

---

## 3. Hello world: list the resolved deps of a project

The smallest useful embedding example: load a `package.json` from the
current directory, resolve its dependency graph against the configured
registry, and print the chosen version of every package in the closure.

```rust
use guroku::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let manifest = Manifest::read_from(&cwd.join("package.json"))?;
    let client = RegistryClient::from_npmrc(&cwd)?;
    let roots: Vec<(String, String)> = manifest
        .all_dependencies()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let resolution = guroku::resolver::resolve(&client, &roots).await?;
    for (name, r) in resolution.iter() {
        println!("{} → {}", name, r.info.version);
    }
    Ok(())
}
```

A few things to notice:

- `guroku::prelude::*` brings in `Result`, `Manifest`, `RegistryClient`,
  `Resolution`, and `Lockfile`. Anything in the prelude is SemVer-covered
  (see Stability below).
- `Manifest::read_from` parses a `package.json`. It does not modify the
  file and does not require a `node_modules` directory to exist.
- `RegistryClient::from_npmrc(&cwd)` walks upward looking for project and
  user `.npmrc` files, merges them, and returns a configured client. See
  the Custom registries section for alternatives.
- `Manifest::all_dependencies` yields `(name, version_spec)` pairs across
  `dependencies`, `devDependencies`, and `optionalDependencies`. If you
  only want a subset, use the more specific `manifest.dependencies()`,
  `manifest.dev_dependencies()`, etc.
- `resolver::resolve` is async because it talks to the network. It
  returns a `Resolution` whose `iter()` walks the full transitive
  closure, not just the direct deps you passed in.

This program does not write anything to disk. It is purely a query.

---

## 4. Wiring the install pipeline

Once you have a `Resolution`, you have two options for actually
installing.

### Option A: one-shot install

```rust
use guroku::commands::install;
use guroku::prelude::*;

let cwd = std::env::current_dir()?;
let manifest = Manifest::read_from(&cwd.join("package.json"))?;
let client = RegistryClient::from_npmrc(&cwd)?;
let roots: Vec<_> = manifest.all_dependencies()
    .map(|(k, v)| (k.clone(), v.clone()))
    .collect();
let resolution = guroku::resolver::resolve(&client, &roots).await?;
install::install_from_resolution(&client, &cwd, &resolution).await?;
```

`install_from_resolution` fetches everything in the resolution into the
content-addressed store, links it into `node_modules`, and writes
`guroku.lock`. It's the same code path the CLI's `guroku install` uses,
just without the argv parsing. The function name is stable across the 1.x
line.

### Option B: drive each step yourself

If you want to interleave install steps with your own work, call the
underlying primitives directly:

```rust
use guroku::commands::fetch_into_cas;
use guroku::linker;

fetch_into_cas(&client, &resolution).await?;
linker::populate_node_modules(&cwd, &resolution).await?;
```

`fetch_into_cas` is the network/disk-heavy step. After it returns, every
package referenced by the resolution is present in the CAS. You could,
for instance, share a CAS across multiple projects and only call
`populate_node_modules` per-project.

`linker::populate_node_modules` is the disk-only step. It does no network
I/O. If you've already populated the CAS during a warm-up phase, you can
call this from a context with no internet access.

Splitting the two is also useful for diagnostics: if `fetch_into_cas`
succeeds but linking fails, you know the network and registry are fine
and the problem is local.

---

## 5. Reading and writing the lockfile

`guroku.lock` is a structured file the resolver/installer write at the
project root. You can read it programmatically:

```rust
let lock = Lockfile::read_from(&cwd.join("guroku.lock"))?;
for (key, entry) in &lock.packages {
    println!("{} resolves to {}", key, entry.resolved);
}
```

The keys in `lock.packages` are stable identifiers (name plus version,
plus a peer hash where applicable). The values include the resolved URL,
the integrity hash, and the dependency edges as the resolver saw them.

Writing the lockfile is normally the installer's job, but if you've
constructed a `Resolution` yourself and want to persist it:

```rust
let lock = Lockfile::from_resolution(&resolution);
lock.write_to(&cwd.join("guroku.lock"))?;
```

Lockfile format details, including the on-disk schema and version
field, live in `docs/lockfile-format.md`.

---

## 6. Error handling

All fallible operations in guroku return `guroku::Result<T>`, which is an
alias for `std::result::Result<T, GurokuError>`. `GurokuError` is an enum
covering the categories you'd expect: I/O errors, manifest parse errors,
registry errors, resolution conflicts, lockfile mismatches, and so on.

`GurokuError` is `#[non_exhaustive]`. New variants can be added in minor
releases. When you match on it, always include a `_` arm:

```rust
use guroku::GurokuError;

match do_something().await {
    Ok(v) => v,
    Err(GurokuError::Manifest(e)) => {
        eprintln!("bad package.json: {e}");
        return Err(e.into());
    }
    Err(GurokuError::Registry(e)) => {
        eprintln!("registry trouble: {e}");
        return Err(e.into());
    }
    Err(other) => return Err(other.into()),
}
```

The `Display` impl on `GurokuError` is meant to be user-facing: it
includes the path or URL or package name where relevant, and it does not
include stack traces or Rust type names.

If you want richer context (causes, span info), enable tracing as
described in the Logging section below. Errors carry a `source()` chain
you can walk with the standard `std::error::Error` trait.

---

## 7. Custom registries

Most embedders should let `RegistryClient::from_npmrc(&cwd)` figure out
what to do. It reads, in order, the project `.npmrc`, the user
`.npmrc` (`~/.npmrc`), and the built-in default. Auth tokens
(`//registry.example.com/:_authToken=...`) are picked up automatically
and attached to outgoing requests.

If you want to bypass `.npmrc` entirely:

```rust
use url::Url;
use guroku::RegistryClient;

let client = RegistryClient::new(Url::parse("https://registry.example.com/")?);
```

For an authenticated client without an `.npmrc`:

```rust
let client = RegistryClient::new(Url::parse("https://registry.example.com/")?)
    .with_bearer_token("npm_xxxxxx");
```

Scoped registries (`@myco:registry=...`) work the same way through
`from_npmrc`. If you're constructing the client by hand, you can register
scopes explicitly:

```rust
let client = RegistryClient::new(default_url)
    .with_scope("@myco", myco_url);
```

The full list of `.npmrc` keys guroku understands is in
`docs/npmrc.md`. Auth flows are in `docs/auth.md` and
`docs/private-registries.md`.

---

## 8. Driving resolution with overrides

If you want to pin a transitive dependency to a specific version
regardless of what its parent asks for, use overrides. The npm-compatible
form is a map from package name (or path) to a version spec.

```rust
use guroku::resolver;
use std::collections::BTreeMap;

let mut overrides = BTreeMap::new();
overrides.insert("lodash".into(), "4.17.21".into());
overrides.insert("nested>foo".into(), "2.0.0".into());

let resolution = resolver::resolve_with_overrides(
    &client,
    &roots,
    &overrides,
).await?;
```

The `nested>foo` form pins `foo` only when it appears under `nested`.
This matches npm's semantics. See `docs/overrides.md` for the full
syntax.

If you have a `Manifest` already loaded, `manifest.overrides()` returns
the overrides declared in `package.json`, in the same map format. A
common pattern is to start from those, layer your own on top, and pass
the merged map to `resolve_with_overrides`.

---

## 9. Working with non-registry deps

Not every dependency is `^1.2.3` from a registry. guroku also handles
file paths and Git URLs.

`classify_spec` takes a raw version string (the right-hand side of a
`dependencies` entry) and returns a `DepSpec`:

```rust
use guroku::classify_spec;
use guroku::DepSpec;

match classify_spec("github:foo/bar#main") {
    DepSpec::Range(_)        => { /* normal semver range */ }
    DepSpec::File(path)      => { /* file: or local path */ }
    DepSpec::Git(git)        => { /* git URL plus committish */ }
}
```

For `File` and `Git` variants, the resolver does not contact the
registry. Instead, it reads the local manifest (for `File`, the
`package.json` at the given path; for `Git`, after cloning into the
guroku Git cache).

The install pipeline links these via `Resolved::local_source`, which
returns the on-disk path the linker should symlink or copy from.
`Resolved::local_source` returns `None` for ordinary registry deps; in
that case the linker uses the CAS path instead.

If you only want to handle registry deps in your tool, you can filter
non-registry roots out before calling `resolve`. The resolver will
otherwise process them normally; details on the file: and git: protocols
are in `docs/file-deps.md` and `docs/git-deps.md`.

---

## 10. Threading

`RegistryClient` is `Send + Sync + Clone`. The clone is cheap: it shares
the underlying `reqwest::Client` (and its connection pool) via `Arc`.
Build one client at startup, clone it freely, and pass clones to
worker tasks.

`Resolution` and `Resolved` are `Send + Sync`. You can hand a `&Resolution`
to a `tokio::spawn`-ed task, or split the resolution into chunks and fan
out across tasks. Internally guroku already does this during install;
nothing prevents you from doing additional fan-out at the embedder level.

A typical pattern: build the client once, resolve once, then spawn one
task per package to do whatever post-install work your tool needs:

```rust
use std::sync::Arc;

let client = Arc::new(RegistryClient::from_npmrc(&cwd)?);
let resolution = Arc::new(resolver::resolve(&client, &roots).await?);

let mut handles = Vec::new();
for (name, _) in resolution.iter() {
    let client = client.clone();
    let resolution = resolution.clone();
    let name = name.to_string();
    handles.push(tokio::spawn(async move {
        do_something_per_package(&client, &resolution, &name).await
    }));
}
for h in handles {
    h.await??;
}
```

guroku does not impose a global concurrency limit on its public API. If
your tool needs one, wrap `RegistryClient` in your own semaphore.

---

## 11. Logging

guroku emits `tracing` events at `info`, `debug`, and `trace` levels. By
default no subscriber is installed, so the events go nowhere. To see
them, add a subscriber from your binary:

```rust
tracing_subscriber::fmt::init();
```

For more control:

```rust
use tracing_subscriber::{EnvFilter, fmt};

fmt()
    .with_env_filter(EnvFilter::from_default_env())
    .init();
```

Then run with `RUST_LOG=guroku=debug` to see internal events without
drowning in events from your other crates.

The CLI binary uses the same instrumentation. If you've set up a
`tracing` collector for your tool already (OpenTelemetry, Honeycomb,
plain JSON to stdout), guroku will automatically participate in it.

Spans are named after their function, with fields for the package name,
version, and registry where relevant. The naming is best-effort and not
covered by SemVer. If you build dashboards on top of it, expect to
update field selectors at major versions.

---

## 12. MSRV

guroku's minimum supported Rust version is **1.75**. Building against an
older toolchain will fail at compile time.

MSRV bumps are treated as breaking changes: they can only happen at a
new major version, never in a minor or patch release. This means once
you've pinned `guroku = "1"`, you can rely on the MSRV staying at 1.75
for the entire 1.x line.

For the rationale, history of past bumps, and the policy in detail, see
`docs/MSRV.md`.

---

## 13. Stability

Items reachable through `guroku::prelude` are SemVer-covered. That
means:

- `Result`, `GurokuError`
- `Manifest`, `RegistryClient`, `Resolution`, `Resolved`, `Lockfile`
- `DepSpec`, `classify_spec`
- The `commands::install::install_from_resolution` and
  `resolver::resolve` / `resolve_with_overrides` entry points

Other items in the crate (the `internal` module, `cas`, the various
subcommand-specific helpers) are public for the CLI's own use and may
change in any release.

If you find yourself reaching for an item outside the prelude and it's
not documented here, open an issue. Either we'll move it into the
stable surface, or we'll point you at the supported alternative. See
`docs/STABILITY.md` for the full policy.

---

## 14. Common pitfalls

- **Forgetting `#[tokio::main]` or `tokio::runtime`.** guroku is async
  end-to-end. If you call `resolver::resolve(...).await` outside an
  async context you'll get a "no reactor running" panic at runtime, not
  a compile error.

- **Building a `RegistryClient` per call.** Each `RegistryClient` owns a
  reqwest client, which owns a connection pool. Constructing one per
  request defeats the pool and dramatically slows down resolution. Build
  one client at the top of your program and clone it as needed; clones
  share the pool.

- **Catching `GurokuError::Other(_)` for everything.** `Other` is a
  catch-all for cases that don't fit a more specific variant. If you
  find yourself relying on it for a case that recurs in your tool,
  please file a bug: that case probably ought to have its own variant,
  and matching on `Other` substring text is brittle.

- **Modifying `node_modules` between `fetch_into_cas` and
  `populate_node_modules`.** The linker assumes it owns the directory.
  Concurrent edits from your tool can produce confusing errors and a
  half-linked tree. If you need to inject files, do it after the linker
  finishes.

- **Treating `Resolution` as serializable.** It's not, by design — it
  may contain handles into the registry client's caches. If you want to
  persist a resolution, write it as a lockfile (section 5) and read it
  back, rather than serializing the in-memory type.

- **Assuming `from_npmrc` reads environment variables.** It reads files.
  If you set `NPM_TOKEN` or similar in your CI, expand it into the
  `.npmrc` file (npm's standard `${NPM_TOKEN}` interpolation is
  supported) or pass the token directly via `with_bearer_token`.

For runtime issues that aren't embedding-specific, see
`docs/troubleshooting.md` and `docs/error-codes.md`.
