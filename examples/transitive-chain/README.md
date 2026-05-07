# transitive-chain

A minimal example that exercises guroku v0.2's transitive dependency resolution.

## What this example shows

This example declares a single top-level dependency, `is-odd`, and demonstrates
how guroku v0.2 follows the dependency edges in each package's metadata to
also install `is-number` (a transitive dependency of `is-odd`).

The point is to show that you only have to list what you directly depend on.
guroku walks the graph for you, picks compatible versions for each indirect
dependency, and records every resolved version in the lockfile.

## Try it

```sh
cd examples/transitive-chain
rm -rf node_modules guroku.lock
guroku install
ls node_modules
```

Expected: `node_modules/` contains both `is-odd/` and `is-number/` directories,
even though only `is-odd` appears in `package.json`.

## What just happened

- guroku reads `package.json` and sees the requirement `is-odd@^3.0.0`.
- It fetches `https://registry.npmjs.org/is-odd` metadata and picks `3.0.1`,
  the highest version that satisfies `^3.0.0`.
- That version's metadata declares `dependencies: { "is-number": "^6" }`.
- guroku enqueues `is-number@^6`, fetches its metadata, and picks `6.0.0`.
- Both tarballs are downloaded, integrity-verified against the `integrity`
  field from the registry, and extracted into the content-addressable store.
- Both packages are then materialized into `node_modules/` from the store.
- Both end up in `guroku.lock` with their resolved exact versions, so
  subsequent installs are deterministic.

## What's in the lockfile

After a successful install, `guroku.lock` looks roughly like this:

```json
{
  "lockfileVersion": 1,
  "packages": {
    "is-odd@3.0.1": {
      "resolved": "https://registry.npmjs.org/is-odd/-/is-odd-3.0.1.tgz",
      "integrity": "sha512-...",
      "dependencies": {
        "is-number": "6.0.0"
      }
    },
    "is-number@6.0.0": {
      "resolved": "https://registry.npmjs.org/is-number/-/is-number-6.0.0.tgz",
      "integrity": "sha512-...",
      "dependencies": {}
    }
  }
}
```

Notice that the `dependencies` map under `is-odd@3.0.1` points at the exact
resolved version `6.0.0`, not the original `^6` range. The range lives in the
registry metadata; the lockfile records the decision guroku made.

## Why v0.2 installs transitive `dependencies` but not `peerDependencies`

guroku v0.2 walks `dependencies` (and only `dependencies`) when building the
install graph. `peerDependencies` are intentionally ignored in this version:
they require a different resolution model (the consumer is responsible for
providing them, not the package itself), and v0.2 does not yet implement
peer satisfaction checks or warnings.

For the full rationale and the planned behavior in later versions, see
[../../docs/peer-dependencies.md](../../docs/peer-dependencies.md).

## Related

- [../diamond-deps/](../diamond-deps/) shows the multi-path case, where two
  top-level dependencies both pull in the same transitive package and guroku
  has to pick a single version that satisfies both ranges.
