# Workspaces

guroku supports npm-style workspaces: a single `package.json` at the root of
a monorepo declares one or more sub-package directories, and each
`package.json` underneath becomes a "workspace package".

This document covers what works in v0.4 and what is planned for v0.5. For
the implementation details (resolver hooks, discovery internals), see
[`docs/internals/workspaces.md`](internals/workspaces.md).

## What workspaces are

A workspace is a collection of packages managed from a single root. The
root `package.json` is typically marked `private` and lists the
sub-package globs:

```json
{
  "name": "my-monorepo",
  "private": true,
  "workspaces": ["packages/*"]
}
```

Given a layout like:

```
my-monorepo/
  package.json
  packages/
    util/
      package.json
    api/
      package.json
```

`packages/util` and `packages/api` are the workspace packages. They each
have their own `package.json`, version, and dependency list, but share
a single root.

## What v0.4 supports

v0.4 supports **discovery only**:

- Reading the `workspaces` field from the root `package.json`.
- Expanding glob patterns to find sub-package directories.
- Listing the discovered packages via `guroku workspaces`.

Inter-dependency linking — where `packages/api` depending on
`my-monorepo/util` resolves to the local source on disk instead of going
to the registry — is **not** in v0.4. That lands in v0.5.

## `guroku workspaces`

Run from the root of a monorepo to print the discovered list:

```sh
$ guroku workspaces
found 2 workspace package(s):
  @my/util@0.1.0  (packages/util)
  @my/api@0.1.0   (packages/api)
```

Each line shows the package's declared `name@version` and the path
relative to the root. If no workspaces are configured, or no packages
match the glob, the command prints `found 0 workspace package(s):` and
exits successfully.

## Glob patterns supported

Two declaration styles are recognized:

**npm-style array:**

```json
{
  "workspaces": ["packages/*"]
}
```

**pnpm-style object:**

```json
{
  "workspaces": {
    "packages": ["packages/*"]
  }
}
```

Both forms accept the same glob syntax: `*` matches a single path
segment, and entries may be specific paths (no glob characters at all).
Patterns are evaluated relative to the root `package.json`.

## Common monorepo layouts

A few patterns cover most real-world setups.

**Single packages directory** (most common):

```json
{
  "workspaces": ["packages/*"]
}
```

**Apps and libs split** (multi-glob):

```json
{
  "workspaces": ["apps/*", "libs/*"]
}
```

**Specific paths** (no globs):

```json
{
  "workspaces": ["frontend", "backend"]
}
```

You can mix globs and specific paths in the same array.

## Limitations in v0.4

The following are known gaps. Each is tracked for a later release.

- **No inter-package linking.** Declaring `"@my/util": "workspace:*"`
  in `packages/api`'s dependencies does not resolve to the local
  `packages/util` source. v0.4 will treat it as an unresolvable
  specifier. (Tracked for v0.5.)
- **No per-workspace `guroku run`.** There is no way to run a script
  in every workspace at once. You must `cd` into each package
  manually. (Tracked for v0.4.x.)
- **No workspace-aware `guroku add`.** Adding a dependency to a
  specific workspace package via a `--workspace=<name>` flag is not
  yet implemented. For now, edit the workspace's `package.json` by
  hand and re-run `guroku install` from the root. (Tracked for
  v0.4.x.)

## Recommended migration path

If you are coming from npm, pnpm, or yarn workspaces, you can keep
your existing `workspaces` field as-is. guroku will discover the same
set of packages. The one thing you lose until v0.5 is automatic
inter-dependency linking — anything declared with `workspace:*` or
that depended on the old package manager linking sibling packages by
name will not work yet.

A safe staged migration looks like:

1. Run `guroku workspaces` to confirm discovery matches your existing
   tool's output.
2. Continue using your previous package manager for installs that
   rely on inter-dep linking.
3. Switch fully to guroku once v0.5 ships.

## FAQ

**Will my CI break if I switch to guroku v0.4?**

If your CI relied on `guroku install` (or your previous tool's
install) linking workspace packages by name, yes — until v0.5 you
will need to either install each workspace package separately, or
keep using a different package manager for the inter-dep linking
step. Document this clearly in your project's README so contributors
do not get tripped up.

**Does Lerna integration exist?**

No. guroku reads the `workspaces` field directly and does not
consult `lerna.json` or any other Lerna config. You can keep Lerna
around for publishing or versioning if you want; guroku just
ignores it.

**Is there a workspace lockfile?**

Today the root project has a single `guroku.lock`; that is the only
lockfile. Per-workspace lockfiles are not planned. The root lockfile
already captures the resolved tree for every workspace package, so
splitting it would add complexity without buying anything.

## Related docs

- [`docs/internals/workspaces.md`](internals/workspaces.md) —
  implementation details: how discovery hooks into the resolver,
  glob expansion internals, and the v0.5 linking design.
- [`examples/monorepo/README.md`](../examples/monorepo/README.md) —
  a runnable example monorepo that exercises the v0.4 discovery
  path.
