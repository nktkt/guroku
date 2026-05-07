# Sample project

A tiny `package.json` that lets you exercise `guroku install` end to end
against the public npm registry. It pins three small, well-known packages
so the run is fast, deterministic, and easy to inspect.

## Run it

```sh
cd examples/sample-project
guroku install
ls node_modules/
```

Expected output under `node_modules/`:

```
is-odd
lodash
ms
```

If you see those three directories, the install pipeline worked.

## What it tests

This project exercises the full v0.1 install pipeline:

1. Registry metadata fetch — resolves each name+range against
   `https://registry.npmjs.org/` and picks a concrete version.
2. Tarball download — fetches the `.tgz` for each resolved version.
3. SHA-512 verification — checks the downloaded tarball against the
   `dist.integrity` field from the registry response.
4. Store extraction — unpacks the tarball into the content-addressed
   store on disk.
5. Flat `node_modules` linking — links the extracted package directory
   into the project's `node_modules/`.

Three packages were chosen because they are tiny and (mostly) have no
transitive dependencies: `lodash` and `ms` are zero-dep, which keeps the
test surface minimal.

### Known limitation

`is-odd` declares a runtime dependency on `is-number`. v0.1 of guroku
does **not** yet follow transitive dependencies, so `is-number` will
**not** appear under `node_modules/`. Requiring `is-odd` at runtime
will therefore fail until transitive resolution lands. This is expected
for the v0.1 milestone and is the next thing on the roadmap.

## Notes

- See the top-level `CHANGELOG.md` for what shipped in v0.1 and what is
  planned next.
- See the top-level `ARCHITECTURE.md` for a walkthrough of the resolver,
  fetcher, store, and linker that this sample exercises.
