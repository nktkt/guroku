# with-peer-deps

A guroku example showing how `peerDependencies` and `peerDependenciesMeta`
interact with `guroku install` in v0.2.

## What this example shows

This project declares peer dependencies (`react`, `react-dom`) alongside
regular runtime and dev dependencies. It demonstrates what guroku v0.2
actually does when it sees those fields:

- It reads and validates the `peerDependencies` block.
- It reads and validates `peerDependenciesMeta` (including the `optional`
  flag).
- It does **not** install peer dependencies. In v0.2, peers are the host
  project's responsibility.

The example exists to make that contract concrete: after `guroku install`,
you can inspect `node_modules` and the lockfile and see that peers were
parsed but not fetched.

## The package.json

The manifest has four sections worth calling out:

- `dependencies`: `ms` is a real runtime dependency. guroku resolves it,
  downloads it, and writes it into `node_modules/ms`.
- `peerDependencies`: `react` and `react-dom` are *declared* but
  intentionally not installed by guroku v0.2. The host project (anything
  that consumes this package) is expected to provide compatible versions.
- `peerDependenciesMeta.react-dom.optional`: marks `react-dom` as
  optional from this library's point of view. When auto-install for
  peers lands in v0.4, optional peers will be skipped by default; only
  required peers will be pulled in.
- `devDependencies`: `is-odd` is for our own development and tests. It
  is installed by `guroku install` (unless `--production` is passed in a
  future version).

## Try it

```sh
cd examples/with-peer-deps
rm -rf node_modules
guroku install
ls node_modules
```

After the install, `node_modules` should contain:

- `ms` — from `dependencies`
- `is-odd` — from `devDependencies`

You should **not** see `react` or `react-dom`. Those are peer
dependencies, and v0.2 does not install peers, even if one of them is
marked required.

## Why peers are not auto-installed in v0.2

Peer dependency resolution is intentionally deferred. Picking the right
version requires looking at the host project's own constraints, walking
the full dependency graph, and reporting conflicts in a useful way.
v0.2 ships parsing and validation only so that downstream tooling and
the lockfile format are stable before resolution lands.

For the full rationale and the planned semantics, see
[../../docs/peer-dependencies.md](../../docs/peer-dependencies.md).

## What changes when peers are installed

When peer auto-install lands in v0.4:

- `react` will appear in `node_modules` automatically, because it is a
  required peer.
- `react-dom` will be skipped by default, because
  `peerDependenciesMeta.react-dom.optional` is `true`. It will only be
  installed if the host project itself depends on it.
- If the host project pins a version of `react` that does not satisfy
  `^17.0.0 || ^18.0.0`, guroku will fail loudly with a peer conflict
  error rather than silently picking one.

Until then, the behavior in this example is the stable contract: peers
are parsed, validated, and recorded in diagnostics, but never written
to disk.

## Lockfile contents

After `guroku install`, inspect `guroku.lock`. You will see entries for
`ms` and `is-odd` (and their transitive dependencies, if any), but
**not** for `react` or `react-dom`. The lockfile only records resolved
regular and dev dependencies. Peer dependencies are not resolved in
v0.2, so there is nothing to lock.

This will continue to hold even after v0.4: peer entries themselves are
not lockfile records. What gets locked is whatever concrete package the
host project ends up installing to satisfy the peer constraint.
