# The `guroku.lock` file

This is a user-facing reference for the `guroku.lock` file. If you are
contributing to guroku itself and want to know how the lockfile is parsed,
serialised, and validated, read `docs/internals/lockfile.md` instead.

## Quick answer

`guroku.lock` is committed to your repository. It records the exact versions
guroku resolved for every dependency in your `package.json` (direct and
transitive), along with the URL each tarball was fetched from and an integrity
hash. With it, anyone who runs `guroku install` against the same lockfile
gets bit-for-bit the same `node_modules` tree, regardless of when registry
state changes underneath them.

If you have ever worked with `package-lock.json`, the mental model is the
same. The file format is different, but the role is identical.

## Should I commit it?

Yes, with the same nuances as the npm ecosystem:

- **Applications** (anything that gets deployed, run, or shipped as a
  finished artefact): commit it. This is the whole point of having a
  lockfile.
- **Libraries that ship binaries** (CLIs, native addons, anything where the
  install graph affects what users actually run): commit it. The binary you
  publish should be reproducible.
- **Libraries that publish source only** (a typical npm package consumed by
  other projects): committing is optional. Downstream consumers will
  re-resolve against their own constraints, so your lockfile is only used
  during your own development and CI. Many maintainers commit it anyway for
  consistent CI runs; some do not because it adds churn to PRs.

If in doubt: commit it. The cost is a tracked file; the benefit is
reproducibility.

## Anatomy of an entry

Every resolved package appears under the top-level `packages` map, keyed by
`name@version`:

```json
"lodash@4.17.21": {
  "resolved": "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz",
  "integrity": "sha512-...",
  "dependencies": {}
}
```

Field by field:

- **The key, `lodash@4.17.21`** — the package's registered name and the exact
  version guroku resolved. There is exactly one entry per `name@version`
  pair; if two parts of your graph need different versions of the same
  package, you get two entries.
- **`resolved`** — the absolute URL of the tarball. This pins not just the
  version number but also the source. If you switch registries, this
  changes.
- **`integrity`** — a Subresource Integrity hash (currently SHA-512, prefixed
  `sha512-`). On install, guroku recomputes this against the downloaded
  tarball and refuses to use the cache if it does not match. This is how
  the lockfile defends against a tampered registry or a corrupted mirror.
- **`dependencies`** — a map of the runtime dependencies of *this* package,
  keyed by package name, valued by the exact resolved version. An empty
  object means the package has no runtime dependencies. These pointers let
  guroku reconstruct the dependency graph without re-resolving.

`devDependencies` and `peerDependencies` of transitive packages are not
recorded — only what would be installed for a production-style install of
that subgraph. Your top-level dev dependencies appear at the top of the
graph as normal entries.

## What's the `lockfileVersion` field?

The file starts with a small header:

```json
{
  "lockfileVersion": 1,
  "packages": { ... }
}
```

`lockfileVersion` is currently `1`. It exists so that guroku can evolve the
format without silently misreading old lockfiles, or producing new ones that
older guroku binaries cannot parse. If a future guroku release bumps the
version, running `guroku install` will rewrite your lockfile in place;
running an older guroku against a newer lockfile will refuse with a clear
error.

Treat the version as opaque metadata. Do not change it by hand unless you
have a very specific reason and you know what you are doing.

## What changes when?

The lockfile only changes in response to specific commands:

- **`guroku add <pkg>`** — resolves the new package, adds entries for it
  and any new transitive deps, and writes the file.
- **`guroku remove <pkg>`** — drops the package and any transitive entries
  no longer reachable, then writes the file.
- **`guroku install`** (without `--frozen-lockfile`) — reconciles the
  lockfile with `package.json`. If you edited a range in `package.json`
  manually, this is what picks it up.
- **`guroku update [pkg]`** — re-resolves within the existing ranges and
  records the result.

The lockfile does **not** change when you only edit `package.json`. Until
you run `guroku install`, the on-disk lockfile may disagree with your
declared ranges. That is intentional: it lets you stage edits and apply
them deliberately.

## CI usage

In CI you almost always want:

```sh
guroku install --frozen-lockfile
```

With `--frozen-lockfile`, guroku will:

1. Refuse to modify `guroku.lock`.
2. Fail loudly if `package.json` and `guroku.lock` would disagree (a new
   dependency was added, a range no longer matches the locked version, etc.)

If this fails on a CI run, that is the point — it means a developer
forgot to commit an updated lockfile, and you would otherwise have shipped
something different from what they tested locally.

For a typical GitHub Actions step:

```yaml
- run: guroku install --frozen-lockfile
```

For local development the bare `guroku install` is fine; it will update the
lockfile if needed.

## Diff hygiene

`guroku.lock` is designed to be reviewable in pull requests:

- **Stable ordering.** The `packages` map is serialised from a `BTreeMap`,
  so entries are written in alphabetical order by key. Adding a single
  package produces a localised diff, not a reshuffle of the whole file.
- **Pretty-printed JSON.** Two-space indentation, one field per line, keys
  in a stable order within each entry (`resolved`, `integrity`,
  `dependencies`).
- **No trailing whitespace, trailing newline at EOF.** Plays nicely with
  the usual `.gitattributes` and editor config.

If you see a lockfile diff that looks much larger than the change warrants,
it is almost always one of:

- Someone ran `guroku install` without `--frozen-lockfile` on a stale
  branch and pulled in incidental updates.
- The `lockfileVersion` was bumped (one-time migration).
- Someone hand-edited the file and broke the ordering.

## Comparison with other lockfiles

| Feature                   | guroku.lock          | package-lock.json   | pnpm-lock.yaml      | yarn.lock            |
|---------------------------|----------------------|---------------------|---------------------|----------------------|
| Format                    | JSON                 | JSON                | YAML                | custom (YAML-ish)    |
| Top-level version field   | `lockfileVersion: 1` | `lockfileVersion`   | `lockfileVersion`   | header comment       |
| Integrity hashes          | SHA-512 (SRI)        | SRI                 | SRI                 | SRI (newer versions) |
| Records transitive deps   | yes                  | yes                 | yes                 | yes                  |
| Stable ordering           | alphabetical         | alphabetical        | alphabetical        | alphabetical         |
| Workspace/monorepo aware  | not yet              | yes                 | yes                 | yes                  |
| Per-package overrides     | not yet              | yes                 | yes                 | yes (resolutions)    |

guroku is **not compatible** with any of the above. You cannot rename
`package-lock.json` to `guroku.lock` and expect it to work, and the reverse
is also true. An importer that reads `package-lock.json` and produces a
`guroku.lock` is on the roadmap but not implemented.

## FAQ

### Can I edit it by hand?

Don't. The lockfile is a derived artefact; the source of truth is
`package.json` plus the registry. If you find yourself wanting to edit it,
there is almost always a better tool: `guroku add`, `guroku remove`,
`guroku update`, or pinning a range in `package.json`.

If you genuinely must (debugging, recovery, an emergency hotfix), keep
the `lockfileVersion` correct, preserve alphabetical ordering of
`packages`, and run `guroku install` afterwards to validate. guroku will
reject a malformed lockfile rather than silently work around it.

### What if it's out of date?

Run `guroku install` without `--frozen-lockfile`. That reconciles the
lockfile against `package.json` and writes the result. Commit the diff.

If you want to refresh everything within your declared ranges (rather than
just satisfying current ranges), use `guroku update`.

### Why does my git diff churn?

The most common cause is someone running `guroku install` (without
`--frozen-lockfile`) on a different machine where the version ranges in
`package.json` resolve differently — typically because a new patch release
was published since the last install. The fix is one of:

- Pin the offending package more tightly in `package.json` (e.g. `~1.2.3`
  instead of `^1.2.3`) if you want to avoid drift on patch bumps.
- Coordinate so a single person regenerates the lockfile, commits it, and
  others use `--frozen-lockfile`.
- Run `--frozen-lockfile` in CI so unintended drift is caught immediately
  instead of being silently committed.

If churn is happening with no apparent change to `package.json` and no new
upstream releases, file an issue — that should not happen.
