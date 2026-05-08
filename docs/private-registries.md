# Private Registries

guroku speaks the standard npm registry protocol, so any server that
implements it (Verdaccio, JFrog Artifactory, Sonatype Nexus, GitHub
Packages, npm Enterprise, etc.) can be used in place of, or alongside,
the public registry at `https://registry.npmjs.org`.

This page covers how to point guroku at a private registry, how to mix
public and private sources, and the rough edges you are likely to hit
on each major server.

All configuration uses the same `.npmrc` format that npm itself reads,
so existing setups should keep working unchanged.

---

## 1. Quick recipe

The shortest possible setup: put two lines in `~/.npmrc`.

```
# ~/.npmrc
registry=https://npm.acme.internal
//npm.acme.internal/:_authToken=hunter2
```

The first line replaces the default registry. The second line attaches
a bearer token to every request that goes to `npm.acme.internal`.
guroku reads the token and sends it as `Authorization: Bearer hunter2`
on metadata requests, tarball downloads, and audit calls.

After that, you do not need to pass any flags:

```sh
guroku install
```

Every package in `package.json` (and every transitive dependency) is
fetched from `npm.acme.internal`. If your registry mirrors the public
registry, this is all you need.

---

## 2. Mixing public and private

Most teams do not want to mirror the entire public registry. The
standard pattern is: keep the public registry as the default, and
override only your own scope.

```
registry=https://registry.npmjs.org
@acme:registry=https://npm.acme.internal
//npm.acme.internal/:_authToken=hunter2
```

With this config:

- `lodash` resolves against `https://registry.npmjs.org` (no auth).
- `@acme/widget` resolves against `https://npm.acme.internal`
  (bearer auth from the `_authToken` line).
- `@acme/anything-else` also goes to the private registry, because
  the `@acme:registry=` line matches the entire scope.

You can have as many scope overrides as you need:

```
registry=https://registry.npmjs.org
@acme:registry=https://npm.acme.internal
@partner:registry=https://npm.partner.example.com
//npm.acme.internal/:_authToken=hunter2
//npm.partner.example.com/:_authToken=othertoken
```

guroku picks the most specific match per package: a scope-level
`registry=` always wins over the global `registry=`.

### Auth scoping

Auth tokens are matched on host (and port, if non-default). You do
**not** need to repeat them per scope; one `//host/:_authToken=` line
covers every request that goes to that host.

If you have two registries on the same host (different paths), you
will need to use a more specific prefix:

```
//npm.acme.internal/team-a/:_authToken=token-a
//npm.acme.internal/team-b/:_authToken=token-b
```

guroku uses the longest matching prefix.

---

## 3. Per-project config

guroku reads `.npmrc` from three places, in order of decreasing
priority:

1. `<project-root>/.npmrc` — next to `package.json`.
2. `~/.npmrc` — your home directory.
3. Built-in defaults (public registry, no auth).

Drop a `.npmrc` in your project root and it overrides anything in
your home directory for that project only:

```
# my-project/.npmrc
registry=https://npm.acme.internal
//npm.acme.internal/:_authToken=${ACME_NPM_TOKEN}
```

This is especially useful for repos that talk to a single corporate
registry: every contributor gets the right setup just by cloning,
and CI can inject the token via `ACME_NPM_TOKEN`.

guroku expands `${VAR}` references against the process environment
when loading `.npmrc`. Unset variables expand to the empty string;
guroku will warn (`W003: empty auth token for <host>`) and continue.

### What not to commit

Commit `.npmrc` files that reference tokens via environment variables.
Do **not** commit `.npmrc` files with literal tokens — guroku will
load them happily, but anyone with read access to the repo gets a
free token.

A typical `.gitignore` snippet:

```
# allow committed .npmrc, but not local overrides
.npmrc.local
```

And in your committed `.npmrc`:

```
registry=https://npm.acme.internal
//npm.acme.internal/:_authToken=${ACME_NPM_TOKEN}
```

---

## 4. Compatibility table

guroku v0.5 has been smoke-tested against the following servers.
"Metadata" means the `GET /<pkg>` packument endpoint. "Tarballs" means
the `GET /<pkg>/-/<pkg>-<ver>.tgz` endpoint. "Audit" means the
`POST /-/npm/v1/security/advisories/bulk` endpoint that `guroku audit`
uses.

| Server | Metadata | Tarballs | Audit (`/-/npm/v1/security/advisories/bulk`) |
|---|---|---|---|
| Verdaccio | yes | yes | partial (proxies if configured) |
| Artifactory | yes | yes | typically not proxied |
| Sonatype Nexus | yes | yes | typically not proxied |
| GitHub Packages | yes | yes (with auth) | not proxied |
| npm.com | yes | yes | yes |

"Partial" or "not proxied" does not mean guroku is broken — it means
the server returns 404 or 501 for the audit endpoint and `guroku
audit` will fail. Install, lockfile resolution, and tarball download
all work fine. See section 5 for the workaround.

If you have tested guroku against another server (Cloudsmith, AWS
CodeArtifact, Google Artifact Registry, etc.) we would like to hear
about it. Open an issue with the server name, version, and which of
the three columns work.

---

## 5. Audit on private registries

`guroku audit` performs a single POST to:

```
<registry>/-/npm/v1/security/advisories/bulk
```

with a JSON body listing every `name@version` in your lockfile. The
public registry implements this; many private registries do not. When
the registry returns 404, guroku exits with:

```
AuditFailed: HTTP 404 from https://npm.acme.internal/-/npm/v1/security/advisories/bulk
```

(or 501, or 405, depending on the server).

### Workaround: audit against npmjs.org

The bulk advisories endpoint does not require auth and does not
require packages to actually exist on npmjs.org — it just looks them
up by name and version. So you can temporarily point `registry=` at
npmjs.org just for the audit:

```sh
GURO_REGISTRY=https://registry.npmjs.org guroku audit
```

Or, if you prefer config files, drop a one-line override in
`./.npmrc.audit` and run:

```sh
guroku --npmrc=./.npmrc.audit audit
```

This is a stopgap. The right long-term fix is to ask your registry
vendor to proxy the advisories endpoint. Verdaccio supports this via
the `audit` plugin; Artifactory and Nexus do not, as of this writing.

### CI advice

If you run `guroku audit` in CI against a private registry that
does not proxy advisories, the job will fail every run. Either:

- run `audit` as a separate step against npmjs.org, or
- skip `audit` in CI and run it locally before releases.

`guroku audit --allow-missing-endpoint` will downgrade the 404 to a
warning and exit 0. Use it sparingly — it also masks real outages.

---

## 6. Tarball auth gotchas

Most registries serve tarballs from the same host as metadata, with
the same auth, and guroku handles this transparently. There are two
common deviations.

### Different host for tarballs

Some registries return packument JSON where each version's `dist.tarball`
points at a different host (e.g. a CDN). guroku will follow the URL
verbatim. If that host needs its own auth, add a second
`//host/:_authToken=` line:

```
registry=https://npm.acme.internal
//npm.acme.internal/:_authToken=hunter2
//cdn.acme.internal/:_authToken=hunter2
```

guroku does **not** automatically forward the metadata token to
arbitrary tarball hosts.

### Custom Accept header

GitHub Packages requires `Accept: application/octet-stream` on tarball
downloads. Without it, you get a 415 or a 302 to a login page. Some
Artifactory configurations have similar quirks.

guroku v0.5 does **not** yet send custom `Accept` headers on tarball
requests. If your registry needs one, the install will fail with:

```
TarballFetchFailed: HTTP 415 for <pkg>@<ver>
```

or, more confusingly:

```
TarballFetchFailed: invalid gzip header (got HTML)
```

(when the registry returns an error page with `Content-Type: text/html`
instead of a real tarball).

The fix is in v0.6, which adds per-host `accept` config:

```
//npm.pkg.github.com/:accept=application/octet-stream
```

Until then, if your registry needs an `Accept` header, **file an
issue** with the server name and version. We are tracking demand to
prioritize the v0.6 work.

---

## 7. Self-hosted Verdaccio quickstart

Verdaccio is the easiest way to try a private registry locally. It
runs as a single Node process, proxies the public registry by default,
and stores anything you publish to it under `~/.local/share/verdaccio`.

```sh
npx verdaccio --listen 4873
# In another shell:
npm adduser --registry http://localhost:4873   # use any creds
cat >> ~/.npmrc <<'EOF'
registry=http://localhost:4873
//localhost:4873/:_authToken=$(npm token create | grep token | awk '{print $3}')
EOF
cd my-project
guroku install
```

After that, every `guroku install` goes through Verdaccio. Public
packages are fetched on demand and cached locally; you can publish
private packages with `npm publish --registry http://localhost:4873`.

### Verifying it worked

```sh
guroku install --verbose 2>&1 | head -20
```

You should see fetch URLs of the form `http://localhost:4873/...`
rather than `https://registry.npmjs.org/...`. If you still see the
public registry, your `.npmrc` is not being picked up — check
`guroku config list` for the resolved registry.

### Verdaccio + audit

Verdaccio's bundled config does not proxy the advisories endpoint.
To enable it, add to your `verdaccio.yaml`:

```yaml
middlewares:
  audit:
    enabled: true
```

Then `guroku audit` works against `http://localhost:4873`.

---

## 8. Common errors

### `401 Unauthorized`

The most common cause is a missing or wrong token, but the second
most common cause is a host mismatch between the `registry=` line
and the `//host/:_authToken=` line.

```
registry=https://npm.acme.internal:443
//npm.acme.internal/:_authToken=hunter2
```

The `:443` on the registry line and its absence on the auth line is
enough for guroku to consider these different hosts. Make them match
exactly:

```
registry=https://npm.acme.internal
//npm.acme.internal/:_authToken=hunter2
```

Other things to check:

- Token has not expired. Many registries issue tokens with a TTL.
- Token has the right scope (read at minimum; publish if you publish).
- The `//host/` prefix exactly matches your registry's host:port.
- No stray whitespace at the end of the `_authToken=` line.

`guroku config list` prints the resolved registry and a redacted
view of which hosts have tokens; useful for debugging.

### `404 from advisories endpoint`

```
AuditFailed: HTTP 404 from https://npm.acme.internal/-/npm/v1/security/advisories/bulk
```

Your registry does not proxy advisories. See section 5 for the
workaround (point `registry=` at npmjs.org just for the audit).

This error only ever comes from `guroku audit`. If you see it during
`guroku install`, please file a bug — install should never hit the
advisories endpoint.

### `Network error: error sending request`

```
Network error: error sending request for url (https://npm.acme.internal/lodash)
```

Your registry is unreachable. This is almost always a DNS, firewall,
or VPN issue rather than a guroku bug. Check with:

```sh
curl -v https://npm.acme.internal/
```

If `curl` also fails, it is a connectivity problem. If `curl`
succeeds and `guroku` does not, check for proxy environment variables
(`HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY`) — guroku honors them, and a
stale value can route your requests into the void.

### `error decoding response body: invalid JSON`

The registry returned something that is not a packument. Usually
this is an HTML error page from a misconfigured reverse proxy (the
proxy intercepted the request before it reached the registry). Run
with `--verbose` to see the raw response body.

### `unsupported scheme: file://`

You have a `dist.tarball` URL in a packument that points at a
`file://` path. Some old Artifactory mirrors do this. guroku refuses
to fetch `file://` URLs over the registry protocol; ask your
registry admin to fix the metadata.

---

## 9. Future work

Several improvements are planned for v0.6 and beyond:

- Per-host `Accept` and other custom headers (see section 6).
- Better error messages when the registry returns HTML instead of JSON.
- A `guroku registry test <url>` command that smoke-tests metadata,
  tarball, and audit endpoints against a registry and prints a
  compatibility report.
- AWS CodeArtifact and Google Artifact Registry token-refresh helpers.

For the implementation-level view of how guroku resolves registries,
matches auth tokens, and dispatches HTTP requests, see
`docs/internals/private-registries.md`.
