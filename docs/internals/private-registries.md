# Private Registries

This document describes how guroku v0.5 talks to non-default npm registries:
custom default registries, per-scope registries, and bearer-token authentication.
The implementation is intentionally narrow — it covers the cases most teams need
to ship internal packages alongside the public registry, and explicitly defers
the older or rarer npm authentication mechanisms.

The code lives in two files:

- `src/registry.rs` — `RegistryClient`, routing, auth header injection.
- `src/npmrc.rs` — `.npmrc` parser, key/value lookup helpers.

## What v0.5 supports

There are three independent moving parts. They can be used in any combination.

### 1. Default-registry override

A bare `registry=` line in `.npmrc` replaces the built-in default
(`https://registry.npmjs.org`) for every package that does not match a more
specific scope rule.

```
registry=https://npm.example.internal
```

### 2. Per-scope routing

A `<scope>:registry=` line routes any package whose name begins with that
scope to a different registry. The scope must include the leading `@`.

```
@acme:registry=https://npm.acme.internal
@infra:registry=https://npm.infra.internal
```

Scopes are matched literally; there is no wildcard or pattern support.

### 3. Bearer-token authentication

A `//<host>/:_authToken=` line attaches a bearer token to every request
guroku makes to that host. The host is matched on the URL's host field,
ignoring scheme, port, and path.

```
//npm.acme.internal/:_authToken=hunter2
```

All three live in `src/registry.rs` and `src/npmrc.rs`. The npmrc parser
treats them as opaque key/value pairs; the registry client interprets them
at request time.

## Routing logic

`RegistryClient::registry_for(name)` decides which base URL a given package
name should be fetched from.

1. If `name` starts with `@`, extract the scope (everything up to the first
   `/`). Otherwise the name is unscoped and step 2 is skipped.
2. If `<scope>:registry=` is set in the loaded `.npmrc`, parse that value
   as a URL and return it.
3. Otherwise fall back to the default base URL — either the `registry=`
   override or, if that's absent too, the built-in npmjs.org default.

The function returns a borrowed `Url`. Callers that need an owned value
clone it; this is cheap and keeps the lookup pure.

```rust
// Pseudocode
fn registry_for(&self, name: &str) -> &Url {
    if let Some(scope) = name.strip_prefix('@').and_then(|s| s.split('/').next()) {
        if let Some(url) = self.npmrc.scope_registry(&format!("@{scope}")) {
            return url;
        }
    }
    &self.default_registry
}
```

## Auth lookup

`RegistryClient::auth_for(url)` decides whether to attach an `Authorization`
header to a request, given the fully-resolved request URL.

1. Take the URL's host string (e.g. `npm.acme.internal`). Port is part of
   the host key in the npmrc syntax but guroku currently ignores it; the
   default port for the scheme is assumed.
2. Call `npmrc.auth_token(host)`, which does an exact host-string lookup.
3. If `Some(token)`, send `Authorization: Bearer <token>`. If `None`, send
   no auth header — the request goes out anonymously.

The same lookup runs for every request, including redirects to a different
host (the tarball case below). guroku never sends a token to a host other
than the one configured for it.

## Worked example

Given this `.npmrc`:

```
registry=https://registry.npmjs.org
@acme:registry=https://npm.acme.internal
//npm.acme.internal/:_authToken=hunter2
```

The flow looks like:

- `guroku install lodash`
  - `registry_for("lodash")` → `https://registry.npmjs.org` (default).
  - `auth_for("https://registry.npmjs.org/lodash")` → no token configured
    for `registry.npmjs.org`, no auth header.
  - Tarball URL from the metadata is also on `registry.npmjs.org` — still
    no auth.

- `guroku install @acme/widget`
  - `registry_for("@acme/widget")` → `https://npm.acme.internal` (scope rule).
  - `auth_for("https://npm.acme.internal/@acme/widget")` → token found,
    sends `Authorization: Bearer hunter2`.
  - The tarball URL listed in the metadata typically points at the same
    host (e.g. `https://npm.acme.internal/@acme/widget/-/widget-1.0.0.tgz`).
    `auth_for` is called again for the tarball URL, finds the same host,
    and attaches the same bearer token.

If the metadata for `@acme/widget` happened to list a tarball on a third
host that has no token configured, that fetch would go out anonymously.
This is intentional: the auth table is keyed by host, not by package.

## Where this gets called

Auth and routing run for every outgoing HTTP request from `RegistryClient`:

- `fetch_metadata(name)` — cached metadata lookup.
- `fetch_metadata_uncached(name)` — bypasses the on-disk HTTP cache; same
  routing and auth.
- `fetch_tarball(url)` — direct fetch of a tarball URL from metadata.
- `http_post_json(url, body)` — used by `guroku audit` to POST the
  advisories request.

There is no other code path that issues a registry HTTP request. Anything
that needs the registry goes through one of these four functions, so
adding a new auth scheme later means changing one place.

## What we don't yet support

The following keys are recognized by the npmrc parser as unknown but
otherwise ignored. None of them are wired into request building.

- `auth=<base64-of-user:pass>` — npm's classic Basic auth. Common in
  older Artifactory and Nexus setups.
- `always-auth=true` — force the auth header on every request, including
  un-scoped packages on the default registry. Today guroku only attaches
  auth when a host-specific token is configured.
- `_password=<base64>` + `username=` — the npm v6 holdover form. Equivalent
  to `auth=` but split into two keys.
- Multiple `_authToken` entries for the same host with different paths
  (`//host/path/:_authToken=...`). The path component is parsed but not
  used for matching; the last entry for a host wins.
- Reading credentials from `npm_config_*` environment variables. These
  are an npm CLI convention; guroku reads `.npmrc` only.

All of the above are scheduled for v0.5.x or v0.6 depending on demand.

## The audit endpoint

`guroku audit` POSTs to:

```
<registry-base>/-/npm/v1/security/advisories/bulk
```

The base URL is the default registry — audit does not split per-scope. If
you have packages from multiple registries in your tree, only the default
registry's advisory database is consulted.

Auth header propagation works through the same `auth_for(url)` lookup as
metadata and tarballs: if the audit URL's host has a token configured,
the POST goes out with `Authorization: Bearer <token>`. Otherwise it's
anonymous.

## Self-hosted registry compatibility

guroku has been tested against the following self-hosted registries.
"Works" means metadata and tarball fetching succeed and lockfiles round-trip.

- **Verdaccio** — works. It speaks the npm registry API natively, so
  bearer tokens and the standard endpoints behave exactly like npmjs.org.
- **GitHub Packages** — works for metadata. Tarball fetches may need
  extra headers (`Accept: application/octet-stream`) on certain account
  tiers; this is not yet wired into `fetch_tarball`. Workaround: mirror
  the package locally, or wait for the v0.5.x fix.
- **JFrog Artifactory** — works for metadata and bearer-token auth.
  The advisories endpoint is not proxied by default, so `guroku audit`
  against an Artifactory-only setup will return 404. Either point audit
  at npmjs.org explicitly, or enable the advisories proxy in Artifactory.
- **Sonatype Nexus** — same situation as Artifactory: metadata and auth
  work, advisories endpoint is not proxied by default.

If you are running a registry not on this list and it implements the
standard npm registry API, it will most likely work. File an issue with
the registry name and a sample `.npmrc` if it doesn't.

## Diagnostics

When something goes wrong, the two most useful tools are guroku's own
debug log and a manual `curl`.

```sh
GUROKU_LOG=debug guroku install
```

This prints every registry URL guroku resolves, every cache hit and miss,
and the response status. The `Authorization` header is redacted in the
log output — only the presence of a token is shown, never its value.

```sh
curl -v -H "Authorization: Bearer <token>" https://npm.acme.internal/@acme/widget
```

Use this to confirm that the registry accepts your token directly. If
`curl` succeeds but guroku fails, the problem is on guroku's side
(routing, host matching, or .npmrc parsing). If `curl` also fails, the
problem is on the registry side.

A common failure mode is a token configured against the wrong host
spelling — for example `npm.acme.internal` in the registry URL but
`//npm.acme.internal:443/:_authToken=...` in the auth line. guroku
strips the explicit `:443` for HTTPS, but other port mismatches are not
normalized.

## Security notes

See `docs/internals/security-model.md` for guroku's overall threat model.

A few notes specific to private registries:

- Tokens are treated as read-only on guroku's side. guroku never writes
  to `.npmrc`, never refreshes tokens, and never warns when a token
  expires. A 401 from the registry surfaces as a generic HTTP error.
- There is no token rotation or expiry detection. If your registry uses
  short-lived tokens, you'll need to refresh them out-of-band (e.g. via
  a CI step that rewrites `.npmrc` before invoking guroku).
- Tokens are read from disk on every `RegistryClient` construction.
  Long-running guroku invocations do not re-read `.npmrc` mid-run.
- `.npmrc` is not encrypted. Treat it like any other file holding
  credentials: restrict its permissions and keep it out of version
  control.
