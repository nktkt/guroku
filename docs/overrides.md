# Overrides and Resolutions

This document describes how to pin transitive dependencies in guroku v0.5
using the `overrides` (npm) and `resolutions` (yarn classic) fields in
`package.json`. It covers what overrides solve, the simple form supported
in v0.5, what is not yet implemented, common use cases, and how to verify
that an override actually took effect.

## 1. What overrides solve

A direct dependency is one that appears in your own `package.json`. A
transitive dependency is anything pulled in by one of your direct
dependencies. The version of a transitive is normally chosen by whichever
parent package declares it. Most of the time that is fine.

Sometimes it is not. The two most common reasons to take direct control
of a transitive's version are:

- **Patching a known vulnerability before the parent ships a fix.** A CVE
  has been published against `some-lib@<2.1.3`. The package in your tree
  that depends on `some-lib` has not yet released an update bumping its
  range. You cannot wait. You want to force `some-lib@2.1.3` everywhere
  in your tree, today.

- **Forcing a single version of a duplicated transitive.** Two paths
  through the dependency graph land on `lodash` at incompatible ranges
  and the resolver therefore installs both copies. Disk usage and bundle
  size grow. You want exactly one `lodash` in the tree. (This is a v0.5
  simplification — see Caveats below.)

Overrides are how you express "I, the application author, get to decide
what version of this package ships, regardless of what my dependencies
asked for."

## 2. The two field names

The npm CLI (8 and later) uses a top-level field called `overrides`.
Yarn classic uses a top-level field called `resolutions`. The two were
designed independently and have slightly different semantics in their
fully expressive forms, but the **simple form** — a flat map of package
name to exact version — is identical in meaning across both ecosystems.

guroku v0.5 reads both fields. If both are present, `overrides` wins on
conflict. The merge happens key by key:

- A key that appears only in `resolutions`: used.
- A key that appears only in `overrides`: used.
- A key that appears in both with the same value: used (no conflict).
- A key that appears in both with different values: the value from
  `overrides` is used and a warning is logged.

In practice you should pick one field and stick with it. Reading both is
intended as a migration convenience for projects converting from yarn
classic.

## 3. The simple form supported in v0.5

The supported form is a flat object whose keys are package names and
whose values are version specifiers (typically exact versions, but ranges
are also accepted).

```json
{
  "name": "my-app",
  "version": "1.0.0",
  "dependencies": {
    "lodash": "^4.17.0"
  },
  "overrides": {
    "ms": "2.1.3",
    "left-pad": "1.3.0"
  }
}
```

The effect of this configuration is:

- Every appearance of `ms` anywhere in the resolved tree gets pinned to
  `2.1.3`, regardless of what range its parent asked for.
- Every appearance of `left-pad` anywhere in the resolved tree gets
  pinned to `1.3.0`.
- `lodash` is unaffected; it resolves normally against `^4.17.0`.

The same in yarn-classic style:

```json
{
  "name": "my-app",
  "version": "1.0.0",
  "dependencies": {
    "lodash": "^4.17.0"
  },
  "resolutions": {
    "ms": "2.1.3",
    "left-pad": "1.3.0"
  }
}
```

guroku v0.5 treats these two examples identically.

## 4. What v0.5 doesn't yet support

Both npm and yarn allow more expressive override keys than a bare package
name. guroku v0.5 does **not** yet implement these expressive forms.
What is not supported:

- **Path-keyed overrides.** npm allows keys of the form
  `"foo > bar": "1.0.0"`, meaning "only pin `bar` to `1.0.0` when it is
  reached via `foo`." guroku v0.5 parses this string but treats the whole
  key as a literal package name. There is no package literally named
  `foo > bar`, so the entry matches nothing.

- **Glob keys.** Yarn allows keys of the form `"**/foo": "1.0.0"`,
  meaning "pin `foo` everywhere." Again, guroku v0.5 parses this as a
  literal name. There is no package literally named `**/foo`, so the
  entry matches nothing.

The practical impact: if you have an existing yarn project whose
`resolutions` are written in the glob form, you must rewrite them in the
simple form before they will take effect under guroku v0.5. For most
real-world cases the rewrite is mechanical: `"**/foo"` becomes `"foo"`.

If you have an existing npm project that uses path-keyed overrides to
say "pin `bar` only under `foo`," you have a harder choice: either
upgrade to the simpler "pin `bar` everywhere," or wait for a future
guroku release to add path-keyed support.

## 5. Use cases in detail

### 5.1 Security patch

Suppose a vulnerability has been fixed in `ms@2.1.3` but your dependency
tree currently has `ms@2.0.0` because some package in the middle pinned
to `^2.0.0` in a way that does not float (or because the lockfile is
sticky). You want to ship the patched version today.

```json
{
  "overrides": {
    "ms": "2.1.3"
  }
}
```

Re-run install:

```sh
guroku install
```

The lockfile re-resolves `ms` to `2.1.3` everywhere it appears. The
parent packages are not modified — they still declare whatever range
they originally declared — but the resolver substitutes the override.

### 5.2 Test compatibility

You are evaluating whether your application works against the next major
of React. Your real dependencies still pin `^18`. You don't want to
modify each parent package; you just want one bleeding-edge install to
test against.

```json
{
  "overrides": {
    "react": "19.0.0-rc.1"
  }
}
```

Run install. Now every reference to `react` in the tree points at
`19.0.0-rc.1`. When you are done testing, delete the override and re-run
install to revert.

### 5.3 De-duplication

Two paths converge on `lodash` at different version ranges. Without
intervention, the resolver picks two satisfying versions and installs
both. You force a single version:

```json
{
  "overrides": {
    "lodash": "4.17.21"
  }
}
```

Both callers now share `4.17.21`. Caveat: if one of the callers required
a major-incompatible range and you have forced an incompatible version,
that caller may break at runtime. The override does what you asked for;
it is up to you to pick a version that satisfies the actual semver
requirements of the callers.

## 6. Lockfile interaction

Overrides are **not** separately recorded in `guroku.lock`. The lockfile
only stores the resolved tree. Once the override has been applied, the
entries in the lockfile reflect the post-override versions.

This means: if you read `guroku.lock` alone, you cannot tell which
versions came from normal resolution and which came from an override.
The source of truth for "what is currently being forced" is the
`overrides` (or `resolutions`) field in `package.json`.

If you want to audit the override surface, read `package.json`. If you
want to audit the actually-installed versions, read `guroku.lock`. The
two together describe the install completely.

## 7. Removing an override

To remove an override, delete the key from `package.json` and re-run
install:

```sh
guroku install
```

The lockfile re-resolves the affected package(s) using normal
resolution. Other parts of the tree are unaffected.

If you want to remove all overrides at once, delete the entire
`overrides` (or `resolutions`) object and re-run install.

## 8. Verifying overrides took effect

The simplest check is `grep` against the lockfile. The lockfile uses
`<name>@<version>` as a key prefix:

```sh
grep -A3 '"ms@' guroku.lock
```

The version under `"ms@..."` should match your override. If you see
multiple entries for `ms` at different versions, the override did not
fully apply — most commonly because you misspelled the package name.

For a more programmatic check:

```sh
guroku list ms
```

This prints the installed version of `ms` and the path(s) it appears
under. After an override, every path should show the same version.

## 9. Compatibility

A summary table of what works and what doesn't in v0.5:

| Source                             | Form                       | Status      |
| ---------------------------------- | -------------------------- | ----------- |
| npm `overrides`                    | flat name to version       | works       |
| yarn `resolutions`                 | flat name to version       | works       |
| npm `overrides`                    | path-keyed (`foo > bar`)   | not yet     |
| yarn `resolutions`                 | glob (`**/foo`)            | not yet     |
| both fields present, no conflict   | merged                     | works       |
| both fields present, conflict      | `overrides` wins, warns    | works       |

Existing npm-authored simple `overrides` JSON: works without
modification. Existing yarn-authored simple `resolutions` JSON: works
without modification. Existing path-keyed npm overrides: parse, but
match nothing; rewrite to the simple form. Existing yarn glob
resolutions: parse, but match nothing; rewrite to the simple form.

## 10. FAQ

### Why didn't my override apply?

The most common cause is a spelling mismatch. Override keys are matched
exactly against the package's `name` field in its own `package.json`. If
the package is published as `@scope/foo`, the key must be `@scope/foo`,
not `foo`. Override keys are case-sensitive: `Lodash` does not match
`lodash`.

The second most common cause is using a glob or path-keyed form. If your
key contains `*`, `>`, or whitespace, v0.5 will treat the entire string
as a literal name and match nothing. Rewrite to the simple form.

### Can I use a range as an override?

Yes. The value in an `overrides` (or `resolutions`) entry is a version
specifier and may be a range. The resolver re-resolves the affected
package as if every parent had asked for that range. Pinning to an exact
version is more common because the whole point of an override is usually
to remove ambiguity, but ranges are accepted and work as you would
expect.

### Does `guroku audit` report on the override target?

Yes. `guroku audit` reads the lockfile, which already reflects override
resolution. The advisory database is consulted against the
post-override versions. If your override pinned to a vulnerable version,
`audit` will flag that vulnerable version. If your override pinned to a
patched version, `audit` will be quiet (assuming no other findings).

### Can I override a direct dependency?

Yes, although in most cases you should just edit the `dependencies`
range itself. An override on a direct dependency is treated the same
way as an override on a transitive: every reference to that name in the
resolved tree is pinned to the override value. This is occasionally
useful when you want the `dependencies` range to remain loose for
documentation purposes but want to lock the actual installed version.

### Does an override change `package.json` for child packages?

No. The override is applied during resolution. The published
`package.json` files inside `node_modules/<pkg>/package.json` are not
rewritten. Only the resolved version (and therefore the contents of
that directory) reflects the override.

### What happens if I override a package to a non-existent version?

Install fails with a resolution error pointing at the offending
override. The lockfile is not updated. Fix the override value and
re-run install.

### What happens if both `overrides` and `resolutions` are present?

guroku v0.5 reads both and merges them key by key. On a conflicting
key, the `overrides` value wins and a warning is logged. We recommend
not mixing the two fields in the same project; pick one.

### Are overrides inherited from workspace roots?

In a workspace, only the root `package.json`'s `overrides` field is
honoured. Per-workspace `overrides` are ignored in v0.5. This matches
npm's and yarn's behaviour and is the source of truth that maps cleanly
onto a single resolved tree.
