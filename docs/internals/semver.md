# Version Specs in guroku

This document describes how guroku parses and matches version specifications.
It is aimed at contributors working on the resolver, the lockfile, or anything
that touches a `package.json` dependency string.

## Why "node-semver" is its own thing

guroku is an npm-style package manager, and npm uses its own flavour of
semver — commonly called *node-semver* — which is similar to but **not** the
same as the semver dialect used by crates.io and Cargo. Two of the most
visible differences:

- **Caret in `0.x` ranges.** Cargo's `^0.2.3` allows anything `>=0.2.3 <0.3.0`
  *and* allows `0.2.4`, `0.2.99`, etc. — but it also treats `^0.0.3` as
  exact-match-ish in a different way than npm does. The behaviour around the
  zero-major and zero-minor boundaries does not line up across the two
  ecosystems, and we need npm's behaviour, not Cargo's.
- **X-ranges and OR.** Specs like `1.2.x`, `1.x`, and `^1 || ^2` are
  first-class in node-semver. Cargo's `semver` crate does not parse them
  natively.

For these reasons guroku uses the [`node-semver`][node-semver-crate] Rust
crate, **not** Cargo's [`semver`][cargo-semver-crate] crate. Anywhere you see
a "version" or "range" type flowing through guroku, it comes from
node-semver.

## The wrapper module

All version handling is funnelled through `guroku::version`. That module
re-exports the two types we care about from `node-semver`:

```rust
pub use node_semver::{Range, Version};
```

and adds three helpers on top:

```rust
pub fn parse_range(name: &str, spec: &str) -> Result<Range, GurokuError>;
pub fn parse_version(s: &str) -> Result<Version, GurokuError>;
pub fn max_satisfying<'a>(versions: &'a [Version], range: &Range) -> Option<&'a Version>;
```

Code consuming the rest of the crate — the resolver, the registry client, the
lockfile reader/writer — should go through `guroku::version`. Do not import
`node_semver` directly in new code; if you do, the error types will not line
up with `GurokuError` and the caller will lose the package name in the error.

## Supported syntax

The following forms are accepted by `parse_range`. All examples assume a
package whose published versions are
`1.0.0, 1.2.0, 1.2.3, 1.2.3-beta.1, 1.5.0, 2.0.0`.

### Exact

```
1.2.3
```

Matches only `1.2.3`. Equivalent to `=1.2.3`.

### Caret

```
^1.2.3
```

Matches `>=1.2.3 <2.0.0`. From the example list: `1.2.3`, `1.5.0`.

In `0.x` ranges the upper bound shifts down a level:

```
^0.2.3
```

Matches `>=0.2.3 <0.3.0`. This is different from Cargo — see above.

### Tilde

```
~1.2.3
```

Matches `>=1.2.3 <1.3.0`. From the example list: `1.2.3` only.

### Hyphen

```
1.2 - 1.5
```

Matches `>=1.2.0 <=1.5.x`. From the example list: `1.2.0`, `1.2.3`, `1.5.0`.

### X-range

```
1.2.x
1.x
*
```

`1.2.x` matches `>=1.2.0 <1.3.0`. `1.x` matches `>=1.0.0 <2.0.0`. `*` matches
any non-pre-release version.

### Comparator

```
>=1.0 <2.0
>1.0.0
<=2.0.0
```

Whitespace separates ANDed comparators within a single comparator set.

### OR union

```
^1 || ^2
```

Matches anything that satisfies `^1` or `^2`. From the example list: all of
`1.0.0`, `1.2.0`, `1.2.3`, `1.5.0`, `2.0.0`.

### Empty string

An empty spec string is treated as `*`. This matters because npm registries
sometimes serve dependency entries with an empty `version` field, and we
want those to behave the same as a wildcard.

## Pre-release rules

Pre-release versions are handled the way npm handles them, which surprises
people coming from other ecosystems. The rule is:

> A pre-release version like `1.2.3-beta.1` is matched by a range only if at
> least one comparator in the range explicitly mentions a pre-release of the
> *same* `[major, minor, patch]` tuple.

So:

- `^1.2.0` does **not** match `1.2.3-beta.1`, even though `1.2.3-beta.1` is
  numerically inside `>=1.2.0 <2.0.0`.
- `>=1.2.3-alpha` *does* match `1.2.3-beta.1`, because the comparator names a
  pre-release on the `1.2.3` tuple.
- `>=1.0.0-alpha` does **not** match `1.2.3-beta.1`, because the pre-release
  in the comparator is on a different tuple (`1.0.0`, not `1.2.3`).

This is intentional. It prevents `^1.0.0` from silently picking up an
unrelated alpha build of some future minor version. If you want pre-releases,
opt in by writing them into the spec.

## The "highest matching" rule

When a range matches more than one published version, guroku always picks
the **highest** version in the range. Ties are broken by the natural
`Version` ordering from `node-semver`, which already accounts for
pre-release ordering (`1.2.3-alpha < 1.2.3-beta < 1.2.3`).

`max_satisfying` is the canonical entry point for this. The resolver should
not be hand-rolling its own comparison loops.

## What we don't accept (yet)

The following spec forms are valid in npm but are **not** accepted by
guroku v0.2:

- **URL specs.** `git://github.com/user/repo`, `https://example.com/pkg.tgz`,
  `file:./local-pkg`, and similar. These go through entirely different
  resolution paths (clone, download, link) and we have not wired those up
  yet.
- **npm aliases.** `alias@npm:real-name@^1`, where one logical name in
  `package.json` resolves to a different name on the registry.
- **Workspace protocol specs.** `workspace:*`, `workspace:^`, etc., used by
  monorepo tooling.

All three currently return `GurokuError::InvalidVersionSpec`. URL specs and
npm aliases are scheduled for v0.4. Workspace protocol support is scheduled
for v0.5, alongside the rest of the workspaces feature.

## Garbage-in handling

A spec that is not any of the supported forms above — for example
`"banana"`, `"1.2.3.4.5"`, or `"^^1"` — produces:

```rust
GurokuError::InvalidVersionSpec {
    name: String,  // the package name the spec was attached to
    spec: String,  // the original spec string, verbatim
}
```

guroku **never** silently falls back to `latest` on a bad spec. A bad spec
in `package.json` is a hard error at resolve time, and the user is told
which dependency caused it. Silent fallbacks were the single biggest source
of "why did I end up on this version?" bug reports during the v0.1 alpha,
and we are not bringing them back.

## Reference reading

- npm's node-semver, the canonical reference for the dialect:
  <https://github.com/npm/node-semver>
- The `node-semver` Rust crate we depend on:
  <https://docs.rs/node-semver>
- npm's `package.json` documentation, specifically the `dependencies`
  section, which describes how specs are interpreted in practice:
  <https://docs.npmjs.com/cli/v10/configuring-npm/package-json#dependencies>

[node-semver-crate]: https://docs.rs/node-semver
[cargo-semver-crate]: https://docs.rs/semver
