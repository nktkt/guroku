# Optional Dependencies

This document describes how guroku v0.2 handles the `optionalDependencies`
field in `package.json`, what behaviour you should expect today, and how to
work around the current limitations.

## What optional dependencies are

`optionalDependencies` is a top-level field in `package.json` that lists
packages whose installation is allowed to fail. The canonical example is
`fsevents`, a native macOS-only file-watching library that other platforms
should not attempt to build:

```json
{
  "name": "my-app",
  "version": "1.0.0",
  "dependencies": {
    "chokidar": "^3.5.0"
  },
  "optionalDependencies": {
    "fsevents": "^2.3.2"
  }
}
```

In a fully-featured npm-style package manager, the install pipeline tries
to fetch and build each optional package; if any step fails (unsupported
platform, missing toolchain, network error during a non-critical fetch),
the failure is logged but does not abort the install. Application code
then probes for the package at runtime and falls back to a portable
implementation if it is missing.

## What guroku v0.2 does

guroku v0.2 reads, preserves, and writes the `optionalDependencies` field,
but it does **not** install the packages listed in it. Specifically:

- The manifest parser recognises `optionalDependencies` and keeps its
  contents in memory alongside `dependencies` and `devDependencies`.
- The resolver explicitly skips this field when building the dependency
  graph. Optional packages do not contribute nodes or edges, and they
  are not fetched, extracted, or linked.
- The lockfile writer only records what the resolver produced, so
  optional packages do not appear in `guroku.lock`.
- The manifest writer round-trips the field unchanged. Running `guroku
  add` or `guroku remove` on a regular dependency will leave the
  `optionalDependencies` block in your `package.json` exactly as you
  wrote it (key order, formatting via the JSON pretty-printer, and all).

In short: declare them if you want, guroku will not corrupt them, but
they have no effect on what ends up in `node_modules`.

## Why it is not implemented yet

Installing optional packages correctly requires infrastructure that v0.2
does not yet have:

1. **Platform gating.** Many optionals declare `os`, `cpu`, or `libc`
   constraints. The resolver needs to evaluate these against the host
   before deciding to fetch.
2. **Fail-soft semantics.** The current install pipeline treats any
   download, checksum, extraction, or linking error as fatal. Optional
   support means classifying errors per-package and continuing past
   recoverable ones.
3. **Lifecycle scripts.** A package that fails its `install` or
   `postinstall` script must be unwound cleanly, leaving the rest of
   the tree intact. Lifecycle script support itself is on the roadmap
   but not in v0.2.

The v0.2 release prioritises a correct, deterministic install for the
regular dependency path. Optional support arrives in two stages:

- **v0.4** introduces lifecycle scripts and the per-package error
  classification needed to fail soft.
- **v0.5** introduces platform-aware install (`os` / `cpu` / `libc`
  gating) and turns optional resolution on by default.

## What you should do today

If your application genuinely depends on a package that happens to be
listed under `optionalDependencies` upstream, treat it as a regular
dependency in your own `package.json`:

```json
{
  "dependencies": {
    "fsevents": "^2.3.2"
  }
}
```

Otherwise, the recommended pattern is:

1. Declare the package under `optionalDependencies` as you normally
   would.
2. Document the runtime fallback in your code, e.g. a `try { require }`
   block that swaps in a portable implementation when the optional is
   absent.
3. Rely on guroku's round-trip behaviour: `guroku add` and `guroku
   remove` will not touch `optionalDependencies` unless you ask them
   to, so the field survives day-to-day manifest edits even though the
   resolver ignores it.

When v0.5 lands and optionals start installing, no manifest changes
will be required on your end.

## Removal behaviour

`guroku remove <pkg>` searches `dependencies`, `devDependencies`, and
`optionalDependencies` in that order, and removes the first match. You
do not need a flag to remove an optional:

```text
$ guroku remove fsevents
removed fsevents from optionalDependencies
```

If the same package name appears in more than one of the three fields
(unusual, but legal), only the first occurrence is removed; run the
command again to remove the next one. Removal updates `package.json`
in place and, for entries that were also in the lockfile, updates
`guroku.lock` as well. Since optionals are never in the lockfile in
v0.2, removing one only edits the manifest.

## Interaction with `peerDependenciesMeta.<pkg>.optional`

These two fields look related but mean different things, and guroku
treats them differently.

- `optionalDependencies` describes a package this project would like to
  install, but whose installation is allowed to fail. The contract is
  with the install pipeline: do not error if it cannot be set up.
- `peerDependenciesMeta.<pkg>.optional` marks a peer dependency that
  the consumer is allowed to leave unsatisfied. The contract is with
  the resolver: do not warn if the consumer does not provide it.

```json
{
  "peerDependencies": {
    "react": "^18.0.0"
  },
  "peerDependenciesMeta": {
    "react": { "optional": true }
  }
}
```

guroku v0.2 honours `peerDependenciesMeta.<pkg>.optional` already: a
missing optional peer does not produce a warning. This is independent
of the `optionalDependencies` story above.

## FAQ

**Will my optional deps end up in the lockfile?**
No. The resolver skips them, so they are not recorded in `guroku.lock`
in v0.2.

**Will `--frozen-lockfile` complain about a missing optional?**
No. `--frozen-lockfile` checks that the lockfile fully covers the
resolved dependency graph; since optionals are not part of that graph
in v0.2, they are simply outside its scope. You can run with
`--frozen-lockfile` even if your manifest lists optionals that are not
in the lockfile.

**When will this actually work?**
v0.4 brings the lifecycle-script and fail-soft infrastructure, and v0.5
turns on platform-aware optional installs by default. Until then,
declare optionals for forward compatibility but do not rely on them
being present in `node_modules`.
