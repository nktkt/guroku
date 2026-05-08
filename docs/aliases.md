# Aliases

Aliases let you install a registry package under a different local name. This
page covers the spec form, how guroku resolves and lays out aliased packages,
the lockfile representation, and the rough edges to watch for.

## What an alias is

An alias is a dependency entry whose left-hand-side name (the key in
`package.json#dependencies`) is *not* the same as the package that ends up on
disk. The right-hand side carries an `npm:` prefix that tells the package
manager which registry package to actually install.

There are two main reasons to reach for an alias:

1. **Side-by-side versions.** You want two major versions of the same package
   installed at once, each importable under its own name. A standard
   dependency entry can't express this because the import name has to match
   the package name on disk.
2. **Local renaming.** A published package has a name that doesn't fit your
   codebase (too long, ambiguous, includes a vendor prefix, conflicts with
   another internal name). You'd rather import it under a name you control,
   without forking or republishing.

Aliases solve both with a single syntax that is portable across every major
package manager.

## The spec form

In `package.json#dependencies`, the right-hand side becomes
`npm:<real-name>@<spec>`:

```json
{
  "dependencies": {
    "my-lodash-v4": "npm:lodash@^4.17",
    "my-lodash-v5": "npm:lodash@^5"
  }
}
```

After `guroku install`, the layout has both:

- `node_modules/my-lodash-v4` resolves to `lodash` v4.x.
- `node_modules/my-lodash-v5` resolves to `lodash` v5.x.

In your source, `require('my-lodash-v4')` returns lodash v4 and
`require('my-lodash-v5')` returns lodash v5. There is no runtime indirection;
the directory name on disk *is* the alias, and Node resolves it the same way
it resolves any other package.

## Format

The general shape is:

```
npm:<real-package-name>@<spec>
```

- `<real-package-name>` is the name guroku looks up against the registry.
- `<spec>` is whatever you would put on the right-hand side of a normal
  dependency. That includes:
  - **Ranges**: `^4.17`, `~5.0.1`, `>=2 <3`.
  - **Exact versions**: `4.17.21`.
  - **Dist-tags**: `latest`, `next`, `beta`.

Ranges, exact versions, and dist-tags all work because `<spec>` is parsed
with the same machinery as a non-aliased dependency. The only thing the
`npm:` prefix changes is which name guroku resolves; everything else is
identical.

## Scoped packages

Scoped names contain an `@`, which interacts with the `@` that separates the
spec. guroku treats only the **last** `@` as the spec separator, so the
scope is preserved verbatim:

```json
{
  "dependencies": {
    "node-types-old": "npm:@types/node@^16",
    "node-types-new": "npm:@types/node@^20"
  }
}
```

This installs `@types/node` v16.x as `node_modules/node-types-old` and
`@types/node` v20.x as `node_modules/node-types-new`. You can then write:

```ts
import type { Buffer as OldBuffer } from 'node-types-old';
import type { Buffer as NewBuffer } from 'node-types-new';
```

The local name itself does not need to be scoped. You can give a scoped
package a flat alias, or a flat package a scoped alias. The only requirement
is that the local name is a valid npm package name.

## What happens at install

When guroku encounters an `npm:` spec, the install pipeline does the
following:

1. **Parse.** Split off the `npm:` prefix, then split on the last `@` to
   recover `<real-package-name>` and `<spec>`.
2. **Resolve.** Look up `<real-package-name>` in the registry and pick a
   concrete version that satisfies `<spec>`. This is the same resolver path a
   normal dependency would take.
3. **Fetch.** Download the tarball for the resolved version. Because the
   content-addressable store keys on tarball integrity, two aliases pointing
   at the same underlying version share a single CAS entry. There is no disk
   penalty for aliasing a package you also depend on directly under its real
   name.
4. **Lay out.** Place the package in the strict-layout tree under its
   resolved identity (e.g. `lodash@4.17.21`), then point
   `node_modules/<local-name>` at that entry. The local name is what your
   code sees; the strict layout still uses the real package identity for
   internal bookkeeping.

The two aliases in the lodash example earlier produce two CAS entries (one
per version), and two `node_modules/*` entries pointing into them.

## Lockfile

In `guroku.lock`, alias entries are keyed by the **local name**, not the
registry name. The `name` field inside the entry records the registry name
so that resolution is reproducible:

```json
{
  "packages": {
    "my-lodash-v4": {
      "name": "lodash",
      "version": "4.17.21",
      "resolved": "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz",
      "integrity": "sha512-..."
    },
    "my-lodash-v5": {
      "name": "lodash",
      "version": "5.0.0",
      "resolved": "https://registry.npmjs.org/lodash/-/lodash-5.0.0.tgz",
      "integrity": "sha512-..."
    }
  }
}
```

The `resolved` URL points at the registry tarball, not at a local path. When
you re-run `guroku install` with this lockfile present, guroku looks up each
entry by its local name, downloads the tarball at `resolved`, and lays it
out under the local name again. Aliased entries do not get any special flag;
the presence of a `name` that differs from the entry key is what signals an
alias.

## Common use cases

### Side-by-side migration

Pin both major versions while you migrate file by file:

```json
{
  "dependencies": {
    "react-v17": "npm:react@^17",
    "react-v18": "npm:react@^18"
  }
}
```

Files that have already been migrated import from `react-v18`; files still
on the old API import from `react-v17`. Once the last `react-v17` import is
gone, drop the alias and rename `react-v18` to plain `react`.

The same pattern works for any library where you want to land the upgrade
incrementally: `webpack-v4` / `webpack-v5`, `eslint-v8` / `eslint-v9`,
`@types/node` at two LTS lines.

### Forking a published package

When you suspect you'll need to fork a dependency but haven't yet, an alias
lets you swap implementations without changing import sites:

```json
{
  "dependencies": {
    "my-fork": "npm:upstream@^1"
  }
}
```

Your code imports `my-fork` everywhere. When the time comes to fork, you
swap the spec to a file or git dep:

```json
{
  "dependencies": {
    "my-fork": "file:./forks/my-fork"
  }
}
```

No grep-and-replace across the codebase. Import sites stay stable; only
`package.json` changes.

### Renaming a privately published package

If your private registry hosts a package whose name you no longer like, but
republishing under a new name would break every consumer, alias it locally:

```json
{
  "dependencies": {
    "auth": "npm:@acme/legacy-auth-client@^3"
  }
}
```

Your imports say `from 'auth'`. The published name stays put for
backwards-compatibility, and you can change the alias the day the old name
is finally retired.

## Caveats

A few things aliases do not do.

- **Transitive dependencies are unchanged.** The aliased package's own
  `dependencies` still resolve against the registry under their real names.
  If `lodash` depends on `some-helper`, aliasing lodash to `my-lodash-v4`
  does not give you a way to alias `some-helper` too. You would have to
  modify the aliased package's own `package.json`, which means forking it.
- **Bin shims use the published bin name.** If a package declares
  `"bin": { "real-tool": "./bin.js" }` and you install it as `my-tool`,
  the shim that lands in `node_modules/.bin` is still `real-tool`. The
  alias renames the package directory, not the bins it advertises.
- **Bin collisions go to last writer.** If you alias the same package twice
  (or two different packages that ship the same bin name), only one shim
  ends up in `.bin`. The order in which the linker processes packages
  determines which one wins; do not rely on it.
- **`peerDependencies` does not yet honor aliases.** A package that declares
  `peerDependencies: { react: "^18" }` will look for a literal `react`
  entry, not your `react-v18` alias. Peer-dep alias resolution is planned
  for a v1.x.x release; until then, satisfy peers under their real name.

## Compatibility

Alias syntax is portable. The same `npm:<name>@<spec>` form is understood
by every major package manager:

- **npm 6.9 and later** — same syntax, same semantics.
- **pnpm 6 and later** — same syntax, same semantics.
- **yarn classic (1.x)** — same syntax, with the usual caveats around
  `resolutions` overriding aliased entries.
- **bun** — same syntax, same semantics.
- **guroku v1.1** — same syntax, same semantics.

This means a `package.json` written for one of these tools installs cleanly
under any of the others. Aliases are universally portable across the major
package managers, including guroku v1.1, so you can adopt them without
worrying that you've locked yourself into a particular tool.

What does *not* port is the lockfile. Each manager has its own lockfile
format, and the way each represents an aliased entry differs in the
details. If you switch package managers you'll regenerate the lockfile
anyway, and the alias entries in `package.json` will produce equivalent
results in the new lockfile.

## FAQ

**Can I alias a git dep?**

Not yet. `npm:foo@git+https://example.com/foo.git` is not a valid spec;
the `<spec>` after the last `@` has to be a registry-resolvable string
(range, exact, or dist-tag). If you need a git dep under a different name,
use a regular git dep and rename the package in its own `package.json`
before pointing at it. We may extend the alias parser to accept git URLs
in a future release, but it is not in v1.1.

**Can I alias a file dep?**

Not yet. `npm:foo@file:./pkg` does not work for the same reason: `<spec>`
must be a registry spec. To install a local directory under a different
name, use a `file:` dep directly and adjust the local package's
`package.json#name` if you need it to resolve under a non-default name.

**Can I alias a workspace package?**

Not in v1.1. Workspace resolution happens before alias resolution, and the
two paths don't yet interleave. If you need to import a workspace package
under a different name, add a re-export shim in the consuming workspace.

**Can the same alias point at different versions in different workspaces?**

Yes. Aliases live in `package.json#dependencies` like any other entry, so
two workspaces in the same monorepo can declare `react-v18` pointing at
different `react` versions. They will lay out independently in the strict
tree and not interfere.

**How do I find what's installed under an alias?**

Read `guroku.lock`. Entries are keyed by the local (alias) name, and the
`name` field inside each entry tells you the real registry package. A grep
for the alias name in the lockfile will turn up exactly one entry, and the
`name` and `version` fields are authoritative.

**Does aliasing affect dedupe?**

No. Dedupe operates on the resolved package identity, not the local name.
If two aliases point at the same resolved version, they share a CAS entry
and (where the strict layout permits) a single tree node. If they point at
different versions, they are separate nodes regardless of what the local
names are.

**Does aliasing affect audit?**

No. `guroku audit` walks the resolved graph and reports advisories against
the real package identity. An advisory against `lodash@4.17.20` will fire
whether the package is installed as `lodash` or `my-lodash-v4`; the report
includes the local name so you can find the entry in `package.json`.

**Why is the prefix `npm:` and not something tool-specific?**

For portability. The `npm:` prefix is the de facto standard across the
ecosystem; using it means a `package.json` written for guroku installs
cleanly under npm, pnpm, yarn, and bun, and vice versa. We have no plans
to add a guroku-specific prefix.
