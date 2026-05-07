# Path Handling

How guroku decides where files go, how it computes path strings, and which
helpers own the canonical answer. If you find yourself joining a path with `+`,
re-implementing scope flattening, or calling `std::env::current_dir` from a
random module, this doc is a hint that you're off the paved road.

## 1. Project paths

The "project" is whatever directory contains the `package.json` we're operating
on. It's resolved exactly once, at the top of the CLI:

```rust
// crates/cli/src/lib.rs
impl Cli {
    pub fn cwd_or_current(&self) -> Result<PathBuf> {
        match &self.cwd {
            Some(p) => Ok(p.clone()),
            None    => Ok(std::env::current_dir()?),
        }
    }
}
```

Every other module receives the resolved root as a `&Path` argument. The
canonical names below are all derived by `Path::join`:

| Logical name      | Path                                     |
|-------------------|------------------------------------------|
| Project root      | `<cwd>`                                  |
| Manifest          | `<cwd>/package.json`                     |
| Install tree      | `<cwd>/node_modules/`                    |
| Hidden store      | `<cwd>/node_modules/.guroku/`            |
| Lockfile          | `<cwd>/guroku.lock`                      |
| Bin shim dir      | `<cwd>/node_modules/.bin/`               |

Only `Cli::cwd_or_current` calls `std::env::current_dir`. Anywhere else, take
the root as a parameter.

## 2. Per-user paths

The per-user root is `~/.guroku/`. It is resolved through `cache::home`:

```rust
// crates/cache/src/lib.rs
pub fn home() -> Result<PathBuf> {
    let base = dirs::home_dir().ok_or(Error::NoHomeDir)?;
    Ok(base.join(".guroku"))
}
```

`cache::home` honours nothing today, but it is the natural seam for a future
`GUROKU_HOME` env override. Modules must go through it; do not call
`std::env::var("HOME")` directly and do not hand-construct `~/.guroku`.

Layout under the per-user root:

```
~/.guroku/
  cas/<sha[0:2]>/<sha[2:]>/        # content-addressable store
  cache/metadata/<safe>.json       # registry metadata cache
  cache/http/                      # HTTP response cache (etag/304)
  tmp/                             # staging area for atomic moves
```

## 3. Scoped-name flattening

npm package names can include a `/` (e.g. `@types/node`). That slash is fine
inside `node_modules/` because we get to use a real subdirectory, but it is
poison anywhere we need a single directory component.

`cache::safe_segment` does the flattening:

```rust
// crates/cache/src/lib.rs
pub fn safe_segment(name: &str) -> String {
    name.replace('/', "+")
}

assert_eq!(safe_segment("@types/node"), "@types+node");
assert_eq!(safe_segment("lodash"),      "lodash");
```

Where each form is used:

| Location                                | Form                          | Example                                  |
|-----------------------------------------|-------------------------------|------------------------------------------|
| CAS entry                               | sha-keyed, name-agnostic      | `cas/ab/cdef.../`                        |
| `.guroku/<id>/`                         | `safe_segment(name)@version`  | `.guroku/@types+node@20.0.0/`            |
| Metadata cache                          | `safe_segment(name).json`     | `cache/metadata/@types+node.json`        |
| `node_modules/<name>/` (user-visible)   | original (slash preserved)    | `node_modules/@types/node/`              |

The "real" tree (the one Node walks) preserves the scope-as-directory form
because that is what `require()` and `import` resolution expect. The hidden
store under `.guroku/` flattens because each entry must occupy exactly one
directory entry.

CAS entries are tarball-keyed by sha and therefore scope-agnostic. There is no
scoped-name handling in `cache::cas_entry` at all, because there is no name in
the path.

## 4. PATH inside spawned scripts

Lifecycle scripts run with an augmented `PATH`. The contract is in
`scripts::set_path`:

```rust
// crates/scripts/src/env.rs
pub fn set_path(cmd: &mut Command, prepend: &[PathBuf]) {
    let inherited = std::env::var_os("PATH").unwrap_or_default();
    let mut parts: Vec<PathBuf> = prepend.to_vec();
    parts.extend(std::env::split_paths(&inherited));
    let joined = std::env::join_paths(parts).expect("PATH join");
    cmd.env("PATH", joined);
}
```

Callers prepend, in order:

1. `<project>/node_modules/.bin/` (the workspace shim dir)
2. The current package's own `<.guroku>/<id>/node_modules/.bin/` if any

then the inherited `PATH` follows. **Order matters**: `which` resolves
left-to-right, and the project-local shim wins over a globally installed
binary of the same name. This is also the reason we never *append* — a
user-installed `tsc` must not shadow the workspace's pinned one.

## 5. Symlink targets are relative

`node_modules/<name> -> .guroku/<id>/node_modules/<name>` symlinks are written
with **relative** targets so that moving or renaming the project does not
break them.

```rust
// crates/linker/src/path.rs
pub fn relative_to(target: &Path, base: &Path) -> PathBuf {
    // Walk both, drop the common prefix, prepend ".." for each
    // remaining component in `base.parent()`, then push the
    // remainder of `target`.
}
```

So for a link at `node_modules/lodash` pointing into the hidden store, we
emit:

```
node_modules/lodash -> ../.guroku/lodash@4.17.21/node_modules/lodash
```

Never an absolute path. Tests in `linker::path::tests` cover the
move-the-project case.

## 6. CAS path layout invariants

The two-character prefix split (`cas/<sha[0:2]>/<sha[2:]>/`) is enforced in
exactly one place:

```rust
// crates/cache/src/cas.rs
pub fn cas_entry(home: &Path, sha: &str) -> PathBuf {
    debug_assert!(sha.len() >= 4 && sha.chars().all(|c| c.is_ascii_hexdigit()));
    home.join("cas").join(&sha[0..2]).join(&sha[2..])
}
```

See [`cas.md`](./cas.md) for the rationale (filesystem fan-out, dirent count
caps, etc.). Do not reconstruct CAS paths anywhere else.

## 7. Manifest paths

`manifest.bin_entries()` returns the `(name, relative_path)` pairs declared
under `"bin"` in `package.json`. The relative path is interpreted from the
package root, not from the project root:

```rust
// crates/manifest/src/lib.rs
pub fn bin_entries(&self) -> Vec<(String, PathBuf)> { /* ... */ }
```

The linker resolves them against the unpacked location:

```rust
let pkg_root = guroku_dir.join(format!("{}@{}", safe_segment(name), version))
                         .join("node_modules")
                         .join(name); // scope-as-dir
let bin_abs  = pkg_root.join(rel);
```

`rel` is whatever the manifest said — typically `bin/foo.js` or `cli.js`.
We never canonicalise it; symlinks inside the package would resolve through
the user's filesystem and we want the in-tree path.

## 8. Workspace globs

`package.json` `workspaces` patterns are interpreted relative to the *root*
project. The evaluator joins each pattern with `<cwd>` before handing it to
`glob::glob`:

```rust
// crates/workspaces/src/lib.rs
for pattern in patterns {
    let abs = cwd.join(pattern);
    for entry in glob::glob(abs.to_str().ok_or(Error::NonUtf8Glob)?)? {
        // ...
    }
}
```

Two consequences worth knowing:

- A leading `./` in the manifest is fine; `Path::join` collapses it.
- A leading `/` in the manifest is **not** rooted at the filesystem; `glob`
  treats `cwd.join("/abs")` as `/abs`, which is almost certainly a bug in the
  manifest. We do not currently validate this; treat it as user error.

Globs are matched with default `glob` options (no follow-symlinks, case
sensitive). Symlinked workspace roots are not supported.

## 9. Cross-platform paths

Windows uses `\`; Unix uses `/`. We use `Path::join` everywhere, which
produces the right separator for the current target. Do **not** hand-build
paths with literal `"/"` joins.

```rust
// good
let p = root.join("node_modules").join(".bin");

// bad
let p = format!("{}/node_modules/.bin", root.display());
```

The one allowed exception is the strict-layout symlink target generator: it
emits `/`-separated targets unconditionally. Windows tolerates forward slashes
in symlink target strings (the kernel normalises them on resolution), and the
strict-layout output is meant to round-trip through tooling that may parse it
as a POSIX-style path. See [`strict-layout-windows.md`](./strict-layout-windows.md)
for the full rationale.

`safe_segment` produces `+`-flattened names that are valid on every supported
filesystem. We do not need additional sanitisation for Windows because npm
already disallows the characters that would be a problem (`<>:"|?*`).

## 10. Things to avoid

A non-exhaustive list of patterns that look reasonable but break invariants:

- **Calling `std::env::var("HOME")` directly.** Use `cache::home`. The whole
  point of the helper is to give us one place to add an env override.
- **Calling `std::env::current_dir` outside `Cli::cwd_or_current`.** The CLI
  resolves the root once. Threading the root through call sites is cheap and
  makes tests trivial.
- **Absolute paths in symlink targets.** Always relative. Run the
  move-the-project test in your head before merging.
- **Hand-flattening scoped names.** Use `cache::safe_segment`. If you find
  yourself writing `name.replace('/', "_")` or `name.replace('/', "__")`,
  stop — we use `+` and the suffix is part of the on-disk format.
- **Reconstructing CAS paths.** Use `cache::cas_entry`. The two-char prefix is
  load-bearing.
- **Joining paths with `format!` and `/`.** Use `Path::join`. This breaks on
  Windows in subtle ways (the test suite passes because CI is Linux-only for
  the path-heavy crates).
- **Following symlinks before computing relative targets.** `linker::relative_to`
  works on lexical paths. If you `canonicalize` first, the move-the-project
  guarantee is silently lost.

## See also

- [`cas.md`](./cas.md) — CAS layout invariants
- [`symlinks.md`](./symlinks.md) — what links we create and why
- [`strict-layout.md`](./strict-layout.md) — alternate on-disk layout
- [`strict-layout-windows.md`](./strict-layout-windows.md) — Windows symlink notes
- [`workspaces.md`](./workspaces.md) — workspace resolution end-to-end
