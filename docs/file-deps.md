# `file:` Dependencies

guroku supports `file:` protocol dependencies for pulling in a package from a
local path on disk. This is useful for testing an unpublished library against
a downstream consumer, or for monorepo-adjacent setups that pre-date proper
workspace support.

This page describes how `file:` deps work in guroku v0.5.

## 1. Spec form

In your `package.json`, declare a `file:` dep like any other:

```json
{
  "name": "my-app",
  "version": "1.0.0",
  "dependencies": {
    "my-local": "file:./relative/path",
    "other-local": "file:/absolute/path/to/pkg"
  }
}
```

Both relative and absolute paths are accepted. Relative paths are resolved
**relative to the consuming `package.json`**, not relative to your shell's
current working directory. This matches npm/yarn behavior and avoids surprises
when running `guroku install` from a subdirectory.

## 2. What it does

When guroku resolves a `file:` dep, it:

1. Reads the local package's `package.json` directly off disk.
2. Hardlinks each file in the package into the content-addressed store at
   `node_modules/.guroku/<name>@<version>/...`.
3. Surfaces a top-level `node_modules/<name>` symlink, exactly like every
   registry-installed dep.

The on-disk layout looks identical to a regular install:

```
node_modules/
  my-local -> .guroku/my-local@0.0.0-local/node_modules/my-local
  .guroku/
    my-local@0.0.0-local/
      node_modules/
        my-local/
          package.json   (hardlink to ../../../../path/package.json)
          index.js       (hardlink to ../../../../path/index.js)
```

This means tools that walk `node_modules` see `my-local` as a normal package.
No special-casing required in your bundler, test runner, or type checker.

## 3. Editing the source

Because guroku uses **hardlinks**, editing the source file modifies the same
on-disk bytes that the linked file points to. Changes are visible through
`node_modules/<name>` immediately, with no reinstall.

There is one important caveat: most editors that perform an "atomic write"
(write to a temp file, then `mv` it into place) **break the hardlink**. After
such a save, the source path points to a new inode, while the file inside
`node_modules/.guroku/...` still points to the old inode. Subsequent reads
through `node_modules/<name>` see the OLD content.

Editors known to do atomic writes by default:

- VS Code (configurable via `files.useAtomicWrite`)
- Vim with `:set backupcopy=auto` (the default on most platforms)
- Most JetBrains IDEs

Workarounds:

- Re-run `guroku install` after big editor sessions. This rebuilds the
  hardlinks against current source inodes.
- Configure your editor to write in-place (e.g. `:set backupcopy=yes` for
  Vim). This preserves the hardlink but loses the crash-safety benefit of
  atomic writes.
- Wait for `link:` protocol support, planned for v0.5.x. `link:` uses
  symlinks instead of hardlinks and is immune to this problem (at the cost
  of the strict-layout guarantees that hardlinks provide).

## 4. Versioning

`file:` deps are not pinned to a registry version. The lockfile records:

- A synthetic version of `0.0.0-local`, or whatever the local manifest says
  in its `version` field, if present.
- A placeholder `resolved` URL of the form `file:./relative/path` (the
  original spec, normalized).
- No `integrity` field, since hardlinked files have no stable hash across
  edits.

```json
{
  "node_modules/my-local": {
    "version": "0.0.0-local",
    "resolved": "file:./relative/path",
    "link": false
  }
}
```

Reproducibility for `file:` deps relies on the `file:` spec staying the same
in `package.json` and the source tree being present at the expected path.
There is no integrity check; if you need that, publish the package to a
registry (or a private one) and depend on it normally.

## 5. CI considerations

`file:` deps work in CI as long as the path resolves on the CI machine. For
most setups, this means the source tree must be checked into the same repo
as the consumer, at the path the `file:` spec expects.

For repos where you have an internal library and a consumer in the same
checkout, **prefer workspaces (v0.5+) over `file:` deps**. Workspaces give
you:

- Proper version resolution against `workspace:*` specs.
- Single-pass install across all packages.
- Topological build ordering.
- A real lockfile entry for each workspace package.

Use `file:` deps when the source tree lives outside the consumer repo (e.g.
a sibling clone in a developer's home directory), or for one-off local
overrides during debugging.

## 6. Common errors

### `file dependency at './pkg' has no readable package.json`

The path you pointed at doesn't contain a `package.json`, or guroku can't
read it. Check the relative path; remember it's relative to the consuming
`package.json`, not your shell's cwd. A quick sanity check:

```sh
cat ./relative/path/package.json
```

If that fails from the directory containing the consuming `package.json`,
guroku will fail too.

### `failed to parse <path>/package.json: <reason>`

Your local package's manifest is invalid JSON. Open the file, fix the syntax
error, and re-run `guroku install`. Common causes:

- Trailing commas (JSON does not allow them).
- Unquoted keys.
- Comments (JSON does not allow them; use `package.json5` is **not**
  supported).

### `file dependency cycle detected (informational)`

This is a notice, not a failure. See section 7.

## 7. Cyclic file: deps

It is possible to construct a cycle: A's `package.json` declares B via
`file:`, and B's `package.json` declares A via `file:`. guroku tolerates
this. The resolver uses a "sticky-first" rule: the first time a package is
seen during resolution, its identity is locked in, and subsequent encounters
through other paths reuse that resolution rather than recursing again.

Concretely:

```
my-app -> file:../A
A      -> file:../B (via A/package.json)
B      -> file:../A (via B/package.json)  <- cycle
```

guroku resolves A once, resolves B once, and notes the back-edge from B to
A as already-resolved. No infinite loop, no error. Both A and B end up in
`node_modules/.guroku/` and are hardlinked into each other's nested
`node_modules` as needed.

If this happens unintentionally, it usually means two packages should be
merged or one should depend on a published version of the other. guroku
prints an informational notice when it detects a cycle so you can audit it.

## 8. What v0.5 doesn't yet support

- `link:./path` — yarn's symlink-based protocol. Planned for v0.5.x,
  primarily as an escape hatch for the atomic-write editor problem
  (section 3).
- `--prefer-frozen-lockfile` semantics for `file:` deps. Today, `file:` deps
  are always re-resolved from disk on install, regardless of the
  `--frozen-lockfile` flag. This will tighten up in v0.6.
- Auto-detecting that a `file:` dep's source tree moved or was deleted.
  guroku will currently report a vague resolution error; better diagnostics
  are coming.
- Hash-based integrity for `file:` deps. There is no `integrity` field in
  the lockfile and no verification on subsequent installs.

## 9. Comparison with other package managers

| Manager  | `file:` strategy            |
| -------- | --------------------------- |
| npm      | Copies the package contents |
| pnpm     | Symlinks (via `link:`)      |
| yarn     | Copies the package contents |
| guroku   | Hardlinks                   |

guroku's choice of hardlinks is consistent with the rest of its strict
layout: every package in `node_modules` is a hardlink into the
content-addressed store under `.guroku/`. `file:` deps are not a special
case at the storage layer; they're just packages whose source happens to
live outside the store.

The trade-offs:

- **Copies (npm, yarn)** — durable across editor atomic writes, but disk
  usage doubles and edits don't show up without a reinstall.
- **Symlinks (pnpm)** — edits show up live and survive atomic writes, but
  some tools resolve through the symlink and end up reading files outside
  `node_modules`, which can break strict module-resolution assumptions.
- **Hardlinks (guroku)** — edits show up live, disk usage is single-copy,
  strict layout is preserved, but atomic-write editors can desync the link.

If the atomic-write issue bites you regularly, the recommended fix is to
either (a) wait for `link:` support in v0.5.x, or (b) configure your editor
to write in-place. Both are noted in section 3.
