# The `guroku.lock` Lockfile

This document describes the on-disk format and semantics of `guroku.lock`,
the file guroku writes after a successful resolution. It is aimed at
contributors and at users who want to understand exactly what guroku is
recording on their behalf.

## Why a lockfile exists

A manifest such as `package.json` declares dependencies as *ranges*:

```json
{
  "dependencies": {
    "lodash": "^1.2.3"
  }
}
```

A range like `^1.2.3` is a statement of *intent*: "any 1.x release at or
above 1.2.3 is acceptable to me." It is not a statement of fact about
which version is currently installed. Two installs run a week apart, on
two different machines, against the same `package.json` can therefore
end up with different bytes on disk.

That is fine for an exploratory `guroku add` but unacceptable for
reproducible builds, CI pipelines, or a teammate trying to reproduce a
bug on a Friday afternoon. The lockfile is guroku's answer: it records
the resolver's *output* — the exact versions chosen, where their tarballs
came from, and what they hash to — so that the next install gets the
same bytes.

In short: the manifest says what you want, the lockfile says what you got.

## Format and where it lives

The lockfile is a JSON file written to `<project>/guroku.lock`, next to
`package.json`. It has three top-level keys: `lockfileVersion`,
`generatedBy`, and `packages`.

A complete (if minimal) example:

```json
{
  "lockfileVersion": 1,
  "generatedBy": "guroku 0.2.0",
  "packages": {
    "lodash@4.17.21": {
      "resolved": "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz",
      "integrity": "sha512-v2kDEe57lecTulaDIuNTPy3Ry4gLGJ6Z1O3vE1krgXZNrsQ+LFTGHVxVjcXPs17LhbZVGedAJv8XZ1tvj5FvSg==",
      "dependencies": {}
    }
  }
}
```

Keys in `packages` are exact `name@version` strings. The value for each
key is a `PackageLock` record (see schema below).

## Schema details

```text
Lockfile
├── lockfileVersion : u32
├── generatedBy    : String
└── packages       : Map<String, PackageLock>

PackageLock
├── resolved     : String
├── integrity    : Option<String>
└── dependencies : Map<String, String>
```

### `lockfileVersion: u32`

The current value is `1`. A mismatch between what the on-disk lockfile
declares and what this build of guroku understands is a hard error
(`LockfileVersionMismatch`). We do not auto-migrate. The reasoning:

- Auto-migrating silently rewrites a file that the user committed to
  source control. We would rather surface the difference and let the
  user decide.
- A migration that moves forward also has to be sound moving backward
  (so that an older guroku in CI doesn't blow up on a newer file).
  We're not paying that complexity tax in v0.x.

### `generatedBy: String`

A human-readable string identifying the writer (currently
`"guroku <version>"`). It is informational only — guroku makes no
parsing or compatibility decisions based on its contents. If you fork
guroku, change it. If you copy a lockfile from another machine, leaving
it untouched is fine.

### `packages: Map<String, PackageLock>`

The map key is the exact `name@version` of a resolved package, e.g.
`lodash@4.17.21`. There is exactly one entry per resolved package.

Each `PackageLock` has:

- **`resolved: String`** — the URL of the tarball that was fetched. For
  the public npm registry this is the `dist.tarball` field returned by
  the registry; for a private registry it will point there instead.
- **`integrity: Option<String>`** — a Subresource Integrity (SRI) string,
  currently always `sha512-...`. Omitted (i.e. the field is absent from
  the JSON, not present-as-null) if the registry did not provide one.
  See `integrity.md` for how this is verified.
- **`dependencies: Map<String, String>`** — the package's own
  dependencies, as a map from dependency *name* to *exact resolved
  version*. These are not ranges. See "Why exact versions" below.

## Why JSON, not YAML/TOML/binary

JSON wins on a few axes that matter to us at v0.2:

- **Cheap to implement.** `serde_json` gives us reading, writing, and
  pretty-printing for free, and it's already in our tree for talking
  to the registry.
- **Human-readable.** When something goes wrong, you should be able to
  open `guroku.lock` in any editor and see what guroku decided. A
  binary format would force every debugging session through a tool.
- **Diff-friendly.** Code review on a lockfile change is a real
  workflow. JSON, pretty-printed with stable key ordering, produces
  diffs reviewers can actually read.
- **Ecosystem fit.** The npm world already speaks JSON. Users coming
  from `package-lock.json` will not be surprised.

We write the file with `serde_json::to_string_pretty` and append a
trailing newline so editors that auto-add one don't fight us on every
save.

YAML was rejected because of its parser surface area (anchors, tags,
the Norway problem) and TOML because nested maps in TOML are awkward
enough to read that we'd lose the human-readable advantage.

## Why exact versions inside `dependencies`

A subtle but important point: the `dependencies` map inside each
`PackageLock` records *resolved* versions, not the ranges the upstream
package declared. If `lodash@4.17.21` depends on `foo@^1.0.0` and
resolution picked `foo@1.4.2`, the lockfile contains:

```json
"lodash@4.17.21": {
  "dependencies": { "foo": "1.4.2" }
}
```

not `"foo": "^1.0.0"`.

This is a deliberate consequence of what the lockfile *is*: the output
of resolution. If we kept ranges here, `guroku install --frozen-lockfile`
could not be a pure "fetch what's listed" operation — it would have to
re-run resolution to discover what each range actually meant *this
time*, defeating the lockfile's purpose.

## What's not in the lockfile (yet)

The v1 schema deliberately omits several things. They are on the
roadmap, not oversights:

- **Per-package install hooks.** Lifecycle scripts are not yet
  recorded. We'll need this once we support running them.
- **Resolved peerDependencies graph.** We currently do not resolve
  peer dependencies, so there is nothing to record.
- **Per-file integrity.** We hash the tarball as a whole. We do not
  record file-by-file hashes.
- **Signing / provenance.** No signatures, no attestations.
- **Top-level integrity hash of the manifest+lock pair.** A single
  hash that detects "someone hand-edited one of these two files" is
  planned for v0.3.

If you need any of these *right now*, please open an issue describing
the use case before we settle the schema.

## `--frozen-lockfile`

`guroku install --frozen-lockfile` refuses to refresh the lockfile.
Specifically:

- If `guroku.lock` is missing, error with `LockfileOutOfDate`.
- If any root dependency in `package.json` is not covered by the
  lockfile, error with `LockfileOutOfDate`.
- Otherwise, install exactly what the lockfile lists, verifying
  integrity as we go.

CI should always run with `--frozen-lockfile`. A green CI build that
silently regenerated the lockfile is a green CI build that lied about
what it tested.

## Compatibility with npm / pnpm / yarn lockfiles

guroku does not read `package-lock.json`, `pnpm-lock.yaml`, or
`yarn.lock`. The semantic gaps (workspaces, hoisting strategies, peer
dependency policy) are large enough that pretending to consume one of
these files would mislead users about what guroku is actually doing.

A migration helper that ingests an existing lockfile, runs guroku's
resolver, and emits a `guroku.lock` is on the roadmap. Until that
ships, the answer is `guroku install` against your `package.json`.

## Versioning the format

`lockfileVersion` is bumped only for *incompatible* schema changes —
for example, adding a required field, changing the meaning of an
existing field, or removing a field that older guroku relied on.

Additive changes that older guroku can safely ignore (such as a new
optional field on `PackageLock`) do not bump the version. Older
guroku will round-trip the unknown field if it can, or drop it on
rewrite; users who rewrite their lockfile with an older guroku get
the older shape, and that's expected.

When we do bump the version, the release notes will call it out in
bold, and `LockfileVersionMismatch` will name both the version on
disk and the version this build understands.
