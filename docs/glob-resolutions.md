# Glob resolutions: `**/<name>` keys in `package.json#resolutions`

This document describes how guroku v1.1 handles yarn-style glob keys
of the form `**/<name>` inside the `resolutions` field of a
`package.json`. It covers the syntax that guroku accepts today, how
those entries interact with the flat `overrides` field, and what is
explicitly out of scope for v1.1.

## 1. What it is

`**/<name>` is yarn classic's syntax for "force this leaf package name
to a specific version everywhere it appears in the dependency graph,
regardless of which parent dragged it in."

It is the dependency-graph equivalent of saying:

> No matter who asks for `lodash`, they all get exactly version
> `4.17.21`. There is one `lodash` in this project, and that is its
> version.

guroku reads these entries from `package.json#resolutions` and applies
them during dependency resolution, the same way yarn classic does.
The end result is that the resolver collapses every constraint on
`<name>` down to the single pinned version, and the lockfile records
that one version.

## 2. Syntax

The literal form supported by v1.1 is:

```json
{
  "resolutions": {
    "**/<name>": "<version>"
  }
}
```

`<name>` is a single npm package identifier. It may be unscoped
(`lodash`) or scoped (`@types/node`, `@scope/pkg`). `<version>` is any
version string the resolver would accept anywhere else: an exact
version, a range, a tag, or a git/file specifier.

A typical block:

```json
{
  "resolutions": {
    "**/lodash": "4.17.21",
    "**/@types/node": "20.10.0",
    "**/minimist": "^1.2.8"
  }
}
```

You can mix glob keys with non-glob keys. The non-glob keys are
matched first (see "Match precedence" below):

```json
{
  "resolutions": {
    "lodash": "4.17.21",
    "**/lodash": "4.17.20",
    "a > b > lodash": "4.17.19"
  }
}
```

In that example, the path-scoped key wins for the `a > b > lodash`
edge, the flat `lodash` key wins for every other lodash edge, and the
glob key is shadowed entirely (it would only kick in if there were no
flat `lodash` entry).

## 3. What v1.1 supports

v1.1 supports exactly the literal `**/<name>` form, where `<name>` is
a single package identifier (unscoped or scoped). Concretely:

- `**/lodash` matches any node in the dep graph whose package name is
  `lodash`.
- `**/@types/node` matches any node whose package name is
  `@types/node`.

Anything more elaborate parses (so it does not error out your
`package.json`) but does not match anything during resolution:

- `pkg/**/foo` — path-prefixed glob. Parses, never matches.
- `*-helper` — within-name wildcards. Parses, never matches.
- `**/{a,b}` — brace expansion. Parses, never matches.
- `**/foo/**` — trailing wildcards after the name. Parses, never
  matches.

If any of those forms appear in `resolutions`, guroku will log a
single warning per pattern at resolve time noting that the entry was
recognised but is unsupported in this version, then move on. The rest
of the file resolves normally.

## 4. Comparison with flat overrides

`**/lodash` in `resolutions` and `lodash` (without the `**/`) in
`overrides` are functionally similar today. Both pin every occurrence
of `lodash` in the dependency graph to a single version. In v1.1
they produce the same lockfile.

There are still reasons to choose one over the other:

- **Tool compatibility.** npm only reads `overrides`. Yarn classic
  only reads `resolutions`. guroku reads both. If you want your
  `package.json` to pin a version under all three tools, put a flat
  entry in `overrides` and a glob entry in `resolutions`.
- **Pattern shape.** `**/lodash` is a glob. `lodash` (in `overrides`)
  is a literal flat name. They share the same effect today, but the
  glob form leaves room for richer matching in the future without
  changing the syntax surface.
- **Convention.** `resolutions` historically carries glob-shaped
  forces; `overrides` historically carries flat or path-scoped
  forces. Following that split keeps your `package.json` legible to
  anyone who already knows the upstream conventions.

If you are pinning a transitive dep purely to clamp a vulnerability
and do not care which field it lives in, `overrides` (flat) is the
most portable choice. If you want a yarn-shaped file, use
`resolutions` with `**/<name>`.

## 5. Match precedence

When the resolver needs a version for a node at a given path in the
graph, it consults `lookup_with_path` in this order. The first
matching key wins:

1. **Exact-path key in `overrides`.** Example:
   `"a > b > lodash": "4.17.21"`. Matches only the `lodash` edge
   reached through `a > b`.
2. **Flat name in `overrides`.** Example: `"lodash": "4.17.21"`.
   Matches any `lodash` edge.
3. **Exact-path key in `resolutions`.** Example: `"a/b/lodash"` or
   `"a > b > lodash"`. Matches only that path.
4. **Flat name in `resolutions`.** Example: `"lodash": "4.17.21"`.
   Matches any `lodash` edge.
5. **Glob `**/<name>` in `resolutions`.** Example: `"**/lodash"`.
   Matches any `lodash` edge.

Glob is the lowest-priority match. That means a more specific entry
anywhere in `overrides` or `resolutions` will shadow a `**/<name>`
glob for the edges it covers, and the glob only takes effect for
edges that nothing more specific matched.

If two glob entries with the same `<name>` are present (for example,
because of a workspace-merging mistake), the one declared latest in
the merged `package.json` wins, and a warning is logged.

## 6. Why globs are in `resolutions` only

guroku's split is deliberate:

- **Yarn classic** uses globs in `resolutions`. That is the upstream
  convention for the field.
- **npm** does not use globs at all. It uses literal names and nested
  objects in `overrides`.
- **pnpm** supports per-path overrides via the `>` form, but it does
  not support yarn-shaped globs.

Putting glob support only in `resolutions` keeps each field's
semantics aligned with the tool that originated it. `overrides`
behaves like npm/pnpm overrides; `resolutions` behaves like yarn
classic resolutions. You do not have to remember a guroku-specific
rule on top of the upstream conventions.

A user who wants the same effect under all three tools writes the
override twice — once flat in `overrides`, once globbed in
`resolutions`:

```json
{
  "overrides": {
    "lodash": "4.17.21"
  },
  "resolutions": {
    "**/lodash": "4.17.21"
  }
}
```

Under guroku, the flat `overrides` entry wins by precedence, so the
two stay in sync as long as you keep them aligned manually.

## 7. Compatibility

How v1.1's behaviour relates to other package managers:

- **yarn classic.** Full glob support in `resolutions`, including
  path-prefixed forms and brace expansion. v1.1 only honours the
  simple `**/<name>` form. A `package.json` written for yarn classic
  will load under guroku, but any non-`**/<name>` glob in
  `resolutions` will simply not apply.
- **npm.** Does not support globs at all. If you need npm
  portability, also add a flat entry under `overrides` with the same
  version. guroku will read both and the `overrides` entry wins by
  precedence, keeping the two consistent.
- **pnpm.** Supports per-path overrides (the `>` form) but not yarn
  globs. If pnpm portability matters, prefer per-path overrides for
  the targeted edges and use the glob only for the broad pin.

In short: `**/<name>` is the most portable glob form across yarn
classic and guroku, and the only form guroku v1.1 recognises. For
npm/pnpm interop, fall back to flat or per-path entries in
`overrides`.

## 8. Worked example: clamping a transitive vuln across a workspace

Suppose `minimist@<1.2.6` has a known vulnerability and you have a
workspace with several packages, some of which transitively depend on
older minimist via packages you do not control.

Add a glob resolution at the workspace root:

```json
{
  "resolutions": {
    "**/minimist": "1.2.8"
  }
}
```

Run guroku:

```sh
guroku install
```

After this, every node in the dep graph whose name is `minimist` is
pinned to `1.2.8`, even if a transitive parent declared
`"minimist": "^1.2.0"` in its own `package.json`. The pin applies
across every workspace member, because `resolutions` at the root is
visible to all of them.

If a particular workspace member needs a different minimist (say,
because it ships its own bundled tooling), pin that one specifically
with a higher-precedence override:

```json
{
  "overrides": {
    "tooling-pkg > minimist": "1.2.6"
  },
  "resolutions": {
    "**/minimist": "1.2.8"
  }
}
```

The path-scoped `overrides` entry wins for `tooling-pkg`'s edge; the
glob handles everything else.

## 9. What v1.1 doesn't yet do

The following forms are recognised by the parser (so they will not
break your `package.json`) but do not match anything during
resolution. They are tracked on the v1.x backlog:

- `pkg/**/foo` — path-prefixed glob. ("Pin `foo` only when it appears
  under `pkg`.")
- `*-helper` — within-name wildcards. ("Pin every package whose name
  ends in `-helper`.")
- Brace expansion, e.g. `**/{lodash,underscore}`.
- Trailing wildcards after the name, e.g. `**/foo/**`.

If you have a use case that needs one of these, file an issue against
the v1.x milestone with the shape of the pattern and the resolver
behaviour you want.

## 10. Verifying it took effect

After running `guroku install`, inspect the lockfile to confirm every
occurrence of the targeted package is pinned:

```sh
cat guroku.lock | grep '"minimist@'
# all entries should show the pinned version.
```

If you see two different versions in the output, one of two things is
true:

- A higher-precedence entry (exact-path or flat in `overrides`, or a
  more-specific entry in `resolutions`) is shadowing the glob for some
  edges. Audit those entries and remove or align them.
- A non-`**/<name>` glob form was used and silently did not apply.
  Check the resolve-time warnings in `guroku install --verbose` for
  any "unsupported glob pattern" lines.

For a workspace, repeat the check from the workspace root; the root
lockfile records resolutions for every member.

You can also dump the resolved version directly:

```sh
guroku why minimist
```

That prints every node in the graph that depends on `minimist`,
together with the version each one resolved to. With a working
`**/minimist` glob, every line should show the same pinned version.
