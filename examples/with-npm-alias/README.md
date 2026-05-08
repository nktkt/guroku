# with-npm-alias

A guroku example that exercises npm-style package aliases.

## What this example shows

guroku supports the npm alias syntax for declaring dependencies:

```
"local-name": "npm:<real-name>@<spec>"
```

The `local-name` is the directory created under `node_modules` and the
identifier you pass to `require` / `import`. The `<real-name>@<spec>` is
what guroku actually fetches from the registry.

This example pins two different major versions of `is-odd` side-by-side,
under two different local names, alongside a regular (non-aliased)
dependency on `ms`:

```json
{
  "dependencies": {
    "is-odd-v2": "npm:is-odd@^2.0.0",
    "is-odd-v3": "npm:is-odd@^3.0.0",
    "ms": "^2.1.3"
  }
}
```

Both alias entries resolve to the same registry package (`is-odd`) at
different version ranges. They install independently and do not
deduplicate against each other.

## Try it

```sh
cd examples/with-npm-alias
rm -rf node_modules guroku.lock
guroku install
ls node_modules
```

You should see exactly three top-level entries:

```
is-odd-v2
is-odd-v3
ms
```

Note that there is no `is-odd` directory. The aliases replaced the
top-level name; only the alias-given names appear in `node_modules`.

## Verify the registry name

The alias only changes the directory name and the require key. The
package's own metadata is untouched.

```sh
cat node_modules/is-odd-v3/package.json
```

The `"name"` field in that file is `"is-odd"`, not `"is-odd-v3"`. This
matters if any code introspects `package.json` at runtime: it will see
the upstream name.

## In your code

Use the alias name on the require / import side:

```js
const isOddV2 = require('is-odd-v2'); // is-odd@2.x
const isOddV3 = require('is-odd-v3'); // is-odd@3.x

isOddV2(3); // true
isOddV3(3); // true
```

Both copies coexist in the same process. Each has its own module cache
entry keyed by the resolved path under `node_modules`.

## Lockfile entries

The lockfile records each alias under its local name and pins the
resolved tarball URL of the underlying package:

```sh
cat guroku.lock | grep is-odd
```

You will see two distinct keys (`is-odd-v2` and `is-odd-v3`), each with
its own `resolved` URL pointing at the corresponding `is-odd` tarball
on the registry. The `name` field inside each entry is `is-odd` (the
real package name); the alias only lives in the dependency key.

## What v1.1 doesn't yet do

The following alias forms are not supported in v1.1:

- Aliases pointing at non-registry specs, e.g.
  `npm:foo@file:./local` or `npm:foo@git+https://...`. Aliases must
  resolve to a registry version range or tag.
- Aliases declared inside `peerDependencies` or
  `optionalDependencies`. Only `dependencies` and `devDependencies`
  honor the `npm:` prefix today.

Both are tracked for a follow-up release. See `docs/aliases.md` for
the current scope.

## Common use cases

- **Migrating between major versions.** Install both as aliases, port
  call sites one file at a time, then drop the old alias when nothing
  imports it.
- **Forking a published package.** Start with `npm:upstream@^1` under
  a local name, validate behavior, then swap the alias target to a
  `file:` or `git+` spec once the fork lands. (Once the swap is
  supported. See above.)
- **Renaming an internal package** without re-publishing it under a
  new name. Consumers import the new name; the registry record stays
  the same.

## Related docs

- `docs/aliases.md` — user-facing alias semantics and supported specs.
- `docs/internals/npm-aliases.md` — how aliases flow through the
  resolver, lockfile writer, and linker.
