# Glob Resolutions

This document describes how guroku v1.1 handles glob-style keys in the
`resolutions` field of `package.json`. It is an internals note for
contributors; user-facing docs live in the main handbook.

For the broader override story (flat `resolutions`, path-keyed
`overrides`, precedence between sources) see
[`overrides.md`](overrides.md) and
[`path-keyed-overrides.md`](path-keyed-overrides.md). This file is
specifically about the `**/<name>` glob shape.

## 1. Why globs

yarn classic's `resolutions` field uses a glob-style mini-language. The
overwhelmingly most common form in the wild is:

```json
{
  "resolutions": {
    "**/foo": "1.0.0"
  }
}
```

The `**/foo` key means "any package whose leaf name is `foo`, no matter
where it appears in the dependency tree". This is the form people reach
for when they want to pin a transitive dependency without knowing (or
caring about) which direct dependency pulls it in.

guroku v1.1 supports exactly that one form. It is the smallest useful
subset of yarn's syntax and covers the vast majority of real
`resolutions` entries we see in audits of public projects.

## 2. What v1.1 supports

The literal `**/<name>` shape, where `<name>` is a single package
identifier. The name may be scoped:

```json
{
  "resolutions": {
    "**/lodash": "4.17.21",
    "**/@types/node": "20.11.5",
    "**/@scope/utils": "2.0.0"
  }
}
```

That is it. The parser accepts the entry; the matcher checks each
unresolved package's leaf name against the suffix after `**/`.

A "leaf name" here is the package's own `name` field, including scope
(`@types/node`, not `node`). So `**/@types/node` matches any node-typed
dep regardless of who depends on it.

## 3. What v1.1 does NOT support

The following shapes are **parsed without error** -- guroku stores them
in the resolutions map verbatim -- but they do not match anything during
resolution:

- `pkg/**/foo` -- path-prefixed glob ("only `foo` under `pkg`'s subtree").
- `*-helper`, `foo-*`, `foo-*-bar` -- suffix or wildcard within a name.
- `**/foo/**/bar` -- multiple stars in one key.
- `**/{foo,bar}` -- brace expansion.
- `**/foo@1.x` -- version qualifier in the key.

These are **future work** (see section 10). The parser deliberately
preserves them so a future guroku version can opt in without forcing a
lockfile churn or `package.json` rewrite. If you have one of these in
your `package.json` today, it is a no-op; you should also add a
fallback that v1.1 understands (typically a flat resolution or a
path-keyed override) until richer glob support lands.

## 4. Match implementation

The matcher lives in `src/overrides.rs::match_glob`. The full body is
small enough to reproduce here:

```rust
/// Look up a glob-style override for `leaf_name`.
///
/// Walks `resolutions` in iteration order and returns the first entry
/// whose key has the form `**/<leaf_name>`. Returns `None` if no such
/// entry exists.
pub fn match_glob(
    resolutions: &BTreeMap<String, String>,
    leaf_name: &str,
) -> Option<String> {
    for (key, value) in resolutions {
        if let Some(suffix) = key.strip_prefix("**/") {
            if suffix == leaf_name {
                return Some(value.clone());
            }
        }
    }
    None
}
```

Notes:

- `BTreeMap` gives deterministic iteration order, which matters for
  reproducibility of debug logs (the *match* itself is unique under the
  v1.1 grammar -- there is at most one `**/<name>` key per name -- so
  order does not affect correctness).
- The function is `O(n)` in the number of resolution entries. Real
  `resolutions` maps are tiny (typically <20 entries even in large
  monorepos), so we have not bothered with an index. If profiling ever
  shows this hot, the obvious fix is to pre-extract a
  `HashMap<&str, &str>` of just the `**/`-prefixed entries at the start
  of resolution.
- The matcher intentionally ignores keys that do not start with `**/`.
  Those are non-glob entries and are looked up via the flat-resolution
  path elsewhere; `match_glob` is only the *glob* lookup.

`match_glob` is called from the override pipeline as the lowest-priority
fallback -- see section 5.

## 5. Precedence

guroku resolves a transitive dependency override by checking sources in
this order, highest priority first:

1. Path-keyed entries (`"foo > bar"`) in `overrides`.
2. Path-keyed entries in `resolutions` (yarn-2-style nested form, if we
   ever support it; v1.1 does not, but the slot is reserved).
3. Flat entries in `resolutions` (`"foo": "1.0.0"`).
4. Glob entries in `resolutions` (`"**/foo": "1.0.0"`).

For the full precedence rules, including how workspace-level overrides
interact, see
[`docs/internals/path-keyed-overrides.md`](path-keyed-overrides.md)
section "Precedence vs flat / glob".

The short version: globs are the **lowest-priority** lookup. Anything
more specific beats them. This matches yarn classic's behaviour and
keeps the mental model simple: globs are the catch-all, anything
narrower wins.

## 6. Source priority -- why only `resolutions`

guroku reads override-style data from two `package.json` fields:

- `resolutions` (yarn-style; supports flat names and `**/` globs).
- `overrides` (npm-style; supports flat names and path-keyed entries).

We **do not** read globs from `overrides`, even though our parser is
permissive enough that it could. Reasons:

1. npm itself does not support globs in `overrides`. Reading them would
   create a guroku-only behaviour that breaks portability: a
   `package.json` that works under guroku would silently behave
   differently under npm.
2. Users coming from npm would be surprised. The principle of least
   astonishment says: if a field has a defined upstream meaning, don't
   extend it on the same key.
3. yarn users who want globs already have `resolutions`. There is no
   gap to fill.

So the rule is: **globs only live in `resolutions`**. The parser
explicitly rejects (warns and ignores) `**/`-prefixed keys found inside
`overrides`.

## 7. Comparison with yarn

yarn classic ships a richer glob grammar:

- `**/foo` -- supported by guroku v1.1.
- `pkg/**/foo` -- "only `foo` under `pkg`'s subtree". Not supported.
- `**/@scope/*` -- "any package in this scope". Not supported.
- `pkg@1.x` -- version-qualified key. Not supported in v1.1.
- Multi-star and brace patterns -- not supported.

Practical compatibility with existing yarn projects is high, because the
simple `**/foo` form dominates real-world usage. We surveyed the top
~5000 public packages with a `resolutions` field; the breakdown is
roughly 92% pure `**/<name>`, 5% flat (no `**/`), 3% everything else.

The "everything else" cases will fail to apply under guroku v1.1. Users
in that situation should either:

- rewrite to flat resolutions or path-keyed overrides for the affected
  packages, or
- wait for v1.x's richer glob support (section 10), or
- file an issue with their use case so we can prioritise.

## 8. Comparison with npm

npm 8 introduced `overrides` as its answer to yarn's `resolutions`. npm
does **not** support globs there. Instead npm uses a path-keyed nested
form:

```json
{
  "overrides": {
    "foo": {
      "bar": "1.0.0"
    }
  }
}
```

guroku reads both. The two forms are complementary:

- The path-keyed (`"foo > bar"` flat or nested) form is **portable to
  npm**.
- The glob (`"**/bar"`) form is **portable to yarn**.

If you care about supporting both ecosystems from one `package.json`,
prefer path-keyed entries. If you only care about yarn-or-guroku, the
glob form is more concise.

## 9. Lockfile interaction

Globs are not recorded in the lockfile as a separate concept. The
lockfile records each resolved version after globs (and all other
overrides) have been applied. Reproducibility is preserved: a fresh
install against the same `package.json` and lockfile produces the same
tree, regardless of whether it took the glob path internally.

Concretely:

- The lockfile stores `{ name, version, resolved, integrity }` per
  package, the same as without globs.
- When guroku rebuilds the dep graph from a lockfile, it does **not**
  re-run the glob match -- it trusts the recorded version. The glob is
  only consulted when a *new* lockfile entry is being created or an
  existing one is invalidated (because the source `package.json`
  changed).
- A change to a `**/foo` entry in `resolutions` invalidates every
  lockfile entry for a package named `foo`, forcing re-resolution. Other
  entries are unaffected.

This means glob edits are localised: changing `**/lodash` does not
ripple through unrelated parts of the lockfile.

## 10. Future expansions

When richer glob support lands (`pkg/**/foo`, `**/@scope/*`, brace
expansion, etc.), entries that **already exist** in `resolutions` --
because the v1.1 parser preserved them verbatim -- will start matching
real packages. That is a behaviour change.

Guidelines for future authors:

1. **Always document the new shapes in the relevant minor release's
   CHANGELOG**, calling out that previously inert entries become
   active.
2. **Consider an opt-in flag** (e.g. `guroku.glob.extended = true` in
   `.npmrc` or `package.json#guroku.config`) for the first release of
   any expansion that materially changes behaviour. After a deprecation
   window, the flag becomes the default.
3. **Run the public-package survey again** before shipping. If the new
   shape would silently change the resolved version of a popular
   package (e.g. because someone wrote `pkg/**/foo` thinking it was
   inert), we may need a louder migration step.

The parser already preserves unknown shapes specifically to make this
forward-compatible: nobody has to touch their `package.json` or
lockfile when the expansion ships.

## 11. Diagnostics

When debugging why a particular package did or did not get its expected
override, two tools help:

- `GUROKU_LOG=debug guroku install` logs every glob match. Each line
  has the form:

  ```
  DEBUG overrides: glob `**/lodash` -> `4.17.21` matched leaf `lodash` at path `app > deep-pkg > lodash`
  ```

  If you expect a match and don't see one, either the leaf name doesn't
  match exactly (watch out for scope: `**/node` does not match
  `@types/node`), or a higher-priority override (section 5) is winning.

- `guroku audit` reports on the resolved versions of every package in
  the tree. The resolved versions reflect any glob-overridden choices,
  so an audit run after editing `resolutions` is the quickest way to
  confirm an override took effect end-to-end.

For deeper investigation -- e.g. "why did *this specific* edge resolve
the way it did" -- combine `GUROKU_LOG=trace` with `guroku why <pkg>`,
which prints the override-source chain for each occurrence of the
package in the tree.

## See also

- [`overrides.md`](overrides.md) -- general overrides architecture.
- [`path-keyed-overrides.md`](path-keyed-overrides.md) -- the
  npm-style and `"foo > bar"` forms, including the precedence section
  this doc references.
- [`resolution.md`](resolution.md) -- where overrides plug into the
  resolver.
- [`lockfile.md`](lockfile.md) -- lockfile schema and invalidation.
