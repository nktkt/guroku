# with-path-override

A minimal guroku example demonstrating a **path-keyed override** that pins
a transitive dependency only when reached through a specific parent.

## What this example shows

The `package.json` in this directory declares:

```json
{
  "dependencies": { "is-odd": "^3.0.0" },
  "overrides": { "is-odd > is-number": "6.0.0" }
}
```

The override key `"is-odd > is-number"` is a **path-keyed override**: it
pins `is-number` to `6.0.0`, but only when the resolver reaches it as a
dependency of `is-odd`. If `is-number` were also pulled in by some other
top-level package, that other path would be unaffected.

Path-keyed overrides are a v1.1 feature. In v1.0, only flat keys
(`"is-number"`) were supported.

## Try it

```sh
cd examples/with-path-override
rm -rf node_modules guroku.lock
guroku install
```

After this, `node_modules/is-odd/node_modules/is-number` (or the flat
hoisted copy, depending on layout) should be at version 6.0.0.

## Verify it took effect

```sh
grep -A1 '"is-number@' guroku.lock
```

You should see `is-number@6.0.0` in the output. The path-keyed override
is what pinned it to that exact version; without the override, the
resolver would have picked whatever `is-odd@^3.0.0` happens to declare.

## Compare with a flat override

You can change the manifest to use a flat key instead:

```json
{
  "dependencies": { "is-odd": "^3.0.0" },
  "overrides": { "is-number": "6.0.0" }
}
```

For **this** example the resulting lockfile is identical, because
`is-number` is only reachable via `is-odd`. There is exactly one path,
so a flat override and a path-keyed override coincide.

The two forms diverge in projects where the same package is reachable
through multiple parents. Suppose `is-number` were pulled in by both
`is-odd` and some unrelated `math-utils`. A flat override pins both
copies; a path-keyed override pins only the copy reached through the
named path.

## What happens without the override

Drop the `overrides` block entirely and run `guroku install` again.
The resolver picks whatever version of `is-number` `is-odd@^3.0.0`
declares in its own `package.json` (in practice already a 6.x release).
The override here is not about substituting a major version; it is
about pinning a **specific patch** so the lockfile is reproducible
regardless of upstream churn.

## Format details

- The path separator is `>`, with names on either side.
- Whitespace around `>` is tolerated: `"is-odd>is-number"`,
  `"is-odd > is-number"`, and `"is-odd  >  is-number"` all parse the
  same way.
- Multiple steps work: `"a > b > c"` matches `c` only when reached as
  a dependency of `b`, which itself is reached as a dependency of `a`.
- The leftmost name is matched against direct dependencies of the
  current package; it is not anchored to the project root in any
  other way.

## What v1.1 doesn't yet support

The following extensions are recognised at parse time only as plain
strings and will not behave as path patterns:

- **Wildcards within a path**, e.g. `"a > * > b"`.
- **OR patterns**, e.g. `"a|b > c"`.
- **Negation**, e.g. `"!a > b"` to mean "any path except through `a`".

If you write any of these, guroku v1.1 treats the entire string as a
literal package name and the override silently fails to match. A future
version may add real support; for now, write out each path explicitly.

## Related docs

- `docs/path-keyed-overrides.md` — user-facing reference for the
  `overrides` field, including the path-keyed syntax.
- `docs/internals/path-keyed-overrides.md` — how the resolver matches
  paths during dependency walking, and how matches are recorded in the
  lockfile.
