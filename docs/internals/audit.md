# `guroku audit`

Internals notes on how the `audit` subcommand talks to the npm
security-advisories endpoint, what it sends, what it parses, and where
the rough edges are in v0.5.

## What it does

`guroku audit` POSTs the set of installed packages (as recorded in the
lockfile) to:

```
<registry>/-/npm/v1/security/advisories/bulk
```

and prints the advisories the registry returns. It is the same wire
protocol as `npm audit`, minus the bookkeeping that npm performs on top
of it (severity thresholds, fix planning, JSON output, etc.).

The intent is to give CI a one-line vulnerability gate:

```sh
guroku audit && deploy.sh
```

If the audit endpoint returns no advisories, the command exits zero and
the pipeline continues. If anything is reported, exit is non-zero.

## Request body shape

The body is a JSON object whose keys are package names and whose values
are arrays of installed versions:

```json
{
  "lodash": ["4.17.20"],
  "minimist": ["1.2.5", "1.2.6"],
  "left-pad": ["1.3.0"]
}
```

It is built by walking `lockfile.packages`. Lockfile keys have the form
`<name>@<version>`, so we split on the last `@` and group versions by
name. Duplicates can occur in real lockfiles (a package present at
multiple versions because of conflicting peer ranges); we keep all of
them.

Pseudo-code:

```text
let mut body: BTreeMap<String, Vec<String>> = BTreeMap::new();
for key in lockfile.packages.keys() {
    let (name, version) = split_name_version(key);
    body.entry(name.to_string()).or_default().push(version.to_string());
}
```

`BTreeMap` is used so the request body is deterministic, which makes
debugging with `GUROKU_LOG=debug` saner.

## Response shape

The registry replies with an object keyed by package name, where each
value is a list of advisories that apply to one or more of the versions
we asked about:

```json
{
  "lodash": [
    {
      "id": 1065,
      "title": "Prototype Pollution in lodash",
      "severity": "high",
      "url": "https://github.com/advisories/GHSA-jf85-cpcp-j695",
      "vulnerable_versions": "<4.17.21",
      "patched_versions": ">=4.17.21"
    }
  ]
}
```

Packages with no advisories are simply absent from the response; we do
not see an empty array for them. That means an empty top-level object
is the "all clear" reply.

## Advisory fields

The struct we deserialize into:

```text
pub struct Advisory {
    pub id: serde_json::Value,
    pub title: String,
    pub severity: String,
    pub url: String,
    pub vulnerable_versions: String,
    pub patched_versions: String,
}
```

Notes on the fields:

- `id` is `serde_json::Value` deliberately. The npm registry sends both
  numeric advisory IDs (`1065`) and string GHSA IDs
  (`"GHSA-jf85-cpcp-j695"`), sometimes in the same response if the
  registry is mirroring multiple advisory sources. Anything narrower
  would force us to choose one and fail-deserialize the other.
- `severity` is a free-form string in the wire format. We treat it as
  opaque and just print it. When we add `--audit-level=high` (see
  backlog) we will need to define an ordering on the standard values
  `info < low < moderate < high < critical`.
- `vulnerable_versions` and `patched_versions` are semver ranges as
  strings. We display them; we do not currently parse them.
- `title` and `url` are presentation-only.

## The flow

1. **Entry point.** `commands/audit.rs` is invoked from the top-level
   CLI dispatch. It loads `guroku.lock` from the project root.
2. **Client.** It builds a `RegistryClient` via
   `RegistryClient::from_npmrc(...)`. This is important: a private-
   registry user (Verdaccio, Artifactory, Nexus, GitHub Packages) needs
   their bulk-advisory request to land on their proxy, not on
   `registry.npmjs.org`. Using `from_npmrc` means we honour
   `registry=` in `.npmrc` and any per-scope overrides.
3. **Audit call.** `audit::audit(&client, &lock)` does the actual work:
   builds the request body from the lockfile, POSTs it, parses the
   response, and returns an `AuditReport`.
4. **Body construction.** Inside `audit::audit`, we walk
   `lock.packages` exactly once, building the `BTreeMap` shown above.
5. **Wire call.** The POST goes through `http_post_json`, which is the
   same helper used elsewhere for tarball metadata fetches.
6. **Parse.** Response body is deserialized into `AuditReport`, a thin
   newtype around `BTreeMap<String, Vec<Advisory>>`.
7. **Print.** `audit::print_report(&report)` is called from the
   command. It walks the map and prints one block per advisory.

## Authentication

`http_post_json` is responsible for adding the `Authorization: Bearer
<token>` header when the request URL's host has a matching
`_authToken` entry in the resolved `.npmrc`.

This matters because many private registries proxy
`/-/npm/v1/security/advisories/bulk` from the public npm registry but
require the request itself to be authenticated. Specifically:

- **Verdaccio** with `auth.htpasswd` requires a token for any non-GET
  request, including the audit POST.
- **Artifactory** rejects unauthenticated audit requests with `401`.
- **GitHub Packages** does not currently proxy this endpoint at all
  (see Caveats), but when it has, it has required a token.

Our rule is simple: if the resolved registry URL's host has an
`_authToken`, send it; otherwise don't. We do not currently support
basic-auth for audit requests; if a registry requires it, the call will
fail with `401`.

## Exit code

The command exits non-zero when any advisories are returned. Concretely:

- `AuditReport` is empty after parsing -> exit 0.
- `AuditReport` has at least one entry with a non-empty advisory list
  -> exit 1.
- HTTP error, parse error, missing lockfile -> exit 1 with a different
  error class (`AuditFailed`, `LockfileMissing`, etc.).

Note that we do not distinguish "vulnerabilities found" from "audit
crashed" in the exit code today; both are non-zero. CI scripts that
need that distinction should use the JSON output flag (not yet
implemented; see backlog).

The intended idiom in CI is:

```sh
guroku audit && deploy.sh
```

or, for a soft gate:

```sh
guroku audit || echo "advisories present, see above"
```

## What we don't yet do

Several flags that `npm audit` has are deliberately out of scope for
v0.5 and on the v0.5.x backlog:

- `--audit-level=<severity>`. There is no way to say "only fail on
  high+". Today, any advisory at any severity is enough to fail the
  command.
- `--json`. There is no machine-readable output. CI that wants to
  parse advisories has to scrape stdout, which we strongly discourage.
- `guroku audit fix`. We do not suggest, nor apply, dependency bumps
  to clear advisories. The user has to run `guroku update <name>`
  themselves.
- Dev-vs-production filtering. `npm audit --production` lets you
  ignore advisories that only affect devDependencies. We send the full
  installed set, with no filter.

All four are tracked for v0.5.x. None of them require new wire-protocol
work; they are all transformations of the existing request and
response.

## Caveats

- **The advisory database is npm's.** This is true even for users on a
  private registry, because most private registries that implement the
  endpoint do so by proxying the public npm response. A registry that
  does not proxy `/-/npm/v1/security/advisories/bulk` will return
  `404`, which surfaces as `AuditFailed`. There is no fallback today;
  the user has to either point at a registry that supports it or skip
  the audit step.
- **`file:` and `git:` deps go on the wire.** The lockfile records
  these with a synthetic version (e.g. `file:../local-pkg@0.0.0` or a
  git SHA acting as a version). When we group by name and version we
  include them. The npm endpoint will almost always return an empty
  result for them since they are not published packages, so the
  practical effect is a slightly larger request body. We may filter
  these out in a future version, but doing so can hide real
  vulnerabilities in a vendored copy of a published package, so the
  decision is not obvious.
- **No client-side de-duplication of versions.** If a package appears
  three times in the lockfile at the same version, we will send that
  version three times. The npm endpoint tolerates this.
- **No retry.** A flaky audit endpoint will fail the command. Wrap
  with your CI's own retry if that bothers you.

## Diagnostics

Set `GUROKU_LOG=debug` to print the request URL and response status:

```sh
GUROKU_LOG=debug guroku audit
```

You will see lines like:

```
DEBUG guroku::audit POST https://registry.npmjs.org/-/npm/v1/security/advisories/bulk
DEBUG guroku::audit status 200, 3 packages with advisories
```

The request body itself is not logged at `debug` level because it can
be very large for big lockfiles; if you need it, run with
`GUROKU_LOG=trace`.

If the call fails, the error class will be one of:

- `LockfileMissing` -- no `guroku.lock` in the project root.
- `AuditFailed { status }` -- the registry returned a non-2xx status.
- `AuditParseError` -- the registry returned 2xx with a body we could
  not deserialize into `AuditReport`. Usually a sign that the registry
  is not actually proxying the npm endpoint and is returning HTML or
  some other shape.

## Related files

- `src/commands/audit.rs` -- CLI entry point.
- `src/audit.rs` -- request/response types, body construction, POST,
  parse, print.
- `src/registry/client.rs` -- `RegistryClient::from_npmrc`,
  `http_post_json`.
- `src/lockfile.rs` -- `Lockfile::packages`, the map we iterate.
