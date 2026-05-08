# Authenticating to private registries

This page covers how to point `guroku` at a private npm-compatible registry
(npm.com paid orgs, Verdaccio, JFrog Artifactory, Sonatype Nexus, GitHub
Packages, etc.) and how it sends credentials. If you are looking for the
internal design of the auth subsystem, see `docs/internals/auth.md` instead.

## What v0.5 supports

guroku v0.5 supports a single auth scheme: **bearer-token auth via
`_authToken` entries in `.npmrc`**. That covers the vast majority of npm
deployments in the wild.

What is **not** supported in v0.5:

- HTTP Basic auth (`_auth=...`, `username=...` + `_password=...`).
- `npm_config_*` environment variables (e.g. `NPM_CONFIG_REGISTRY`,
  `NPM_CONFIG__AUTH_TOKEN`). These are recognised by the npm CLI but
  guroku currently ignores them.
- `${VAR}` interpolation inside `.npmrc` values. The token must be
  written literally.

If you need any of the above, see the "What v0.5 does NOT yet do" section
below for workarounds.

## Setting it up

The auth config lives in `.npmrc`, the same file npm itself uses. A
typical setup looks like:

```
registry=https://registry.npmjs.org
@acme:registry=https://npm.acme.internal
//npm.acme.internal/:_authToken=hunter2
```

Two locations are searched, in this order:

1. `<project>/.npmrc` — the project root, next to `package.json`.
2. `~/.npmrc` — your home directory, machine-wide for the current user.

If the same key appears in both, **the project file wins**. This lets you
override a personal token for a specific repo without changing your
machine-wide config.

A few rules of the file format:

- `registry=...` sets the default registry for unscoped packages.
- `@scope:registry=...` sets the registry for a single scope. Scoped
  packages always go through the scope's registry, never the default.
- `//host/:_authToken=...` attaches a token to one host. The leading `//`
  and the trailing `/` are required and match the way npm itself parses
  the file.
- Lines starting with `;` or `#` are comments.
- Whitespace around `=` is preserved. Don't add spaces.

You can have multiple `_authToken` entries — one per host you talk to:

```
//npm.acme.internal/:_authToken=hunter2
//npm.pkg.github.com/:_authToken=ghp_xxxxxxxxxxxxxxxx
//registry.npmjs.org/:_authToken=npm_xxxxxxxxxxxxxxxx
```

## What gets sent

For every outbound HTTP request whose host matches an `_authToken`
entry, guroku adds:

```
Authorization: Bearer <token>
```

This includes:

- Package metadata fetches (`GET /<pkg>` and `GET /<pkg>/<version>`).
- Tarball downloads (`GET /<pkg>/-/<pkg>-<version>.tgz`), even when the
  tarball URL points at a CDN under the same host.
- Audit POSTs (`POST /-/npm/v1/security/audits/quick`).
- Any redirect target whose host still matches.

Hosts that do **not** match get no `Authorization` header at all —
guroku will not silently leak your token across hosts. The match is
exact on hostname (case-insensitive); `npm.acme.internal` and
`other.acme.internal` are different hosts.

If a request returns `401 Unauthorized`, guroku surfaces this as
`E_AUTH` with the host and registry URL in the error body. It does not
retry without auth.

## What to put in `<token>`

The token format depends on which registry you are talking to. guroku
does not care about the contents — it just forwards the string — but
your registry does.

### npmjs.com

Generate a token at:

```
https://www.npmjs.com/settings/<user>/tokens
```

You have two choices:

- **Granular Access Token** (recommended). Lets you scope the token to
  specific packages or orgs and set an expiry. Pick "Read-only" if you
  only need `guroku install`. For `guroku publish` you also need
  "Read and write". For `guroku audit` against private advisories you
  need the "Read advisories" permission.
- **Classic Token**. The older format. "Read-only", "Automation", and
  "Publish" all work; "Automation" is the typical CI choice because it
  bypasses 2FA for publish.

Tokens look like `npm_<40+ chars>`. Paste the whole string after
`_authToken=`.

### Verdaccio

Verdaccio's auth API depends on its plugins. The most common path is:

```
npm login --registry https://verdaccio.example.com
```

run **once** on a workstation, then copy the resulting `_authToken`
line out of `~/.npmrc` into your guroku config. Verdaccio's web UI
also has a "tokens" tab if a recent version is deployed.

### JFrog Artifactory

In the Artifactory UI, click your username → "Edit Profile" → "Generate
an Identity Token". Use the identity token as the `_authToken`. Do
**not** use your Artifactory API key directly — Artifactory's npm
endpoint expects bearer tokens, not API keys.

### Sonatype Nexus

In Nexus 3, go to your user profile → "NPM" tab and click "Set NPM
Repository Password". Nexus then prints a base64 blob; the part you
want is everything after `//host/:_authToken=` in the snippet it
shows.

### GitHub Packages

Generate a Personal Access Token (classic) at
`https://github.com/settings/tokens` with the `read:packages` scope.
For publishing, also tick `write:packages`. The host is
`npm.pkg.github.com`:

```
@your-org:registry=https://npm.pkg.github.com
//npm.pkg.github.com/:_authToken=ghp_xxxxxxxxxxxxxxxx
```

Fine-grained PATs do not currently work with GitHub Packages; you must
use a classic PAT.

## Don't commit your `.npmrc`

`.npmrc` files commonly contain tokens. Add the path to `.gitignore`:

```
# .gitignore
.npmrc
```

This is **especially** important for `<project>/.npmrc`, which sits
inside your repo and is easy to commit by accident. If you have already
committed a token, rotate it — `git rm` alone doesn't help, the token is
in history.

If you want a project-level `.npmrc` checked in but **without** the
token (e.g. just the `registry=` and `@scope:registry=` lines), split
it: keep the public bits in a committed `.npmrc` and the `_authToken`
line in `~/.npmrc`. guroku merges them automatically.

## CI

Most CI systems expose secrets as environment variables. Since v0.5
doesn't read `npm_config_*` env vars, the standard pattern is to write
a one-line `~/.npmrc` from the secret before invoking guroku.

GitHub Actions:

```yaml
- name: Write npmrc
  run: |
    echo "//npm.acme.internal/:_authToken=${{ secrets.NPM_TOKEN }}" >> ~/.npmrc
- name: Install
  run: guroku install
```

GitLab CI:

```yaml
install:
  script:
    - echo "//npm.acme.internal/:_authToken=${NPM_TOKEN}" >> ~/.npmrc
    - guroku install
```

CircleCI:

```yaml
- run:
    name: Write npmrc
    command: echo "//npm.acme.internal/:_authToken=${NPM_TOKEN}" >> ~/.npmrc
- run: guroku install
```

A few CI hygiene notes:

- Append to `~/.npmrc` (`>>`), don't overwrite (`>`). Some CI base
  images ship a default `~/.npmrc`.
- Mask the secret in your CI's secret store. guroku does not echo the
  token, but a typo in your shell script can.
- Don't `cat ~/.npmrc` for debugging. Use `GUROKU_LOG=debug` instead
  (see below).

## Testing your config

To verify auth is being applied to the right requests:

```sh
GUROKU_LOG=debug guroku install
```

In the debug output, look for lines of the form:

```
http GET https://npm.acme.internal/@acme/widget auth=bearer
http GET https://registry.npmjs.org/lodash auth=none
```

`auth=bearer` means a token was attached for that host. `auth=none`
means no `_authToken` matched. The token itself is **redacted** in
logs — guroku replaces it with `<redacted>` before formatting the line.

If you are seeing `auth=none` on a host where you expected a token,
the most common causes are:

- Missing `//` prefix or trailing `/` on the `_authToken` key.
- Hostname mismatch (e.g. `_authToken` for `npm.acme.internal` but
  the registry URL is `https://npm.acme.internal:443/` — the explicit
  port confuses some legacy parsers; remove it).
- Project `.npmrc` overriding the home one with an empty value.

## What v0.5 does NOT yet do

The following are tracked but not in 0.5:

- **`npm_config_*` env vars.** Use `.npmrc` for now. CI users: write a
  one-liner `.npmrc` from your secret as shown above.
- **Basic auth** (`_auth=...` or `username=` + `_password=...`). If
  your registry only supports basic auth, you are stuck on npm or yarn
  for now. Most registries that started life with basic auth (older
  Verdaccio, older Artifactory) also accept bearer tokens — check
  their docs.
- **`${VAR}` interpolation** inside `.npmrc` values. npm and yarn both
  expand `${NPM_TOKEN}` in `.npmrc` at parse time; guroku does not.
  Write the literal token, or generate the file in CI.
- **Auth for `git+https://` clones.** When `package.json` references a
  git URL, guroku shells out to `git`. Authentication for that is
  handled by `git` itself: `~/.git-credentials`, the OS credential
  helper, or SSH keys via `git+ssh://`. guroku does not pass its
  `_authToken` to `git`.

## Token rotation

guroku has no special handling for token rotation. The flow is:

1. Generate a new token in your registry's UI.
2. Edit `.npmrc` and replace the old `_authToken` value.
3. Re-run `guroku install` (or whatever command you were running).
4. Revoke the old token in your registry's UI.

There is no in-memory cache of credentials across runs, so step 3 picks
up the new value immediately. If you are running a long-lived process
(`guroku watch`, etc.), restart it.

## FAQ

**Can I use the same token for two scopes?**

Yes, as long as both scopes resolve to the same host. `_authToken` is
keyed by host, not by scope. If `@acme` and `@acme-internal` both
point to `npm.acme.internal`, one `_authToken` entry covers both.

**Why is my token being sent to npmjs.org?**

It shouldn't be. guroku only attaches a token if the request host
matches the `//host/` prefix on an `_authToken` entry. If you are
seeing this, double-check the prefix — a typo like
`//registry.npmjs.org/:_authToken=` followed by your **internal**
token is a common mistake when copying configs between machines.
Run `GUROKU_LOG=debug guroku install` and check which host each
request is going to.

**Can I disable auth for a single command?**

Not directly in v0.5. Two workarounds:

- Comment out the `_authToken` line temporarily (`;` at the start).
- `cd` into a directory with no `.npmrc` and run with `--cwd` pointing
  at the project, so only the home `.npmrc` applies — or vice versa.

A `--no-auth` flag is on the roadmap.

**Does guroku support `.netrc`?**

No. Only `.npmrc`. `.netrc` was historically used by `npm` for some
registries; modern npm has moved entirely to `_authToken` and so has
guroku.

**Where is the token stored on disk?**

Wherever you wrote it — `.npmrc` is just a plain text file. guroku
does not copy it elsewhere, does not write it to its cache, and does
not include it in lockfiles. If you want OS-keychain-backed storage,
that is a planned feature but not in v0.5.
