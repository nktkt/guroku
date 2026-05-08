# with-overrides

A minimal guroku example that uses `package.json#overrides` to pin a
transitive dependency to a specific version.

## What this example shows

This package directly depends on `is-odd@^3`. That range pulls in
`is-number` as a transitive dependency. Without intervention the
resolver would pick whatever `is-number` version best satisfies the
constraint declared by `is-odd`.

The `overrides` block in `package.json` forces `is-number` to resolve
to exactly `6.0.0`, regardless of what the parent asks for. This is the
canonical use of `overrides`: pinning a transitive dep you do not
directly depend on.

```json
{
  "dependencies": {
    "is-odd": "^3"
  },
  "overrides": {
    "is-number": "6.0.0"
  }
}
```

## Try it

```sh
cd examples/with-overrides
rm -rf node_modules guroku.lock
guroku install
```

A clean install ensures nothing is reused from a previous lockfile.

## Verify the override applied

```sh
cat guroku.lock | grep '"is-number@'
# Should show is-number@6.0.0 (or whatever the override pinned).
```

If the override took effect you should see exactly one entry, pinned to
`6.0.0`. If you see a different version, the override was not honored,
which is a bug worth filing.

## Without the override

Delete the `"overrides"` block from `package.json` and re-install:

```sh
rm -rf node_modules guroku.lock
guroku install
cat guroku.lock | grep '"is-number@'
```

The resolver now picks the highest 6.x that `is-odd@^3.0.0` accepts.
With the override in place it gets pinned to `6.0.0` specifically. The
difference is small here but the mechanism is the same one you would
use for security patches in real trees.

## Why use overrides

Three common cases motivate overrides:

- **Security**: patch a transitive vulnerability before the parent
  package ships a fix. You bump the bad dep yourself instead of
  waiting on the maintainer.
- **Compat**: force a single version of a duplicated transitive when
  two parents pull in incompatible ranges and the duplicates break at
  runtime (singletons, peer state, etc.).
- **Testing**: try `react@next` ahead of dep tree adoption, or pin a
  pre-release across the graph to validate a migration before all
  parents update their ranges.

## `overrides` vs `resolutions`

Same idea, different names.

- npm 8+ uses `overrides`.
- yarn classic uses `resolutions`.

guroku reads both. If a package declares both blocks, `overrides`
wins on conflict. Pick one and stick with it; mixing them in a single
`package.json` is a smell.

## Limitations in v0.5

Only the simple flat-name to version form is supported:

```json
{ "overrides": { "is-number": "6.0.0" } }
```

The following forms are recognized in the schema but not matched by
the resolver yet:

- Path-keyed: `"foo > bar"` to override `bar` only when reached
  through `foo`.
- Glob keys: `"**/foo"` and similar patterns.

If you need either, watch the tracking issue or pin the offending dep
at the top level as a workaround.

## Related docs

- `docs/overrides.md` — user-facing guide and full syntax reference.
- `docs/internals/overrides.md` — how the resolver applies overrides
  during graph construction.
