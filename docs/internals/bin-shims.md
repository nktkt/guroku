# node_modules/.bin/ Shims

This document describes how guroku v0.4 materialises executable shims for
direct dependencies under `node_modules/.bin/`. These shims are what makes
`npm run <script>` work: when a package script invokes `eslint`, `tsc`, or
`mocha`, the shell finds those binaries because `node_modules/.bin/` is
prepended to `PATH` before the script runs.

## What is a `.bin/` shim

Every npm package may declare one or more executables in its
`package.json#bin` field. When that package is installed as a dependency,
the package manager creates a symlink under
`<project>/node_modules/.bin/<name>` pointing at the actual JS file inside
the package. npm-style script runners (and guroku's own `run` command)
prepend `node_modules/.bin/` to `PATH` before spawning a script, so a bare
`eslint` or `tsc` invocation resolves to the right file via normal `PATH`
lookup.

In short:

```
package.json#bin   ->   node_modules/.bin/<name>   ->   actual cli.js
```

Without these shims, `npm run lint` could not find `eslint` unless the
script wrote out `./node_modules/eslint/bin/eslint.js` by hand.

## The two `bin` shapes

`package.json#bin` accepts two shapes. Both must be supported.

### String form

```json
{
  "name": "eslint",
  "bin": "./bin/eslint.js"
}
```

When `bin` is a string, the bin name is taken from the package's `name`
field. For scoped packages, the scope (`@scope/`) is dropped:

```json
{
  "name": "@babel/cli",
  "bin": "./bin/babel.js"
}
```

Produces a shim at `node_modules/.bin/cli` -- the leading `@babel/` is
stripped, leaving the bare package name. (npm itself does the same.)

### Object form

```json
{
  "name": "typescript",
  "bin": {
    "tsc": "./bin/tsc",
    "tsserver": "./bin/tsserver"
  }
}
```

When `bin` is an object, each key becomes a separate shim name and each
value is the path inside the package. A single dependency can therefore
contribute multiple bins.

## Where shims live

```
<project>/
  node_modules/
    .bin/                       <-- one symlink per (package, bin_name)
      tsc
      tsserver
      eslint
      cli
    typescript/                 <-- direct-dep alias (see strict-layout.md)
    eslint/
    .guroku/
      typescript@5.4.5/
        node_modules/
          typescript/
            bin/
              tsc               <-- the real file
```

There is exactly one `node_modules/.bin/` directory per project root.
guroku does not (in v0.4) create per-package nested `.bin/` directories;
transitive dep bins are not made available on `PATH`.

## What the symlink actually points at

The shim target is the real script file inside the package's installation
under `.guroku/<name>@<version>/node_modules/<name>/<rel>`. For example,
the `tsc` shim points at:

```
../.guroku/typescript@5.4.5/node_modules/typescript/bin/tsc
```

Note that the target is a *relative* path. This matters: the entire
`node_modules` tree can be copied or moved between machines (as long as
the layout stays intact) without breaking shims. If guroku stored
absolute paths, the tree would be tied to the specific user and project
location at install time.

## Direct deps only

v0.4 only generates shims for *direct* dependencies -- packages listed
in the project's own `dependencies` / `devDependencies` / `optionalDependencies`.
Transitive dependencies' bins are not exposed.

This matches pnpm's behaviour. npm differs: it shims everything, which
leads to the equivalent of phantom dependencies -- a script can invoke a
binary contributed by some random transitive package and it just works,
until that package is no longer pulled in transitively and the script
silently breaks.

guroku takes the stricter view: if a script wants to use `mocha`, the
project must declare `mocha` itself.

## Order of operations during install

The bin-shim step runs after package materialisation, never before. The
sequence is:

```
1. resolve()                 // build the dep graph
2. fetch_to_cas()            // populate the content-addressed store
3. populate_node_modules()   // hardlink files into .guroku/<name>@<ver>/...
4. populate_bin_dir()        // create node_modules/.bin/<name> symlinks
```

`populate_bin_dir` runs last because it depends on the bin script files
already existing on disk (hardlinked from the CAS) -- otherwise the
freshly-created symlink would dangle. By the time step 4 runs, every
file referenced by every direct dep's `package.json#bin` is already on
disk under `.guroku/...`.

## POSIX exec bit

A symlink alone is not enough on POSIX: the underlying file must have
the executable bit set. guroku tries to set the source file's mode to
`0o755` before creating the shim:

```
chmod 0755 .guroku/typescript@5.4.5/node_modules/typescript/bin/tsc
ln -s ../.guroku/typescript@5.4.5/.../bin/tsc node_modules/.bin/tsc
```

Most tarballs already ship the bin file with mode `0o755` and the CAS
preserves whatever mode was extracted, so this is usually a no-op. The
explicit `chmod` exists as a safety net for tarballs that lost their
exec bit somewhere in transit (rare, but it happens with poorly-built
publishes).

## Windows

On Windows the linker uses `symlink_file_or_dir` with the
`cfg(windows)` branch (see `crates/linker/src/symlinks.rs`). Plain
file symlinks on Windows require either:

- Administrator privileges, or
- Developer Mode enabled (Settings > Privacy & security > For developers).

Without one of those, `CreateSymbolicLinkW` returns
`ERROR_PRIVILEGE_NOT_HELD` and the install errors out with a message
pointing the user at Developer Mode. This is a known papercut and is
why most Windows-targeting npm-likes ship `.cmd` wrapper shims instead
of symlinks (see "What v0.4 does NOT yet do" below).

## What v0.4 does NOT yet do

Several known gaps; documented here so they don't surprise contributors.

### `.cmd` wrapper shims on Windows

npm v6+ generates a small `<name>.cmd` (and `<name>.ps1`) wrapper next
to each shim on Windows, which lets Windows shells execute the bin
without symlinks. This sidesteps the Developer Mode requirement and
also gracefully handles the case where the JS file has no shebang.
guroku v0.4 does not generate these wrappers; it relies entirely on
symlinks plus `node`'s own resolution.

Planned for a later version.

### Conflict detection

If two direct deps both declare a bin named `foo`, v0.4's behaviour is
"last write wins" -- whichever package the linker visits second
overwrites the symlink from the first. There is no warning, no error,
no way to pick which one to keep.

A real conflict resolver should:

- Detect the collision during graph building.
- Either error out, or pick a winner deterministically and log a warning.

This is tracked as a known limitation.

## Diagnostics

Inspect what shims got generated:

```
$ ls -la node_modules/.bin/
total 0
lrwxr-xr-x  1 user  staff   54 May  7 12:00 eslint -> ../.guroku/eslint@8.57.0/node_modules/eslint/bin/eslint.js
lrwxr-xr-x  1 user  staff   54 May  7 12:00 tsc    -> ../.guroku/typescript@5.4.5/node_modules/typescript/bin/tsc
lrwxr-xr-x  1 user  staff   60 May  7 12:00 tsserver -> ../.guroku/typescript@5.4.5/node_modules/typescript/bin/tsserver
```

Resolve a single shim's target:

```
$ readlink node_modules/.bin/tsc
../.guroku/typescript@5.4.5/node_modules/typescript/bin/tsc
```

Check that the resolved target actually exists and is executable:

```
$ ls -la "$(readlink -f node_modules/.bin/tsc)"
-rwxr-xr-x  ...  .guroku/typescript@5.4.5/node_modules/typescript/bin/tsc
```

If a shim is dangling (target does not exist), something went wrong
between `populate_node_modules` and `populate_bin_dir` -- usually a
bug in path computation, occasionally a partial install that was
interrupted. Re-running `guroku install` should heal it.
