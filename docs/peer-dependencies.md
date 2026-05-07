# Peer Dependencies

This document describes how guroku v0.2 handles the `peerDependencies` field
in `package.json`, what you should do today as a library author, and how the
behaviour compares to other package managers.

## What peer dependencies are

A peer dependency is a package that your library expects the consumer
(the *host project* installing your library) to provide. The library does
not bundle or install the dependency itself; instead, it declares a version
range it can work against, and trusts the host to satisfy it.

The canonical example is a React component library. The library uses
React's APIs, but it must run against the same React instance that the
host application uses, otherwise hooks and context break. So the library
declares React as a peer:

```json
{
  "name": "my-button-kit",
  "version": "1.0.0",
  "peerDependencies": {
    "react": "^18.0.0"
  }
}
```

The host project then has its own `dependencies` entry for `react`, and
both `my-button-kit` and the host application share that single copy.

Peer dependencies are different from regular dependencies in two ways:

- They are not installed into the dependent's `node_modules` subtree.
- The version range expresses *compatibility*, not *what to fetch*.

## What guroku v0.2 does with them

guroku v0.2 reads the `peerDependencies` field, preserves it in the
in-memory manifest, and round-trips it back when writing manifests. It
does **not** install peers, and it does **not** resolve peer ranges
during dependency resolution.

In practical terms:

- `guroku install` ignores `peerDependencies` entirely when computing
  the dependency graph for the host project.
- If a transitive dependency declares peers, guroku does not try to
  hoist or co-locate the peer; it just records the field.
- `guroku publish` writes `peerDependencies` through to the published
  manifest unchanged.

The user's host project is expected to provide compatible versions of
each peer in its own `dependencies` (or `devDependencies`) block.

## Why we don't auto-install peers (yet)

Peer auto-installation is a policy decision, not a technical one, and
the ecosystem disagrees about the right policy:

- **npm 3 through 6** did not auto-install peers; they only warned.
- **npm 7 and later** auto-install missing peers and emit warnings on
  conflicts, but still install in many cases.
- **pnpm** validates peer ranges strictly and refuses the install when
  ranges from different consumers conflict.

Each of those policies has a defensible argument and a different blast
radius. Rather than ship a half-baked choice and change it later,
guroku v0.2 adopts the conservative "declarative-only" behaviour:
read the field, surface it, do not act on it.

Auto-install of peer dependencies is on the v0.4 roadmap. It will
reuse the same resolver and conflict reporter that handle regular
dependencies today.

## What you should do today

If you maintain a library that declares a peer dependency, take the
following steps so users on guroku v0.2 are not surprised:

1. **Document the peer in your README.** State the package and the
   range, and tell users to install it themselves.
2. **Add a runtime check** if the failure mode would otherwise be
   confusing (for example, a cryptic `Cannot find module` from deep
   inside your code):

   ```js
   try {
     require.resolve('react');
   } catch (_e) {
     throw new Error(
       'my-button-kit requires react as a peer dependency. ' +
       'Please add react to your project dependencies.'
     );
   }
   ```

3. **Add the same package to `devDependencies`** so your own tests
   and examples can run. `devDependencies` are installed normally by
   guroku and do not propagate to consumers.

A complete library manifest typically looks like this:

```json
{
  "name": "my-button-kit",
  "version": "1.0.0",
  "peerDependencies": {
    "react": "^18.0.0"
  },
  "devDependencies": {
    "react": "^18.2.0"
  }
}
```

## Optional peers

A peer dependency can be marked optional via `peerDependenciesMeta`.
This tells the package manager that the consumer is allowed to omit
the peer; the library has a code path for the missing case.

```json
{
  "name": "my-router",
  "version": "1.0.0",
  "peerDependencies": {
    "react": "^18.0.0",
    "react-dom": "^18.0.0"
  },
  "peerDependenciesMeta": {
    "react-dom": {
      "optional": true
    }
  }
}
```

guroku v0.2 reads `peerDependenciesMeta` as part of `manifest.other`
and round-trips it unchanged. The planned v0.4 auto-installer will
consult this field and skip optional peers that are not present in
the host project, instead of treating them as missing.

## What about version conflicts in peers?

When two libraries in the same host project declare peer ranges for
the same package, those ranges may not intersect. For example,
library A declares `react: ^17.0.0` and library B declares
`react: ^18.0.0`; no single React version satisfies both.

In v0.2 this conflict is invisible to guroku because peers are not
resolved. It is on you to spot the mismatch by reading the manifests
or the libraries' documentation.

When v0.4 adds peer auto-install, conflicts between peer ranges from
different libraries will be surfaced through the same
`ResolutionConflict` mechanism that already reports conflicts between
regular dependency ranges. The error message will list each library
and the range it requires, so you can decide whether to upgrade,
downgrade, or remove one of the consumers.

Until then, coordinate manually.

## Comparison with npm and pnpm

- **npm 7+** installs missing peers automatically as part of the
  regular install, and emits warnings (not errors) when peer ranges
  conflict. The install usually still succeeds.
- **pnpm** validates peer ranges strictly. If two libraries demand
  ranges that do not intersect, pnpm refuses to install and prints
  a conflict report. Optional peers are honoured.
- **guroku v0.2** behaves like npm 3 through 6: peers are recorded
  but neither installed nor validated. You are responsible for
  declaring compatible versions in your host project's
  `dependencies`.

When the v0.4 auto-installer lands, guroku will move closer to the
pnpm end of the spectrum, with strict validation and explicit
conflict errors rather than silent warnings.

## FAQ

**Can I install a package whose peer requirements aren't met?**

Yes. guroku v0.2 does not enforce peer dependency ranges, so an
install will succeed even if the host project is missing a declared
peer or has an incompatible version. The library may then fail at
runtime; that is the trade-off of the conservative policy.

**How do I see warnings?**

Run guroku with the log level raised to `warn`:

```sh
GUROKU_LOG=warn guroku install
```

In v0.2 this prints diagnostics from the resolver and the manifest
loader. Peer-related diagnostics will become more useful in v0.4
once the auto-installer participates in resolution.
