# cas-shared

A walkthrough demonstrating content-addressable storage (CAS) deduplication
across multiple independent projects on the same machine.

This directory intentionally contains no `package.json`. The example operates
on two scratch projects under `/tmp` so you can see the global CAS at work
without polluting any real workspace.

## What this example shows

When two different projects depend on the same package at the same version,
guroku stores the package bytes exactly once in the per-user CAS at
`~/.guroku/cas`. Each project's `node_modules` then references those bytes
via hardlinks (or symlinks into the CAS-backed store), so the on-disk cost
of the second project is essentially zero.

The visible consequences:

- The package files in both projects share inode numbers.
- The aggregate `du` of two `node_modules` trees is much smaller than the
  sum of two independent installs would be.
- The CAS directory contains a single canonical copy of each unique file.

## Setup

Create two unrelated scratch projects that happen to depend on the same
version of the same package.

```sh
mkdir -p /tmp/proj-a /tmp/proj-b
echo '{"name":"a","version":"0.1.0","dependencies":{"lodash":"4.17.21"}}' > /tmp/proj-a/package.json
echo '{"name":"b","version":"0.1.0","dependencies":{"lodash":"4.17.21"}}' > /tmp/proj-b/package.json
```

## Install in both

Run guroku against each project. The first install populates the CAS; the
second should be noticeably faster because the bytes are already on disk.

```sh
guroku install --cwd /tmp/proj-a
guroku install --cwd /tmp/proj-b
```

## Inspect inode sharing

The same `lodash@4.17.21` file in both projects should resolve to the same
inode number, proving that they are hardlinks to the same underlying bytes.

```sh
stat -f '%i' /tmp/proj-a/node_modules/.guroku/lodash@4.17.21/node_modules/lodash/package.json
stat -f '%i' /tmp/proj-b/node_modules/.guroku/lodash@4.17.21/node_modules/lodash/package.json
# Same inode number on both.
```

On Linux, `stat` uses different flags:

```sh
stat -c '%i' /tmp/proj-a/node_modules/.guroku/lodash@4.17.21/node_modules/lodash/package.json
stat -c '%i' /tmp/proj-b/node_modules/.guroku/lodash@4.17.21/node_modules/lodash/package.json
```

## Inspect the CAS entry

The CAS is laid out as `~/.guroku/cas/<2-char-prefix>/<rest-of-hash>`. List
the top level and then drill into one of the prefix shards.

```sh
ls -la ~/.guroku/cas | head
ls -la ~/.guroku/cas/<one-of-the-prefixes> | head
```

You should see a flat collection of content-addressed blobs, each named by
its hash. There is no per-package directory at this layer; dedup happens at
the package-tarball granularity in v0.3.

## Disk math

Compare the size of each project's `node_modules` against the size of the
CAS itself.

```sh
du -shx /tmp/proj-a /tmp/proj-b
du -shx ~/.guroku/cas
```

Two projects' `node_modules` directories should be tiny (mostly symlinks
and small metadata files); the CAS holds the real bytes exactly once. As
you add more projects sharing the same dependencies, only the symlink
overhead grows.

## Why this matters

For monorepos with many packages, multi-checkout development setups (e.g.
several worktrees of the same repository), and CI runners that reuse a
warm cache between jobs, the savings compound. A laptop with a dozen
checkouts of services that all pull in `react`, `typescript`, and a few
hundred transitive deps pays the disk cost once instead of a dozen times.

## What v0.3 does NOT yet do

- Share the CAS across users on the same machine. Today the store lives
  under `~/.guroku/cas`, scoped to the invoking user.
- Mount the CAS on a network filesystem for shared use across hosts.
- Dedup at the per-file level (pnpm v3 style). v0.3 dedups at the
  package-tarball level; two packages that contain identical files still
  store those files independently.

All three are planned for later releases.

## Cleanup

```sh
rm -rf /tmp/proj-a /tmp/proj-b
```

The CAS at `~/.guroku/cas` is left intact; it is shared global state and
will be reused by future installs.
