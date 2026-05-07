# with-lockfile

An example project that demonstrates using guroku with a committed
`guroku.lock`.

## What this example shows

This example shows how to use `guroku.lock` to get reproducible installs,
and how to use the `--frozen-lockfile` flag to enforce that reproducibility
in CI. When a lockfile is committed to the repo, every developer (and every
CI run) resolves to the same set of versions, regardless of when they run
`guroku install`.

## Files

Three things live in this directory:

- `package.json` — declares dependency ranges (`^4.17.0`, etc.).
- `guroku.lock` — the committed lockfile pinning resolved versions.
- `README.md` — this file.

## Try it

```sh
cd examples/with-lockfile
rm -rf node_modules
guroku install --frozen-lockfile
```

The `--frozen-lockfile` flag asks guroku to install exactly what the
lockfile says, refusing to refresh. This is the recommended CI invocation.

## What if I want to refresh?

Drop the flag:

```sh
guroku install
```

This re-resolves and rewrites `guroku.lock`. Re-add the flag once your
changes are committed.

## How to detect lockfile drift in CI

Your CI step should fail if the lockfile would have to change to match the
manifest. `guroku install --frozen-lockfile` does that for you: if a
dependency range in `package.json` no longer matches the resolved version
in `guroku.lock`, the install aborts with a non-zero exit code instead of
silently bringing in a new version.

This is the single check you need. You don't need a separate "verify
lockfile" step.

## Notes

- The lockfile in this example was generated against the public npm
  registry. If npm ever takes one of these tarballs down, the install will
  fail with an integrity error rather than silently substituting.
- This example's deps were chosen because they have NO transitive deps in
  v0.2's resolver. (Once peer/optional installation lands in v0.4, more
  interesting examples will follow.)

## Related docs

- [docs/lockfile-format.md](../../docs/lockfile-format.md) — the on-disk
  format of `guroku.lock`.
- [docs/cli-reference.md](../../docs/cli-reference.md) — full reference for
  `guroku install` and its flags, including `--frozen-lockfile`.
