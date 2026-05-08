# with-glob-resolution

## What this example shows

This example demonstrates yarn-style `**/<name>` glob keys in the
`resolutions` field of `package.json`. guroku v1.1 honours the simple
form of this glob, pinning every occurrence of a given leaf package
name to a specific version regardless of where it appears in the
dependency tree.

The package.json for this example:

```json
{
  "dependencies": { "is-odd": "^3.0.0" },
  "resolutions": { "**/is-number": "6.0.0" }
}
```

## Try it

```sh
cd examples/with-glob-resolution
rm -rf node_modules guroku.lock
guroku install
grep -A1 '"is-number@' guroku.lock
```

The lockfile shows `is-number@6.0.0`, even though `is-odd@^3.0.0`
would normally resolve `is-number` to a different version.

## What `**/<name>` matches

The `**/<name>` glob is a leaf-name match anywhere in the dep tree.
If `is-number` appears under `is-odd`, under `webpack`, under any
transitive chain whatsoever, every instance is pinned to `6.0.0`.

It does not matter:

- how deep the dependency is nested
- which parent package brings it in
- whether it is a direct or transitive dependency

The match is purely on the final leaf name.

## Compared with a flat override

Writing `"is-number": "6.0.0"` in `overrides` is functionally similar
to `"**/is-number": "6.0.0"` in `resolutions`. The differences are
mostly ecosystem conventions:

- npm reads `overrides` only; yarn reads `resolutions` only. guroku
  reads both, so the choice is a style decision.
- The `**/` prefix is yarn idiom. Use whichever your team or
  fellow tooling expects to see.

If you have a mixed toolchain, picking the form your other tools
recognise avoids surprises when someone runs `npm install` or
`yarn install` against the same `package.json`.

## Compared with a path-keyed override

A path-keyed override like `"is-odd > is-number": "6.0.0"` pins
`is-number` ONLY when reached through `is-odd`. If another part of
the tree also depends on `is-number`, that copy is unaffected.

The glob version `"**/is-number": "6.0.0"` pins every instance
everywhere. Use the path-keyed form when you need surgical control,
and the glob form when you want a blanket pin.

## Precedence

When an override could match a package via multiple paths, guroku
applies them in this order (highest priority first):

1. exact path in `overrides`
2. flat name in `overrides`
3. exact path in `resolutions`
4. flat name in `resolutions`
5. glob in `resolutions` (lowest-priority)

This means a more specific rule always wins. A `**/<name>` glob is a
fallback that only applies when no more specific override matches.

## What v1.1 doesn't yet support

The glob support in v1.1 is intentionally narrow. The following
yarn-style forms are NOT recognised:

- `pkg/**/foo` (path-prefixed glob)
- wildcards in the name itself, such as `*-helper`
- brace expansion, such as `{is-number,is-odd}`

If you write one of these, guroku v1.1 will warn and skip the entry
rather than guess at semantics.

## Compatibility

`**/<name>` is the most common yarn-classic form seen in the wild,
and the simple shape covers the vast majority of real-world uses.
v1.1 supports just that simple shape; richer globs are tracked on
the v1.x backlog and are expected to land in a follow-up release.

If you are migrating from yarn classic and your `resolutions` block
uses only `**/<name>` keys, no changes are needed for guroku.

## Related docs

- `docs/glob-resolutions.md`
- `docs/internals/glob-resolutions.md`
