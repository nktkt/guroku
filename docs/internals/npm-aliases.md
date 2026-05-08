# npm: aliases

Internals doc for the `npm:<real>@<spec>` dependency alias form. This is a
v1.1 feature and the notes below describe the path the data takes from
`package.json` parsing through to a fully-linked `node_modules` tree.

## 1. What it is

A package.json dependency entry of the form

```json
{
  "dependencies": {
    "my-cool-name": "npm:lodash@^4.17"
  }
}
```

instructs the resolver to fetch `lodash` from the registry but expose the
package under the consumer's chosen alias name (`my-cool-name`). The
on-disk layout, the `require` lookup name, and the lockfile key all use
the alias - the registry name is only used for fetching metadata and the
tarball.

This matches npm's and pnpm's behaviour. The motivating use cases are:

- Pulling two versions of the same package side-by-side under different
  names (e.g. `react-17` and `react-18`).
- Replacing a package with a fork without rewriting every `require` site
  in the consumer.
- Vendoring a registry package under a project-specific name.

## 2. Spec classification

`src/specs.rs::classify` recognises the `npm:` prefix and returns a
dedicated `DepSpec::Alias` variant:

```rust
pub fn classify(spec: &str) -> DepSpec {
    if let Some(rest) = spec.strip_prefix("npm:") {
        // Split on the LAST '@' so scoped names survive the round trip.
        if let Some(at) = rest.rfind('@') {
            let real_name = rest[..at].to_string();
            let inner_spec = &rest[at + 1..];
            return DepSpec::Alias {
                real_name,
                inner: Box::new(classify(inner_spec)),
            };
        }
        // No version pin: treat as `npm:<name>@*`.
        return DepSpec::Alias {
            real_name: rest.to_string(),
            inner: Box::new(DepSpec::Range("*".into())),
        };
    }
    // ... other classifiers (range, file:, git:, workspace:, etc.) ...
}
```

The split is on the **last** `@`, which is what makes scoped names work.
For `npm:@types/node@^20`:

- `rest` is `@types/node@^20`
- `rfind('@')` lands on the `@` before `^20`, not on the leading scope `@`
- `real_name` becomes `@types/node`
- `inner` becomes `DepSpec::Range("^20")`

If you split on the first `@` you would get `real_name = ""` and an
`inner` of `types/node@^20`, which is wrong; the test suite in
`src/specs.rs` has a regression case for this.

## 3. `DepSpec::Alias` shape

```rust
pub enum DepSpec {
    Range(String),
    File(PathBuf),
    Git(GitSpec),
    Workspace(String),
    Alias {
        real_name: String,
        inner: Box<DepSpec>,
    },
    // ...
}
```

`inner` is boxed because `DepSpec` is otherwise a fixed-size enum and we
need indirection to break the cycle. The recursive shape is deliberate:
in principle `npm:foo@file:../path` is a syntactically valid alias, and
the parser will build a `DepSpec::Alias { real_name: "foo", inner:
DepSpec::File(...) }` for it. The resolver does not currently follow
that case (see section 10), but the type system does not forbid it - if
we decide to support it later we only need to add a resolver branch, not
re-shape the data.

In practice the inner is a `Range` 99% of the time.

## 4. Resolver flow

The relevant function is
`src/resolver.rs::resolve_with_manifest_overrides`. The resolver runs a
worklist over `(local_name, spec, path)` triples; each iteration does:

```rust
while let Some((local_name, raw_spec, path)) = queue.pop() {
    // 4.2: apply manifest-level overrides keyed by `path`.
    let spec = apply_overrides(&overrides, &path, &raw_spec);

    // 4.3: classify (after override, since overrides can introduce npm: prefixes).
    let classified = classify(&spec);

    // 4.4: decompose alias.
    let (registry_name, inner_spec, alias_real) = match classified {
        DepSpec::Alias { real_name, inner } => {
            (real_name.clone(), *inner, Some(real_name))
        }
        other => (local_name.clone(), other, None),
    };

    // 4.5: resolve against the registry using `registry_name`.
    let resolved = resolve_against_registry(&registry_name, &inner_spec, ...)?;

    // 4.6: record under `local_name`, NOT `registry_name`.
    let entry = Resolved {
        name: local_name.clone(),
        version: resolved.version,
        tarball: resolved.tarball,
        integrity: resolved.integrity,
        aliased_from: alias_real, // None for non-alias deps
        // ...
    };
    resolution_map.insert(local_name.clone(), entry);

    // Enqueue this resolved package's own deps under their package.json names.
    for (child_local, child_spec) in resolved.dependencies {
        queue.push((child_local, child_spec, path.child(&local_name)));
    }
}
```

A few subtleties:

- Overrides are applied **before** classification so that an override
  can rewrite a plain spec into an `npm:` alias (or vice versa).
- `aliased_from` is preserved on `Resolved` for downstream consumers
  (the lockfile writer in particular wants to know whether to emit the
  registry name in any ancillary fields).
- The recursive case (alias with non-Range inner) currently bails with
  `Err(ResolveError::UnsupportedAliasInner)` rather than silently
  resolving the wrong thing.

## 5. Why `local_name` is the resolution key

Every layer above the registry talks in terms of the name the consumer
**typed in their package.json**. Specifically:

- node_modules lookup: when the runtime executes
  `require('my-cool-name')` inside a consumer file, Node walks up the
  directory tree looking for `node_modules/my-cool-name`. It does not
  know or care what the registry name was.
- Sibling lookups: a child package that depends on `my-cool-name` will
  similarly look it up by that name.
- Lockfile keys: `guroku.lock` keys entries by the local name so that
  re-running install matches what's on disk.
- Override matching: overrides are written against the local name
  because that's the name the consumer sees and would type.

If we keyed the resolution map by `registry_name` we would constantly
need to translate back to `local_name` at every consumer site. Keying by
`local_name` lets the linker, the lockfile writer, and the override
system all consult the same map without translation.

The one thing that does need the registry name is the registry fetch
itself - and that's exactly what `aliased_from` and the resolver-local
`registry_name` variable are for.

## 6. Linker behaviour

`commands/mod.rs::into_linked_packages` translates the resolution map
into a list of `LinkedPackage` records that the strict-layout linker
consumes. The relevant simplification is:

```rust
fn into_linked_packages(map: &ResolutionMap) -> Vec<LinkedPackage> {
    map.iter()
        .map(|(local_name, resolved)| LinkedPackage {
            name: local_name.clone(),  // local name, NOT resolved.aliased_from
            version: resolved.version.clone(),
            cas_path: resolved.cas_path.clone(),
            // ...
        })
        .collect()
}
```

The strict-layout linker then writes:

```
node_modules/.guroku/<local-name>@<version>/node_modules/<local-name>/
node_modules/<local-name>  ->  ../.guroku/<local-name>@<version>/node_modules/<local-name>
```

For our running example with `"my-cool-name": "npm:lodash@^4.17"`
resolving to `4.17.21`:

```
node_modules/.guroku/my-cool-name@4.17.21/node_modules/my-cool-name/
    package.json    (this still says "name": "lodash" internally)
    index.js
    ...
node_modules/my-cool-name -> ../.guroku/my-cool-name@4.17.21/node_modules/my-cool-name
```

Note that the inner `package.json` still has `"name": "lodash"`. We do
**not** rewrite it. Some packages introspect their own `package.json`
and would break if we did. Node doesn't care - it resolves by path, not
by manifest name.

Consequence: `require('my-cool-name')` resolves; `require('lodash')`
does **not** resolve, unless the consumer also added a regular `lodash`
dep alongside the alias, in which case both names are linked
side-by-side.

## 7. CAS behaviour

The content-addressable store is keyed by the tarball's SHA-512, not by
package name. This means:

- Two aliases pointing at the same registry version share a single CAS
  entry. `"a": "npm:lodash@4.17.21"` and `"b": "npm:lodash@4.17.21"`
  cause exactly one tarball download and one extraction.
- The same is true for an alias and a regular dep at the same version:
  `"my-lodash": "npm:lodash@4.17.21"` plus `"lodash": "4.17.21"` share
  one CAS entry.

The `cas_paths` map (post-v1.1 fix) is keyed by **local name** rather
than registry name. The pre-fix bug was that the linker would look up
`cas_paths[registry_name]` and miss for aliased entries because they
were stored under the local name; the fix was to make the lookup match
the storage. See `commands/mod.rs` and the regression test
`tests/aliases_share_cas.rs`.

## 8. Lockfile recording

Alias entries land in `guroku.lock` under their **local name**. The
`resolved` URL points at the registry tarball, and the `dependencies`
field of the entry uses the registry-side dep names (because those are
the names the resolved package's own `package.json` contains).

```yaml
packages:
  /my-cool-name@4.17.21:
    resolved: "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz"
    integrity: "sha512-..."
    aliasedFrom: "lodash"
    dependencies:
      # whatever lodash's own deps are, by their real names
```

`aliasedFrom` is the breadcrumb that lets a future `guroku why` or
`guroku list` show the registry name alongside the local name without
re-classifying every spec.

## 9. Bin shims

`populate_bin_dir` reads bin entries from the package's own
`package.json`, which uses the registry name internally. So an alias
`"mocha-fork": "npm:mocha@^10"` will produce a `mocha` shim (because
mocha's package.json says `"bin": { "mocha": "bin/mocha.js" }`), not a
`mocha-fork` shim.

If two deps both contribute a bin called `mocha`, the last writer wins
(this is the same behaviour as for non-aliased deps). The order is
determined by resolution order, which is stable but not particularly
meaningful from a user's point of view.

Future work: prefix alias bins, or at minimum surface a warning when an
alias's bin would collide with another dep's bin. This is tracked as a
v1.2 item in `docs/internals/v1.0-checklist.md`.

## 10. What we don't yet do

- **Aliases pointing at non-registry sources.** The `DepSpec::Alias`
  shape supports a non-`Range` inner (because `inner` is a boxed
  `DepSpec`), but the resolver currently rejects anything other than
  `Range` inside an alias with `UnsupportedAliasInner`. Adding support
  for `npm:foo@file:../path` is a matter of writing the resolver branch
  and a couple of tests; the data model is already there.
- **Aliases in `peerDependencies` / `optionalDependencies`.** The parser
  for these maps does not yet route through `classify`, so an `npm:`
  prefix in either field is taken as a literal version string and fails
  semver parsing. Fix is mechanical: route them through the same
  `classify` call as `dependencies`.
- **Auto-cleanup of orphaned aliases on `guroku remove`.** Removing the
  `my-cool-name` entry from `package.json` and re-running install will
  drop the alias from `node_modules`, but `guroku remove my-cool-name`
  currently looks up by registry name internally and fails to find the
  alias. Workaround: edit `package.json` and re-run `guroku install`.

## 11. Diagnostics

`GUROKU_LOG=debug guroku install` prints both the local name and the
registry name when fetching metadata for an alias:

```
DEBUG resolver: fetching metadata local=my-cool-name registry=lodash spec=^4.17
DEBUG resolver: resolved local=my-cool-name registry=lodash version=4.17.21
DEBUG linker:   linking local=my-cool-name version=4.17.21 cas=<sha>
```

For non-alias deps the `registry` field is omitted (or equals `local`,
depending on log level). This makes alias-related issues immediately
visible in a debug trace - if you see only the registry name and not
the local name, the alias has been collapsed somewhere it shouldn't
have been.

If you suspect an alias is being keyed by the wrong name, the fastest
check is:

```sh
GUROKU_LOG=debug guroku install 2>&1 | grep -E "local=|registry="
```

and verify that every alias entry shows both names and that the linker
line uses the local name.
