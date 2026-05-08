# with-cascade-backtrack

## What this example shows

A small project that resolves cleanly under guroku v1.2's pubgrub-backed
resolver. The project is intentionally simple: the dependencies do not
actually trigger cascading backtracks, and the same manifest would resolve
under v1.1's BFS resolver too.

Treat this directory as the *shape* of a cascade scenario: the manifest
layout, the lockfile shape, the way the resolver is invoked. It exists so
you have a runnable starting point when you go to reproduce a real-world
cascade.

For real cascade reproductions (registry shapes that the BFS path could
not solve, but pubgrub can), point at the GitHub issue tracker. Those
fixtures live with the bug reports rather than being checked in here, so
they stay coupled to the version of the registry that produced them.

## Try it

```sh
cd examples/with-cascade-backtrack
rm -rf node_modules guroku.lock
guroku install                            # uses pubgrub
GUROKU_RESOLVER=bfs guroku install        # uses v1.1 BFS for comparison
```

Both invocations should succeed and produce equivalent lockfiles for this
example.

## What "cascading backtrack" means

Two related things, in order:

- **Diamond conflict.** Package `X` depends on `core@^1`, package `Y`
  depends on `core@^2`. If a single published version (say `core@1.5.0`)
  satisfies both ranges, both resolvers find it on the first try. If no
  such version exists, v1.1's BFS resolver gives up with a conflict
  error. The pubgrub resolver instead backtracks: it picks a different
  version of `X` (or `Y`) that brings in a different `core` range, and
  retries.

- **Cascade.** Backtracking `X` is rarely free. `X@1.4.0` and `X@1.3.0`
  often pull in different versions of `X`'s *other* dependencies, and
  those dependencies may already have been resolved against the old `X`.
  A cascading backtrack walks back through that subtree, undoing the
  decisions that were predicated on the now-rejected version of `X`.
  v1.1 does not do this transitively; pubgrub does, because the
  algorithm is built around incompatibility tracking that naturally
  records *why* each decision was made.

## What v1.2 actually delivers

v1.2 ships pubgrub-the-crate as the new default solver. Cascade
backtracking falls out of the algorithm itself, not from extra
guroku-side bookkeeping. If a registry state is solvable, pubgrub will
find a solution; if it is not, the failure report names the
incompatibilities involved instead of just the first conflict the BFS
walk happened to hit.

## What v1.2 does NOT deliver

- A macrobench harness comparing pubgrub vs BFS performance. We have
  microbenchmarks, but no end-to-end timing suite over realistic
  registry snapshots.
- Structural npm-Range to pubgrub-Range translation. The integration
  uses candidate-set translation: we enumerate the candidate versions
  for a package and feed those to pubgrub, rather than translating the
  semver range expression itself into pubgrub's range type.
- `file:` and `git:` roots inside the pubgrub path. Those still fall
  back to the BFS resolver, because their version identity is not a
  semver point that pubgrub can reason about.

## Comparing the two resolvers

Set `GUROKU_RESOLVER=bfs` to take the v1.1 path. For this example, both
resolvers should produce identical lockfiles, because the dependency
graph here does not require cascading. For projects that previously
errored out with BFS conflict messages, the default pubgrub path may now
succeed; that is the user-visible v1.2 win.

If you want to confirm parity locally, run each resolver into a fresh
`node_modules` and diff the resulting `guroku.lock` files:

```sh
rm -rf node_modules guroku.lock
guroku install
mv guroku.lock guroku.lock.pubgrub

rm -rf node_modules guroku.lock
GUROKU_RESOLVER=bfs guroku install
mv guroku.lock guroku.lock.bfs

diff guroku.lock.pubgrub guroku.lock.bfs
```

## What's in this directory

- `package.json` — the example manifest.
- `README.md` — this file.

A `package.json` for this example looks like:

```json
{
  "name": "with-cascade-backtrack",
  "version": "0.0.0",
  "private": true,
  "dependencies": {}
}
```

The exact dependency set is intentionally minimal; swap in the package
versions from a bug report when reproducing a real cascade.

## Related docs

- `docs/internals/pubgrub-integration.md`
- `docs/v1.2-release-notes.md`
