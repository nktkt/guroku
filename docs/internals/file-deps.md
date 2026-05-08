# `file:` Dependencies

This document describes how guroku handles local path dependencies declared via
the `file:` protocol in `package.json`. These dependencies are resolved against
the local filesystem rather than the registry, but they otherwise flow through
the same resolver, install, and link pipeline as registry packages.

## 1. Spec form

A `file:` dependency is declared in `package.json` like any other dependency,
but the version range is replaced with a `file:` URI pointing at a directory:

```json
{
  "name": "my-app",
  "version": "1.0.0",
  "dependencies": {
    "my-local": "file:./path/to/package",
    "shared-utils": "file:../shared",
    "vendor-lib": "file:/abs/path/to/vendor"
  }
}
```

Rules:

- The path is interpreted **relative to the project root** (the directory
  containing the `package.json` that declared the spec).
- **Absolute paths** are accepted as-is.
- The path must point at a directory containing a `package.json`.

## 2. Classification

In `src/specs.rs`, the `classify` function inspects the raw spec string and
returns a `DepSpec` enum. For `file:` specs, it strips the prefix and returns:

```rust
DepSpec::File(PathBuf)   // path with the `file:` prefix removed
```

Classification happens once, eagerly, when a manifest is loaded. The resolver
then dispatches on the variant.

## 3. Resolution

For `DepSpec::File(path)`, the resolver:

1. Joins the path against the consuming project's root if relative.
2. Reads `<path>/package.json` directly from disk.
3. Returns `FileDepMissingManifest { path }` if the file is missing or
   unreadable.
4. Synthesises a `VersionInfo` from the manifest's `name` and `version` fields.
   If either is absent, sentinels are used:
    - missing `name` -> the dependency key from the parent manifest.
    - missing `version` -> `0.0.0-local`.

There is no registry lookup, no tarball fetch, and no integrity check at this
stage. The synthesised `VersionInfo` carries enough metadata for the resolver
to walk transitive dependencies declared inside the local package.

## 4. `Resolved::local_source`

Every `Resolved` node in the dependency graph has an optional field:

```rust
pub struct Resolved {
    pub name: String,
    pub version: Version,
    pub local_source: Option<PathBuf>,
    // ... tarball_url, integrity, etc.
}
```

The install pipeline branches on this field:

- `local_source = None`
    - Standard registry path: fetch tarball, verify integrity against the CAS,
      extract into the content store, then link.
- `local_source = Some(p)`
    - Skip CAS entirely. The path `p` is passed straight through to the linker
      as `source_dir`.

`p` may be absolute or relative. If relative, it is interpreted relative to the
consuming project's root (same convention as the spec).

## 5. Linking

`into_linked_packages` walks the resolved graph and, for each node, builds a
`LinkedPackage`:

```rust
pub struct LinkedPackage {
    pub id: PackageId,
    pub name: String,
    pub source_dir: PathBuf,   // either CAS path or local_source
    pub deps: Vec<PackageId>,
}
```

For `file:` deps, `source_dir = local_source.unwrap()`.

The strict-layout linker then walks `source_dir` and **hardlinks** every file
into the target slot:

```
node_modules/.guroku/<id>/node_modules/<name>/<files>
```

Directories are recreated; regular files are hardlinked. Symlinks inside the
source tree are preserved as symlinks. Files matched by `.npmignore` /
`files` whitelist semantics are respected, just as with a registry package.

## 6. Why hardlinks (not symlinks)

The strict layout uses hardlinks throughout: registry packages are hardlinked
out of the CAS, and `file:` packages are hardlinked out of the source tree.
This keeps the layout uniform — every file under `node_modules/.guroku/` is a
real file in the consuming project's filesystem, not a pointer to somewhere
outside it.

A future flag could opt into "live editing" by symlinking the package
directory instead, similar to pnpm's `link:` protocol. That would make edits
in the source tree visible immediately without re-running install, at the cost
of breaking the strict-layout invariant. This is not currently implemented.

## 7. Re-running install picks up changes

Hardlinks share an inode with the source file. The practical consequences:

- **Modifying an existing file** in the source dir is reflected immediately in
  every linked location. No re-install needed: the inode is shared.
- **Adding a new file** is *not* visible until the next `guroku install`. The
  linker will create a new hardlink for it on the next run.
- **Deleting a file** in the source dir leaves a stale hardlink behind under
  `node_modules/.guroku/`. The next `guroku install` will detect the
  divergence and prune it.

```sh
# Edit existing file -- visible everywhere immediately.
$EDITOR ./path/to/package/index.js

# Add new file -- run install to pick it up.
touch ./path/to/package/new-helper.js
guroku install
```

## 8. Lockfile

`file:` deps are recorded in `guroku.lock` like any other dependency, with the
synthesised version. The `resolved` URL field uses a placeholder:

```
file:///guroku-local-source
```

This sentinel exists because the lockfile schema requires *some* URL, but a
real path would be project-specific and break reproducibility across machines.

Reproducibility for `file:` deps relies on the fact that the consuming
project's `package.json` carries the correct `file:` spec. Cloning the repo
and running `guroku install` will re-resolve the local path on each machine.
The lockfile pins the *shape* of the dependency graph (transitive deps,
synthesised version) but cannot pin the *contents* of the local directory.

Example lockfile entry:

```json
{
  "my-local@0.0.0-local": {
    "resolved": "file:///guroku-local-source",
    "integrity": null,
    "dependencies": {
      "lodash": "4.17.21"
    }
  }
}
```

## 9. Caveats

### Path is relative to the consuming project

The recorded path is interpreted relative to the directory containing the
consuming `package.json`. Moving that `package.json` somewhere else without
also moving (or updating) the `file:` target will break the next install.

### Cyclic file deps

`file:` deps can form cycles:

```
A/package.json   -> "b": "file:../B"
B/package.json   -> "a": "file:../A"
```

This works as long as both manifests are valid on disk. The resolver's
sticky-first cycle handling tolerates the cycle: the first node visited for a
given identity wins, and back-edges become references rather than recursive
visits.

### Lockfile cannot pin contents

Two developers checking out the same repo will get the same dependency graph,
but the actual *bytes* under each `file:` target can differ if their working
trees differ. This is by design — `file:` deps are intended for development
loops, not for distributing code.

## 10. Comparison with other package managers

### npm

Similar semantics: npm reads the local manifest and either copies the
directory contents into `node_modules` or symlinks (under
`--install-strategy=linked`). guroku's hardlink approach is closer to npm's
default copy than to its symlink strategy, but cheaper because hardlinks share
inodes.

### pnpm

pnpm distinguishes `file:` (which copies) from `link:` (which symlinks the
directory and is the recommended form for in-monorepo links). pnpm's
content-addressable store is bypassed for both. guroku currently has only
`file:`; a future `link:` (symlink-the-directory) variant could be added.

### yarn classic

Yarn classic supports both `link:./path` and `file:./path`. `link:` symlinks
the package directory; `file:` packs and unpacks (effectively copies). guroku
behaves like yarn's `file:` but uses hardlinks instead of full copies.

### Summary

| Manager       | `file:` behaviour          | Live-edit form |
|---------------|----------------------------|----------------|
| npm           | copy (symlink under flag)  | `--install-strategy=linked` |
| pnpm          | copy                       | `link:` |
| yarn classic  | pack/unpack copy           | `link:` |
| guroku        | hardlink                   | (not yet)      |

## See also

- `docs/internals/specs.md` — full spec classification rules.
- `docs/internals/strict-layout.md` — the `node_modules/.guroku/` layout.
- `docs/internals/hardlinks.md` — hardlink semantics across platforms.
- `docs/internals/lockfile.md` — lockfile format.
