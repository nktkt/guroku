# Overrides and Resolutions

This document describes how guroku v0.5 reads and applies the
`overrides` (npm 8+) and `resolutions` (yarn classic) fields from
`package.json` during dependency resolution.

## Why overrides exist

A package's transitive dependency tree is normally controlled by the
authors of each intermediate package: if `foo` declares `bar@^1.0.0`,
the resolver picks whatever version of `bar` happens to satisfy that
range at install time. Most of the time this is fine.

Occasionally it is not. Common reasons to override a transitive:

- A security advisory applies to a specific version range, and the
  parent package has not yet released a fix.
- A particular release of a transitive is broken (publish error,
  regression) and you need to pin to the previous version.
- You are debugging a bug in a transitive and want to force a local
  patched version everywhere it appears.

To make this expressible at the project level (rather than forking the
parent package), npm 8 added the `overrides` field and yarn classic
ships an equivalent `resolutions` field. Both let the root project say
"no matter who asks, this name resolves to this spec".

## What v0.5 ships

v0.5 supports the simplest form of both fields: a top-level map from
package name to an exact version (or any spec the resolver knows how
to parse).

```json
{
  "name": "my-app",
  "dependencies": {
    "foo": "^1.0.0"
  },
  "overrides": {
    "lodash": "4.17.21"
  },
  "resolutions": {
    "minimist": "1.2.8"
  }
}
```

Both `overrides` and `resolutions` are read into the same logical
table. When the same key appears in both, `overrides` wins. This
matches npm's behavior: in a project that uses both fields, npm
ignores `resolutions` for keys it also has in `overrides`.

## What v0.5 does NOT yet ship

The path-keyed forms supported by npm and yarn are not implemented in
v0.5. Specifically:

- npm path syntax: `"foo > bar": "1.0.0"` — only override `bar` when
  it is reached as a child of `foo`. Other paths to `bar` are left
  alone.
- yarn glob syntax: `"**/foo": "1.0.0"` — match any path ending in
  `foo`. Yarn also accepts more elaborate globs.

Both are tracked as v0.5.x work. See "Future work" at the end of this
doc.

## The `lookup` function

The single entry point for "is there an override for this name" is:

```rust
pub fn lookup(manifest: &Manifest, name: &str) -> Option<String> {
    if let Some(spec) = manifest.overrides.get(name) {
        return Some(spec.clone());
    }
    if let Some(spec) = manifest.resolutions.get(name) {
        return Some(spec.clone());
    }
    None
}
```

The order is fixed:

1. Check `manifest.overrides[name]`.
2. Else check `manifest.resolutions[name]`.
3. Else `None`.

This function is intentionally pure: it does not consult the lockfile,
the registry, or the dependency graph. It only answers "what does
`package.json` say about this name?".

## The `merged` function

For inspection and tooling, `overrides::merged` returns a single
`BTreeMap<String, String>` with both fields combined:

```rust
pub fn merged(manifest: &Manifest) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (k, v) in &manifest.resolutions {
        out.insert(k.clone(), v.clone());
    }
    for (k, v) in &manifest.overrides {
        out.insert(k.clone(), v.clone());
    }
    out
}
```

`resolutions` are inserted first; `overrides` are inserted last and
therefore replace any duplicate keys. The `BTreeMap` ordering is
chosen so the output is deterministic and easy to diff.

This is what `lookup` is built on top of conceptually, though
`lookup` avoids allocating the full map for the common case where the
caller only needs a single name.

## How the resolver applies them

Override application lives at the boundary between "I have a
dependency request" and "I am about to ask the registry about it".
The relevant function is `resolver::resolve_with_overrides`:

```rust
pub fn resolve_with_overrides(
    manifest: &Manifest,
    name: &str,
    raw_spec: &str,
) -> Result<Resolved, ResolveError> {
    let effective_spec = match overrides::lookup(manifest, name) {
        Some(forced) => forced,
        None => raw_spec.to_string(),
    };

    let classified = classify_spec(&effective_spec)?;
    let metadata = fetch_metadata(name, &classified)?;
    pick_version(name, &classified, &metadata)
}
```

The flow for every `(name, raw_spec)` request:

1. Look up `overrides[name]`.
2. If present, replace `raw_spec` with it as the *effective* spec
   (typically an exact version, but any parseable spec works).
3. Classify the effective spec (range, exact, tag, git, file, etc.).
4. Fetch metadata as usual.
5. Resolve to a concrete version as usual.

The resolver still validates the override: an unparseable spec fails
with the same `InvalidSpec` error any other bad spec would produce.
The override is not magic; it is just a forced replacement of the
spec string before normal resolution runs.

## Caveats

- Overrides are GLOBAL. Every appearance of the named package is
  affected, regardless of the path through the dependency graph that
  reached it. v0.5 does not distinguish "lodash via foo" from "lodash
  via bar". This is a deliberate v0.5 simplification, not an oversight
  of npm semantics.
- Setting an override to a version that does not exist on the
  registry produces `NoMatchingVersion` at resolve time, exactly as if
  you had written that version in `dependencies` directly.
- `resolutions` glob patterns such as `"**/foo"` are read into the
  map verbatim. The lookup is a plain string equality check against
  the package name, so a key like `"**/foo"` only matches a package
  literally named `**/foo`, which does not exist. In other words,
  globs do not work yet; they are silently inert.
- Overrides do not bypass integrity checks or the content-addressed
  store. The forced version still has to download cleanly and match
  its registry-provided integrity.

## Lockfile interaction

Overrides are NOT recorded in `guroku.lock` as a separate section.
The lockfile records the resolved version of each package, and that
version already reflects whatever override was in effect at resolve
time.

Practical consequences:

- Adding, removing, or changing an override invalidates the relevant
  entries in the lockfile and requires re-resolution. The override
  itself is not a lockfile field, so the diff shows up as a version
  change on the affected packages.
- To audit "what overrides are currently in effect for this project",
  read `package.json`. The lockfile alone cannot tell you whether a
  given pinned version came from an override or from natural
  resolution.
- Two projects with identical `dependencies` but different
  `overrides` will produce different lockfiles, as expected.

## Future work

The following items are on the v0.5.x roadmap:

- Path-keyed overrides (`"foo > bar": "1.0.0"`). This requires the
  resolver to know the current path through the dependency graph at
  the moment it asks for an override, not just the package name.
- Glob support (`"**/foo"`, and the more elaborate yarn forms). This
  is largely a matter of compiling the patterns at manifest load time
  and matching them in `lookup` instead of doing string equality.
- Reporting overrides in `guroku audit` output, so security auditing
  can clearly indicate "this advisory is mitigated by an override"
  versus "this advisory is unmitigated".

Until these land, projects that need path-specific or glob-based
overrides should continue to use npm or yarn for the resolve step and
import the resulting lockfile, or pin the offending package at the
top level as a regular dependency.
