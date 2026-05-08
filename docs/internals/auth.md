# Authentication (HTTP)

This document describes how guroku v0.5 authenticates outbound HTTP
requests against npm-compatible registries. It is deliberately scoped
to the wire-level behaviour: where a token comes from, how it is
attached to a request, and which npm legacy mechanisms are intentionally
not implemented.

For the broader npmrc parsing story see `npmrc.md`. For the registry
caching layer see `http-cache.md`.

## 1. Source of credentials

guroku reads credentials from npmrc files only. Two files are consulted,
in this order, with the first taking precedence on key collisions:

1. `<cwd>/.npmrc` — the project-local file, discovered by walking up
   from the current working directory passed to `RegistryClient::from_npmrc`.
2. `~/.npmrc` — the user-global file in `$HOME`.

The two files are flattened into a single key/value map; later wins
mean the project file overlays the user file.

The keys that affect authentication are:

- `//<host>/:_authToken=<token>`

  A bearer token. The key is matched by host (and, optionally, path
  prefix — see below). Any outbound request whose URL host matches
  `<host>` will have `Authorization: Bearer <token>` attached.

  Examples:

  ```ini
  //registry.npmjs.org/:_authToken=npm_xxxxxxxx
  //npm.pkg.github.com/:_authToken=ghp_xxxxxxxx
  ```

- `<scope>:registry=<url>`

  Routes scoped package fetches to a non-default registry. This is
  not itself an auth directive, but it is load-bearing for auth: the
  scoped URL gets its own `_authToken` lookup keyed off its host.

  Example:

  ```ini
  @acme:registry=https://npm.pkg.github.com/
  //npm.pkg.github.com/:_authToken=ghp_xxxxxxxx
  ```

  A `@acme/foo` install routes to `npm.pkg.github.com`, and the auth
  lookup then matches the GitHub token, not the npmjs.org one.

guroku does not read the global `_authToken` (no host prefix) — every
token must be host-scoped. This matches modern npm behaviour and avoids
accidentally leaking a registry token across registries.

## 2. How RegistryClient stores npmrc

`RegistryClient::from_npmrc(cwd: &Path)` is the single constructor that
wires up auth:

```rust
let npmrc = Npmrc::discover(cwd)?;
let client = RegistryClient {
    http: reqwest::Client::new(),
    cache: HttpCache::default(),
    npmrc,
    ..
};
```

There is no separate `Credentials` struct, no `AuthStore`, no token
cache. The parsed `Npmrc` is held by value on the client and every
auth decision goes through one method:

```rust
npmrc.auth_token(host: &str) -> Option<&str>
```

`auth_token` does the host-prefix matching against the npmrc map. The
client itself never inspects npmrc keys directly — it only ever asks
`auth_token`. This keeps the matching rules in one place and makes the
client trivially composable with a stub `Npmrc` in tests.

## 3. Where the header is added

There are exactly four call sites that may attach an `Authorization`
header. All four follow the same pattern: resolve the URL, ask
`auth_for(&url)` for an optional token, and if present, call
`req.bearer_auth(token)`.

| Call site                       | Purpose                                       |
| ------------------------------- | --------------------------------------------- |
| `fetch_metadata`                | Cached packument GET (`registry/<name>`).     |
| `fetch_metadata_uncached`       | Forced-fresh packument GET (cache bypass).    |
| `fetch_tarball`                 | Tarball GET (`registry/<name>/-/<tgz>`).      |
| `http_post_json` (audit)        | `POST /-/npm/v1/security/audits` and similar. |

The helper looks roughly like:

```rust
fn auth_for(&self, url: &Url) -> Option<&str> {
    let host = url.host_str()?;
    self.npmrc.auth_token(host)
}

fn apply_auth(&self, req: RequestBuilder, url: &Url) -> RequestBuilder {
    match self.auth_for(url) {
        Some(token) => req.bearer_auth(token),
        None        => req,
    }
}
```

Any other HTTP egress (e.g. fetching the npm advisory feed, OCI
clients) is either unauthenticated or routes through the same four
sites. There is no "default headers" middleware on the reqwest
client — this is intentional, so an unrelated future request cannot
accidentally inherit a bearer token meant for the registry host.

## 4. The header we send

The only auth header guroku v0.5 emits is:

```
Authorization: Bearer <token>
```

That is the entire auth surface on the wire.

guroku does not emit:

- `Authorization: Basic <base64>` — no Basic auth path, even if the
  user has a `_password` and `username` pair in their npmrc.
- An always-on token derived from `always-auth=true`. The flag is
  recognised by the parser but does not change request behaviour;
  bearer tokens are sent whenever the host matches, regardless.
- Anything derived from `npm_*` environment variables (see below).

## 5. Scope routing

For scoped packages, the registry URL is selected by `registry_for`:

```rust
fn registry_for(&self, name: &PackageName) -> Url {
    if let Some(scope) = name.scope() {
        if let Some(url) = self.npmrc.scope_registry(scope) {
            return url.clone();
        }
    }
    self.npmrc.default_registry().clone()
}
```

Auth then runs against whatever URL `registry_for` returned. So for
`@acme/foo`:

1. `registry_for("@acme/foo")` returns the value of
   `@acme:registry=`, e.g. `https://npm.pkg.github.com/`.
2. `auth_for(&that_url)` looks up `npm.pkg.github.com` in the npmrc
   map and finds the GitHub token.
3. The request goes out with the GitHub bearer, not the default
   registry's bearer.

Unscoped packages skip step 1 and use the default base directly.

## 6. What we don't do (yet)

The following npm-compatible behaviours are intentionally absent in
v0.5. Most are listed here so that grepping the source for them
returns this document.

- **`npm_config_*` environment variables.** npm CLI promotes
  `NPM_CONFIG_REGISTRY=...` and similar into runtime config. guroku
  reads npmrc files only. If you want to override the registry in CI,
  write a `.npmrc` next to the lockfile.
- **`legacy-auth-token=<base64-of-user:pass>`.** This is an npm v6
  holdover for registries that only spoke Basic auth. We don't
  decode it and we don't send Basic.
- **`cafile=<path>` / custom WebPKI roots.** We trust the system root
  store via `rustls-native-certs`. Pinning a custom CA bundle is not
  yet wired in; it needs a separate `RegistryClient` constructor that
  can take a custom TLS config.
- **Auth for `git+https://` clones.** When a dependency resolves to a
  git URL, guroku shells out to `git` as a subprocess. `git` then
  uses whatever the user has configured —
  `~/.git-credentials`, the system credential helper, an SSH key, or
  a deploy key. guroku does not pipe a registry `_authToken` into
  git, and it does not rewrite `git+https://` URLs to inject
  credentials.

If any of the above blocks a real workflow, file an issue with the
specific registry and use case — none of these are philosophically
out-of-scope, they just aren't wired up.

## 7. Diagnostics

`GUROKU_LOG=debug` increases verbosity on the registry client. It
does not log the token. Specifically:

- The request logger strips `Authorization` from any header set it
  prints. The strip is unconditional — it does not check whether the
  value is a bearer or basic header.
- The response logger does not echo request headers.
- npmrc parsing logs key names, never values, for any key matching
  `*token*`, `*password*`, or `auth*`.

This still leaks the URL and the host, which is enough to identify
the registry. If that bothers you, redact at the shell level.

Operationally: use a personal access token, not a long-lived account
password. PATs can be revoked per-machine, scoped to a single
registry, and rotated without touching anything else.

## 8. Test surface

`RegistryClient::without_http_cache(npmrc, base)` exists to build a
client that bypasses the on-disk HTTP cache. It is the canonical hook
for integration tests that want deterministic network behaviour.

There is no equivalent `without_auth` constructor yet. If a test
needs to assert that a request goes out unauthenticated, today the
options are:

1. Construct a `RegistryClient` with an empty `Npmrc` (no token keys
   set). `auth_for` will then return `None` for every host.
2. Use a mock HTTP layer (wiremock) and assert the absence of an
   `Authorization` header on incoming requests.

A `RegistryClient::without_auth` helper that ignores npmrc tokens
entirely would be cheap to add and is worth considering if the test
suite grows a third or fourth case that needs it. It is not
implemented in v0.5.
