# Path-keyed overrides

Path-keyed overrides let you pin a transitive dependency only when it is
reached through a specific chain of parent packages. They are the most
precise tool guroku ships for surgically fixing a single bad version
without disturbing the rest of the graph.

This document covers the syntax, when to reach for path-keyed overrides
versus simpler alternatives, the exact match semantics guroku uses, and
how to verify the result took effect in `guroku.lock`.

## What a path-keyed override is

A *flat* override pins every occurrence of a dependency in the graph.
If you write `"glob": "10.3.10"`, every package that depends on `glob`
ends up resolving to `10.3.10`, no matter who pulled it in.

A *path-keyed* override pins a dependency only when it is reached
through a particular parent chain. If you write
`"webpack-cli > glob": "10.3.10"`, the pin applies only to the `glob`
that hangs underneath `webpack-cli`. Any other package that depends on
`glob` continues to resolve normally.

The mental model is:

- Flat override: "this name resolves to this version, full stop."
- Path-keyed override: "this name resolves to this version, but only
  inside this particular subtree."

Path-keyed overrides exist because real graphs frequently disagree
about which version of a leaf dependency they want, and a single flat
pin is often too aggressive a hammer.

## Syntax

A path key is a string of package names separated by `>`:

```
"a > b > c": "1.0.0"
```

This pins package `c` to `1.0.0` when it is reached via `a -> b -> c`.

Rules:

- Each segment is a bare package name. Scoped names (`@scope/name`) are
  fine; the parser splits on `>` only.
- Whitespace around `>` is tolerated. `"a > b > c"`, `"a>b>c"`, and
  `"a >b> c"` all parse identically.
- The last segment is the dependency being pinned; all earlier segments
  describe the path to it.
- Path keys live in the same `overrides` block as flat keys. You can
  mix the two freely.

```json
{
  "overrides": {
    "left-pad": "1.3.0",
    "webpack-cli > glob": "10.3.10",
    "@my-org/cli > minimist": "1.2.8"
  }
}
```

## When to use path-keyed overrides

Three concrete situations where a flat override is the wrong tool:

1. **One transitive parent ships a buggy lock.** A library you depend
   on has pinned an old version of some leaf, and that old version has
   a bug. You want to bump it under that library only, without forcing
   every other consumer of the leaf to move. Pin via the parent chain.

2. **Different paths legitimately want different versions of the same
   dep.** Two parts of your tree need two versions of `glob` for real
   reasons (one needs the v7 callback API, one needs the v10 promise
   API). Use two path-keyed overrides to keep both intact instead of
   collapsing them with a flat pin.

3. **You want to express "fix only when this combination of parents is
   in play."** This shows up in monorepo split-brain situations, where
   one workspace pulls in a tool through path A and another workspace
   pulls in the same tool through path B. A path-keyed override lets
   you fix the broken combination without rewriting the working one.

If none of those describe your situation, reach for a flat override
first. Path-keyed overrides are precise but also more brittle: if the
intermediate parent renames or restructures, the key stops matching.

## Example

A common case: a webpack config is inflated because `webpack-cli`
pulls in an old `glob` that drags in old `inflight`, which leaks
memory. You want to bump `glob` under `webpack-cli` to `10.3.10`,
where the leak is gone, but you don't want to touch the `glob` that
your test runner depends on.

```json
{
  "overrides": {
    "webpack-cli > glob": "10.3.10"
  }
}
```

After `guroku install`:

- `webpack-cli`'s `glob` resolves to `10.3.10`.
- Any other package that depends on `glob` keeps whatever version its
  own range requested.
- `guroku.lock` records both versions side by side.

## Versus flat overrides

A flat override is the simpler form:

```json
{
  "overrides": {
    "glob": "10.3.10"
  }
}
```

This pins `glob` everywhere. Any package in the graph that asks for
any version of `glob` ends up with `10.3.10`.

Tradeoffs:

- Flat is simpler. One key, one version, applied uniformly.
- Flat is sometimes too aggressive. If a dependency genuinely needs
  the old API, a flat pin can break it.
- Path-keyed is narrower. It pins only the subtree you name.
- Path-keyed is harder to maintain. The key references intermediate
  package names; if those change, the key silently stops matching.

Use flat when you genuinely want the whole tree on one version. Use
path-keyed when you want to fix a specific subtree without collateral
damage.

## Versus glob resolutions

For backwards compatibility with yarn-style configs, guroku also
accepts globs in the `resolutions` block:

```json
{
  "resolutions": {
    "**/glob": "10.3.10"
  }
}
```

A glob like `**/glob` matches any leaf in the graph named `glob`,
regardless of how it was reached. This is *more* aggressive than a
flat override, because it bypasses the version constraint logic that
flat overrides still respect when picking a compatible version.

Specificity ranking, from broadest to narrowest:

1. Glob in `resolutions` (`**/glob`) - matches any leaf by name.
2. Flat in `overrides` (`glob`) - matches by name, with version-aware
   selection.
3. Path-keyed in `overrides` (`webpack-cli > glob`) - matches only
   when the named chain is present.

Path-keyed is the most specific. Reach for it when you need that
specificity and accept the maintenance cost.

## Match semantics

A path-keyed override matches a dependency at install time if the key
appears as a **contiguous suffix** of the resolution path. The
resolution path is the chain from the root project down to the
dependency being installed, recorded as a list of package names.

The contiguous part is important: the segments in the key must appear
in order, with no other packages between them.

Worked examples:

- Resolution path `["root", "webpack-cli", "glob"]`,
  key `"webpack-cli > glob"` -> match. The key is a contiguous
  suffix.

- Resolution path `["root", "webpack-cli", "log4js", "glob"]`,
  key `"webpack-cli > glob"` -> no match. `log4js` sits between
  `webpack-cli` and `glob`, breaking contiguity.

- Resolution path `["root", "webpack-cli", "log4js", "glob"]`,
  key `"log4js > glob"` -> match. The two-segment key is a contiguous
  suffix of the path.

In other words, the key describes the immediate parents of the pinned
dependency, not just any ancestor. If you want to match across an
unknown intermediate, you need a different tool (today, that means
flat or glob; see "What v1.1 doesn't yet support" below).

## Order of precedence

When multiple override forms could apply to the same resolution, the
following order wins (highest first):

1. Exact path in `overrides` - the most specific match in
   `overrides` by path key.
2. Flat name in `overrides` - flat override on the dep name.
3. Exact path in `resolutions` - same path syntax, but in
   the legacy `resolutions` block.
4. Flat name in `resolutions` - flat override on the dep name in
   `resolutions`.
5. Glob in `resolutions` - last-resort wildcard match.

If two path-keyed entries both match (for example, a longer key and a
shorter key both being valid suffixes), the longer key wins. This
keeps the precedence rule consistent: more specific beats less
specific at every layer.

## Verifying it took effect

After `guroku install`, the lockfile is the source of truth. To
quickly see which versions of a dep are present:

```sh
grep -A1 '"glob@' guroku.lock
```

Each entry records the resolved version under its package@range key.
For a path-keyed override, you should see the pinned version listed
under the relevant key, while other entries for the same dep keep
their own resolved versions.

For a more structured view, the install command also accepts a
`--why` flag:

```sh
guroku why glob
```

This prints every resolution path that ends in `glob`, along with the
selected version for each. If the path-keyed override took effect,
the path you named will show the pinned version; the others will
show whatever the regular resolver picked.

## What v1.1 doesn't yet support

Path keys in v1.1 are intentionally narrow. The following forms are
*not* supported and will be rejected with a parse error:

- **Wildcards within a path.** `"a > * > b"` is not yet legal. There
  is no way to say "any single intermediate parent."
- **Negation.** There is no way to say "pin `glob` everywhere except
  when reached through `webpack`."
- **Or-patterns.** `"a|b > c"` is not yet legal. If you want to apply
  the same pin under two different parents, write two separate keys.

These are tracked on the v1.x backlog. If you hit a real-world case
that needs one of them, file an issue with the specific dependency
graph - we use those reports to prioritize.

## Compatibility

Path-keyed override syntax is compatible with the major npm-family
package managers:

- **npm 8+**: same syntax, in the `overrides` field of `package.json`.
  Path keys are supported and behave identically.
- **pnpm 7+**: same syntax, in `pnpm.overrides` inside `package.json`.
  Path keys behave identically.
- **yarn classic (1.x)**: similar but different. Yarn uses globs in
  `resolutions` rather than path keys; the closest equivalent is a
  glob like `webpack-cli/**/glob`, which is broader. Yarn berry has
  its own resolution syntax; consult its docs.

When migrating between managers, flat and path-keyed entries port
cleanly between npm, pnpm, and guroku. Yarn classic configs need to
be rewritten into one of the other forms, because guroku does not
interpret yarn's slash-separated globs as path keys.

For broader migration guidance, see `docs/migration/`.
