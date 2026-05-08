# Aliasing and the Linker

This note explains how npm-style aliases (`"react-old": "npm:react@^16"`) flow
through guroku's install pipeline: from the manifest, through the spec
classifier and resolver, into the CAS keyer, and finally onto disk via the
linker. It's a v1.1 explainer; the v1.0 pipeline didn't have to think about
any of this because there were no aliases.

## 1. The vocabulary

Three names show up in any discussion of aliases. Keep them straight:

- **local_name** — the key the user typed in `package.json`. This is what
  the rest of the system thinks the package is called. It's the
  `node_modules/<this>` directory name and the lockfile prefix.
- **real_name** — the actual package name on the registry. This is what
  guroku has to send to npm to get a tarball.
- **version / range** — the semver constraint, exactly as it would be for a
  non-aliased dep.

Worked example:

```json
{
  "dependencies": {
    "react-old": "npm:react@^16"
  }
}
```

- local_name = `react-old`
- real_name  = `react`
- range      = `^16`

## 2. Where the split happens

`specs::classify` is the first place that has to recognise the `npm:` prefix.
When it sees one, it returns:

```rust
DepSpec::Alias {
    real_name: String,
    inner: Box<DepSpec>,
}
```

The classifier splits the right-hand side on the **last** `@`, not the first.
This matters for scoped real names: `npm:@types/node@^20` has to parse as
real_name = `@types/node`, range = `^20`, not real_name = `` and version =
`types/node@^20`. Splitting on the last `@` falls out cleanly: everything
left of it is the package name (which may itself contain a leading `@`),
everything right is the version specifier.

The inner `DepSpec` is whatever the inner string would have parsed to on its
own (a semver range, a `git+` url, a `file:` path, etc.). Aliasing is an
outer wrapper, not a parallel universe of spec kinds.

## 3. Resolver bookkeeping

The resolver has to walk a tightrope: it has to fetch metadata under the
real name, but record the result under the local name, because everything
downstream of the resolver thinks in local names.

- The work queue carries the **local name** as its `name` field. Every
  conflict message, every progress line, every cycle-detection check uses
  the local name. This is intentional: when a user gets an error, they
  should see the string they typed in `package.json`, not the string
  guroku internally fetched.
- When the queue item's spec is `DepSpec::Alias { real_name, inner }`, the
  resolver uses `real_name` to talk to the registry: that's the name on the
  metadata document, that's the name in the tarball URL. The `inner` spec
  drives version selection as if it were a normal dep.
- The result goes into the `Resolution` map keyed under the **local name**.
  So `resolution["react-old"] = Resolved { ..., aliased_from:
  Some("react") }`.
- For non-aliased entries, `aliased_from = None`. Code that needs the
  registry name does `resolved.aliased_from.as_deref().unwrap_or(&name)`.

The conflict path deliberately uses local names. If a user has two
unrelated deps that happen to alias to the same real_name, they're not in
conflict — they're allowed to coexist (see section 5).

## 4. CAS keying

`commands::install::install_from_resolution` builds a `cas_paths`
side-table keyed by the resolution-map key, which is the **local name**.

Why local? Because the next stage, `into_linked_packages`, iterates the
resolution map and for each entry looks up `cas_paths[name]`. The two
sides of that lookup must use the same key. Since the resolution map is
keyed by local_name, `cas_paths` has to be too.

The wrong thing — and what v1.0 implicitly did, because it had no aliases
to worry about — is to key by `info.name` (the name pulled from the
fetched manifest, which is the real_name). For an aliased entry that
would write the entry under `react` while the iterator looked it up as
`react-old`, and the linker would silently drop the package.

This is a small change in code (one identifier swap) but a subtle bug in
behaviour. The unit test for it is `tests/resolution_alias_lookup.rs`.

## 5. Linker / node_modules layout

The directory we create in `node_modules` is named after the **local
name**, not the real name. That gives:

```
node_modules/
  react-old/        <- contents are react@16.x
    package.json    <- "name": "react"
  react/            <- contents are react@19.x
    package.json    <- "name": "react"
```

Note the asymmetry: the directory name is `react-old`, but the
`package.json` inside still says `"name": "react"` — we don't rewrite the
manifest, that would break the package's own self-references.

This matches npm's semantics exactly. Aliases exist precisely to let you
depend on two different majors of the same package side-by-side, by
giving one of them a local rename. Without aliases, the resolver would
have to pick one version of `react` and reject the other.

In `LinkedPackage`, the `name` field is set to `local_name`. v1.0 set it
to `info.name` (the real name); the v1.1 fix is the one-liner that
substitutes local_name. Everything that consumes `LinkedPackage` —
hardlink layout, bin shim creation, lifecycle hooks — then uses the
correct name without further changes.

## 6. Lockfile

The lockfile key is `<local_name>@<version>`. So the example above gives
two distinct entries:

```
react-old@16.14.0:
  resolved: https://registry.npmjs.org/react/-/react-16.14.0.tgz
  aliased_from: react
  ...
react@19.0.0:
  resolved: https://registry.npmjs.org/react/-/react-19.0.0.tgz
  ...
```

`aliased_from` is omitted (or null) for non-aliased entries.

This layout is forwards-compatible with v1.0 lockfiles. A v1.0 lockfile
with `react@19.0.0` and no aliases looks identical under v1.1: the
`aliased_from` field is absent, and the local_name happens to equal the
real_name, so nothing has to be migrated. New aliases simply add new keys.

## 7. What's NOT done in v1.1

Aliasing inside transitive deps is not supported.

Concretely: if you depend on `is-odd`, and `is-odd` depends on
`is-number`, you cannot from your `package.json` say "rewrite the
`is-number` dep inside `is-odd` to point at my locally-renamed
`is-number-renamed`". That would require guroku to walk the dep tree and
substitute names inside other packages' manifests, which we don't yet do.
The override system (see `overrides.md`) has the machinery to rewrite
versions but not to rename.

Aliases for **root** deps work, and they propagate naturally in the sense
that the aliased copy lives in `node_modules/<local_name>/` and is
visible to anyone who specifically asks for `<local_name>`. Transitive
aliasing is in the v1.x backlog.

## 8. Tests

The test files that exercise this pipeline:

- `tests/specs_alias_classify.rs` — classifier recognises `npm:` and
  splits on the last `@`, including for scoped real names.
- `tests/specs_alias_unparse_round_trip.rs` — `DepSpec::Alias` round-trips
  through the unparser.
- `tests/resolved_aliased_from.rs` — `Resolved.aliased_from` is populated
  for aliased entries and `None` otherwise.
- `tests/resolution_alias_lookup.rs` — the resolution map is keyed under
  local_name; aliased entries are findable under the user-typed key.
- `tests/manifest_aliases_dont_collide.rs` — a manifest with both
  `"react-old": "npm:react@^16"` and `"react": "^19"` resolves cleanly,
  with both packages installed side-by-side.
