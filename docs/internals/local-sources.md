# Local sources

This document unifies the `file:` and `git+` install paths under a single
mental model. In v0.5 these are the two ways a package's bytes can come from
the user's own filesystem instead of from a registry tarball, and although
the user-facing specs look very different, the install pipeline treats them
almost identically once resolution finishes.

Read this alongside [file-deps.md](./file-deps.md), [git-deps.md](./git-deps.md),
[strict-layout.md](./strict-layout.md), and [hardlinks.md](./hardlinks.md).

## 1. What "local source" means in v0.5

A *local source* is any package whose installable bytes live in a directory
on the user's filesystem at install time, rather than being fetched as a
tarball from a registry. Two shapes exist today:

- `file:./path` — a relative or absolute path to a package directory.
- `git+https://...` (and friends) — a git URL that we clone into a guroku-
  managed cache directory before reading.

Once cloned, a git dep is structurally indistinguishable from a file dep:
both are "a directory on disk that contains a `package.json`". This is the
key abstraction that lets the rest of the pipeline stay unaware of the
distinction. Anything that *isn't* one of these two is, by definition, a
registry dep, and goes through the CAS-backed tarball path described in
[cas.md](./cas.md).

## 2. The `Resolved::local_source` field

Every node the resolver emits is a `Resolved`. To carry the local-vs-registry
distinction without growing two parallel struct variants, we add one field:

```rust
pub struct Resolved {
    pub name: String,
    pub version: Version,
    pub info: VersionInfo,
    pub deps: Vec<Resolved>,
    /// Some(path) for file:/git: deps; None for registry deps.
    pub local_source: Option<PathBuf>,
}
```

The invariant is simple:

- `local_source.is_none()` — fetch via CAS, hardlink from the CAS into the
  strict layout.
- `local_source.is_some()` — skip CAS entirely, hardlink directly from the
  given directory into the strict layout.

The install pipeline branches on this field at exactly two points: the CAS
fetch queue (skips `Some`) and the linker's `source_dir` selection (uses the
`Some` value verbatim). Everything else — graph building, peer resolution,
overrides, lockfile writing, lifecycle scripts — handles registry and local
nodes uniformly.

## 3. Spec to local path mapping

The mapping from `DepSpec` to `local_source` happens in the resolver, in the
arm that classifies the spec.

### File specs

```rust
DepSpec::File(p) => {
    let path = if Path::new(&p).is_absolute() {
        PathBuf::from(p)
    } else {
        project_cwd.join(p)
    };
    Resolved {
        local_source: Some(path),
        // ...synthesised VersionInfo, see section 4
    }
}
```

The path is resolved relative to *the consuming project's cwd*, not the cwd
of whatever transitive package declared the dep. This matches npm's
historical behaviour and is the only choice that makes lockfile reuse from a
checked-out repo coherent. Absolute paths pass through verbatim, which is
useful in monorepo bootstrap scripts but should not appear in committed
manifests.

### Git specs

```rust
DepSpec::Git(g) => {
    let clone_dir = git::ensure_cloned(&g)?;
    Resolved {
        local_source: Some(clone_dir),
        // ...synthesised VersionInfo, see section 4
    }
}
```

`git::ensure_cloned` is the entry point documented in
[git-deps.md](./git-deps.md). It returns the on-disk path of a fresh or
cached clone with the requested ref checked out. From the resolver's
perspective the only difference from a file dep is that the path was
produced by a side-effecting clone rather than read from the spec.

## 4. Synthetic `VersionInfo` — `read_local_manifest`

The rest of the resolver, the lockfile writer, and the linker all expect to
work with a `VersionInfo` (the registry's per-version document shape). To
keep them shape-stable we synthesise one for local sources. The helper is
`read_local_manifest`, which reads the package's own `package.json` and
fills in:

```rust
fn read_local_manifest(dir: &Path, declared_name: &str) -> Result<VersionInfo> {
    let manifest = read_package_json(dir)?;
    Ok(VersionInfo {
        name: manifest.name.unwrap_or_else(|| declared_name.to_string()),
        version: manifest
            .version
            .unwrap_or_else(|| "0.0.0-local".parse().unwrap()),
        dist: Dist {
            tarball: "file:///guroku-local-source".to_string(),
            integrity: None,
            shasum: None,
        },
        dependencies: manifest.dependencies.unwrap_or_default(),
        // ...peer/optional/dev dropped per usual rules
    })
}
```

Field-by-field:

- **name** — taken from the manifest's `name` field. If the manifest is
  missing the field (rare but legal in private/local trees) we fall back to
  the *declared* name from the consuming project's `package.json`. This
  fallback only fires for local sources; registry packages always have a
  name.
- **version** — taken from the manifest's `version`. Falls back to the
  sentinel `0.0.0-local`. This sentinel is what shows up in `guroku.lock`
  for unversioned local trees, and is the signal we use in diagnostics to
  remind users that the lockfile is not pinning anything meaningful.
- **dist.tarball** — set to the placeholder URL `file:///guroku-local-source`.
  This URL is *never fetched*; it exists only because downstream code
  unconditionally serializes a `resolved` field into the lockfile and we
  don't want a `null` there. Any code that tries to fetch this URL is a
  bug — the CAS skip in section 5 is what protects us.
- **dependencies** — copied from the manifest. Transitive resolution of
  these proceeds normally: a file dep's deps can themselves be registry,
  file, or git deps, and so on, with no special-casing needed.

## 5. Skipping the CAS

`commands::install::install_from_resolution` is the single funnel through
which every install passes after resolution. It assembles a queue of CAS
fetches, then drives the linker. The CAS-queue step filters local sources
out:

```rust
let cas_jobs: Vec<_> = resolution
    .iter()
    .filter(|r| r.local_source.is_none())
    .map(|r| FetchJob::from(r))
    .collect();

cas::fetch_into_cas(&cas_jobs, &client).await?;
```

Local-source nodes never enter `fetch_into_cas`, so they never touch the
HTTP client, never compute integrity, and never write into the CAS. Their
bytes stay where they are; the linker reads from the source directory
directly. This is the only place the install pipeline has a hard branch on
`local_source` other than the linker.

A subtle consequence: if a registry dep and a local dep happen to have the
same name and version, we will still create two distinct nodes in the
strict layout (different IDs), because the local one has no integrity hash
to deduplicate against. This is intended.

## 6. Linking

`into_linked_packages` translates the `Resolved` graph into a `Vec<LinkedPackage>`
that the strict-layout linker consumes. The relevant slice:

```rust
for r in resolution.iter() {
    let source_dir = match &r.local_source {
        Some(path) => path.clone(),
        None => cas.path_for(&r.info.dist).to_path_buf(),
    };
    linked.push(LinkedPackage {
        id: id_for(r),
        name: r.name.clone(),
        source_dir,
        // ...
    });
}
```

The strict-layout linker (see [strict-layout.md](./strict-layout.md))
hardlinks each file from `source_dir` into
`node_modules/.guroku/<id>/node_modules/<name>/...`. The end-user-visible
`node_modules/<name>` symlink then points to that materialised tree, *not*
to the original source directory.

This indirection matters: a consumer doing `realpath
node_modules/some-local-dep` lands inside `.guroku/<id>/...`, not in the
local source. That is the correct answer for the strict layout's
guarantees, even though it sometimes surprises users who expect a direct
symlink into their working tree.

## 7. Why hardlinks, not symlinks, into the source

Three reasons we hardlink the source's files into `.guroku/<id>/...`
rather than symlinking `node_modules/<name>` straight at the source dir:

1. **Consistency with the rest of the strict layout.** Every other package
   in `.guroku/` is a hardlink farm. Making local sources the one
   exception would force every consumer of the layout (the runner, the
   bin-shim writer, audit, etc.) to handle a second case.
2. **Editing in place still works.** Hardlinks share inodes, so a write to
   the file in the source directory is immediately visible through the
   `.guroku/<id>/...` path, and vice versa. Users get the live-editing
   ergonomics they expect from `file:` deps without us having to special-
   case the layout.
3. **The lockfile's `resolved` URL is meaningless for locals.** If we
   symlinked `node_modules/<name>` directly at the source, the placeholder
   `file:///guroku-local-source` would surface in `realpath` output and in
   any tool that walks `node_modules`. Materialising into `.guroku/<id>/...`
   keeps the placeholder strictly internal.

The trade-off is that *adding or removing* files in the source dir is not
picked up automatically — only edits to existing files are. A re-install
re-runs the linker and reconciles. v0.5 does not watch local sources.

## 8. Lockfile entries

Local-source packages appear in `guroku.lock` like any other package, with
two cosmetic differences:

- `version` is whatever `read_local_manifest` produced (often the
  `0.0.0-local` sentinel).
- `resolved` is the placeholder `file:///guroku-local-source`.

```yaml
"my-local-dep@0.0.0-local":
  resolved: "file:///guroku-local-source"
  dependencies:
    lodash: "^4.17.21"
```

Reproducibility for a local-source entry depends entirely on the consuming
project's `package.json#dependencies` carrying the correct `file:` or
`git+` spec. The lockfile cannot, by itself, locate the source — it
records what the source *contained* at install time, not where to find it.
This is the same trade-off npm and pnpm make.

For git deps in particular, the spec should pin a commit SHA or a tag;
floating refs (`#main`) defeat reproducibility even though the lockfile
will happily record the resolved tree.

## 9. Per-package postinstall

Local-source packages run their own `postinstall` (and other lifecycle
scripts — see [lifecycle.md](./lifecycle.md)) the same way registry
packages do. There is no opt-out and no special prompt; if a user adds a
`file:` dep with a `postinstall`, it runs.

The script's cwd is the *materialised* directory:

```sh
# cwd when running postinstall for a local-source package
node_modules/.guroku/<id>/node_modules/<name>
```

It is **not** the original source path. Two reasons:

- The script sees the same view of `node_modules/` that it would for any
  registry package, so behaviours like resolving sibling `.guroku` deps
  work uniformly.
- Writes the script makes to its cwd (e.g. compiling a native addon into
  `build/Release/`) land in the materialised tree, not the user's source
  dir. This avoids polluting a checked-out git working tree on every
  install.

If a postinstall script needs to know the original source path it can read
`process.env.GURUKU_LOCAL_SOURCE`, which the runner sets when
`local_source.is_some()`.

## 10. What v0.5 doesn't yet support

The local-source model in v0.5 deliberately covers only the two shapes
above. The following are known omissions, in rough order of how often
they come up:

- **`link:./path`** — yarn's symlink protocol. Semantically distinct from
  `file:`: it would symlink `node_modules/<name>` directly at the source
  directory for live editing, bypassing the `.guroku/<id>/...` materialisation
  entirely. We have a sketch but no implementation; it interacts non-
  trivially with the strict layout's invariants.
- **Git repos whose `package.json` lives in a subdirectory.** Some repos
  are monorepos where the publishable package is at e.g. `packages/foo/`.
  npm spells this `git+https://...#path:packages/foo`. v0.5 has no
  `path:` field on `DepSpec::Git`, so the only supported case is
  package-at-root.
- **Submodules.** `git::ensure_cloned` does *not* run `git submodule
  update --init`. Packages that need their submodules will fail at the
  manifest-read step, or worse, install in a broken state.
- **SVN, Mercurial, fossil.** Only git is wired up. There is no plugin
  hook for other VCSes; adding one would mean abstracting `git::ensure_cloned`
  behind a trait.

Each of these is tracked separately. None block v0.5.

## 11. Testing

Coverage is split between unit and integration:

- `tests/specs_*.rs` — unit tests over the `DepSpec` parser. These cover
  classification: `file:./x`, `file:/abs/x`, `git+https://...`,
  `git+ssh://...`, `https://.../tar.gz`, etc., and verify that each is
  routed to the correct `local_source` policy without touching disk.
- The full install path for local sources (clone, read manifest,
  hardlink into `.guroku/<id>/...`, run postinstall) requires a real
  local directory or a real git clone. v0.5 ships unit tests only;
  integration coverage is deferred to v0.6 along with the test
  harness work tracked in [parallelism.md](./parallelism.md).

In practice the unit tests catch ~all classification regressions, and
the install-side code is exercised end-to-end by manual smoke runs
during release.

## See also

- [specs.md](./specs.md) — the upstream `DepSpec` parser.
- [file-deps.md](./file-deps.md) — `file:` spec details and edge cases.
- [git-deps.md](./git-deps.md) — `git::ensure_cloned`, ref handling, and
  the clone cache layout.
- [strict-layout.md](./strict-layout.md) — what `.guroku/<id>/...` looks
  like and why.
- [hardlinks.md](./hardlinks.md) — the cross-filesystem fallback story
  that also applies to local sources.
- [lockfile.md](./lockfile.md) — lockfile schema and the `resolved` field
  semantics.
- [lifecycle.md](./lifecycle.md) — postinstall and friends.
