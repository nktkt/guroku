# Credential Handling

This document describes how guroku treats registry credentials throughout
its lifetime: where they come from, how they are represented in memory,
who is allowed to read them, and where they ultimately leave the
process. It is a companion to `docs/internals/auth.md`, which focuses on
the request-construction side of authentication. This document is the
security-boundary view: what a credential touches and what it does not.

## 1. Where credentials enter guroku

guroku reads credentials from exactly two locations:

- `<cwd>/.npmrc` — the project-local config in the working directory
  guroku was invoked from. If a key is set both here and in the
  user-global file, the project-local value wins.
- `~/.npmrc` — the user-global config in the user's home directory.

That is the entire list. Specifically, guroku does **not** read
credentials from any of the following:

- environment variables (no `NPM_TOKEN`, no `npm_config_*`, no
  `GUROKU_TOKEN`),
- hard-coded fallbacks in the binary,
- a command-line `--token=` flag,
- any platform keychain or secret store,
- a `.guroku/` directory or guroku-specific config file.

If neither `.npmrc` exists, or if neither contains a `_authToken` for
the registry being contacted, guroku makes the request unauthenticated
and lets the registry return whatever it returns (typically a 401 for
private packages, or a successful response for public ones).

## 2. In-memory representation

After the two `.npmrc` files are parsed and merged (project-over-user),
the result is a single struct:

```rust
pub struct Npmrc {
    entries: BTreeMap<String, String>,
}
```

There is no separate `Credentials` struct. Tokens live in this map
alongside non-secret values like `registry=`, `cache=`, and `prefix=`.
The key for a token is the literal `.npmrc` key, e.g.
`//registry.npmjs.org/:_authToken`, and the value is the literal
token string.

This is a deliberate choice. We do not want to add a separate "secret"
type because:

- `.npmrc` itself does not distinguish secrets from non-secrets, so
  any classification we apply is heuristic.
- Wrapping tokens in a newtype gives a false sense of safety: the
  underlying `String` is still a regular allocation, still visible in
  a heap dump, and still printable via `Debug` if anyone derives it.

We get safety from the rules in the rest of this document, not from
the type system.

## 3. Where credentials are consumed

There is exactly one consumer:

```rust
// src/registry.rs
pub fn auth_for(&self, url: &Url) -> Option<&str>
```

Given a request URL, `auth_for` walks the `Npmrc` map looking for a
`_authToken` key whose host-and-path prefix matches the URL, and
returns a borrowed `&str` into the same `BTreeMap` value. The token is
not cloned, not copied, and not converted to `String` on the hot path.
The borrow lives just long enough for the caller (the request builder)
to attach an `Authorization` header.

No other module in guroku is permitted to call `auth_for`. Resolution,
the lockfile, the CAS, the linker, lifecycle scripts, and the CLI all
go through `RegistryClient` and never see the raw token.

## 4. Where credentials leave guroku

A token leaves the process in exactly one form: as the value of an
`Authorization: Bearer <token>` header on an outgoing reqwest call to
the registry that the token is scoped to.

A token never leaves through any of these channels:

- log output (see redaction rules in section 5),
- stdout or stderr written to the user's terminal,
- the lockfile (`guroku.lock` does not store auth material),
- the on-disk cache (`~/.guroku/cache/`),
- the CAS (`~/.guroku/store/`),
- a rewrite of `.npmrc`. guroku treats both `.npmrc` files as
  read-only. We never call `fs::write` on them, even to "tidy up"
  formatting or insert defaults. The user's editor is the only thing
  that mutates those files.

## 5. Logging and redaction

guroku uses `tracing` for all diagnostic output. The convention in
`src/registry.rs` is:

- Log the request URL.
- Log the response status code.
- Do **not** log request headers.
- Do **not** log request bodies.
- Do **not** log response bodies (only their length, when relevant).

Concretely: there is no `tracing::debug!("headers: {:?}", req.headers())`
anywhere in the registry module, and there is no point where the
`Npmrc` map is dumped at any log level. Running with `GUROKU_LOG=debug`
or even `GUROKU_LOG=trace` gives you URL-and-status-level detail but
does not leak tokens.

If you are adding logging to the registry module, the rule is: log
URLs and status codes, log timing, log retry attempts. Do not log
anything that came out of the `Npmrc` map.

## 6. The process boundary

guroku spawns subprocesses in two contexts:

- `git clone` for git-protocol dependencies, and
- `sh -c <script>` (or the platform equivalent) for lifecycle scripts
  declared in a package's `package.json` (`preinstall`, `install`,
  `postinstall`, etc.).

In both cases the child inherits the parent's full environment.
Critically, that environment **does not** by default contain registry
tokens, because guroku never exports `_authToken` (or anything
derived from it) into its own environment. Tokens live entirely in
the in-memory `Npmrc` map and are reachable only via `auth_for`,
which is only called by the request builder, not by the subprocess
launcher.

The corollary: a lifecycle script will see `process.env.NPM_TOKEN`
if and only if the user themselves put `NPM_TOKEN` in their shell
environment before invoking guroku. guroku does not synthesize one.
The same applies to `npm_config_*` variables: guroku does not
populate them from `.npmrc`.

## 7. What this means for malicious dependencies

A common attack pattern for the npm ecosystem is a postinstall
script that reads `process.env.NPM_TOKEN` and exfiltrates it. Under
guroku's default behavior, that variable is not set by guroku, so a
package that relies on it will find an empty value.

This is a useful but partial mitigation. A determined script can
still read `~/.npmrc` directly through normal filesystem APIs, since
it runs as the same user. Closing that hole requires sandboxing
lifecycle scripts (see section 11), which is future work.

In other words: guroku raises the bar from "trivial environment
read" to "filesystem read of a known path". Both are bad; one is
worse.

## 8. Threats not addressed

guroku's credential handling is intentionally narrow. The following
threats are out of scope and are not mitigated by the design above:

- **Filesystem read access to `~/.npmrc`.** Anything running as the
  user can read the file. The recommended mitigation is the
  conventional one: `chmod 600 ~/.npmrc`. guroku does not enforce
  this and does not warn about it.
- **Memory dumps.** While guroku is running, the token is a live
  `String` in the heap. A coredump, a `ptrace` attach, or a debugger
  will see it. We do not zero memory on drop and we do not use any
  locked-memory facility.
- **TLS interception.** guroku trusts the system root certificate
  store via `reqwest`'s default TLS backend. A user or admin who has
  installed a corporate MITM CA can see the token in transit. This
  is by design — corporate proxies are a real use case — but it is
  not a "secure against the network" property.
- **Compromised CI runner.** If the CI environment is compromised,
  the attacker has the same access as the CI job, including its
  `.npmrc`. guroku has no special CI-mode hardening.

These are real threats; they are simply outside what package-manager
credential handling can address on its own.

## 9. Lifecycle of a token within a guroku run

1. The user invokes `guroku install` (or any other command that
   talks to a registry).
2. `RegistryClient::from_npmrc(cwd)` reads `<cwd>/.npmrc` and
   `~/.npmrc`, parses both, merges them with project-wins, and
   stores the result in an `Arc<Npmrc>` held by the client.
3. For the rest of the run, every registry HTTP request is built by
   the same `RegistryClient`, which calls `self.npmrc.auth_for(url)`
   to attach a header if a matching token is found.
4. When the process exits, the `Arc<Npmrc>` is dropped, the
   `BTreeMap` is dropped, and the underlying `String` allocations
   are freed.

There is no token rotation within a single run. If the user edits
`~/.npmrc` while guroku is running, that edit is not picked up;
the in-memory snapshot taken at step 2 is authoritative for the
remainder of the process. To rotate a token, restart guroku (or, in
a long-lived embedding, build a fresh `RegistryClient`).

## 10. What guroku does not yet do

Several features that adjacent ecosystems have are explicitly absent:

- **OS keychain integration.** guroku does not read from macOS
  Keychain, Windows Credential Manager, or `libsecret`. Tokens come
  from `.npmrc` only.
- **Token-scope detection.** guroku does not know whether a token is
  read-only, read-write, or has publish permission. It just attaches
  the bearer header and lets the server decide.
- **Token-expiry detection.** A 401 response is surfaced as a
  generic registry error. guroku does not parse `WWW-Authenticate`
  challenges and does not specifically advise the user to refresh
  their token.
- **Per-registry credential prompts.** guroku will not interactively
  ask the user for a token. If the registry returns 401, the
  command fails, and the user is expected to update their `.npmrc`.
- **`.npmrc` writeback.** guroku will not add, remove, or rewrite
  entries in either `.npmrc` file.

These are not technical impossibilities. They are deferred decisions
that we want to make alongside a broader trust model rather than
piecewise.

## 11. Future work

The credential surface intersects with the larger picture in
`docs/internals/security-model.md`. The work items most relevant to
this document are:

- **Sandboxing lifecycle scripts.** Run `pre/post/install` scripts
  in an environment that cannot read `~/.npmrc`. This closes the
  filesystem hole left open in section 7.
- **Package signing.** Verify a publisher signature on the tarball
  itself, so that a stolen registry token cannot be used to publish
  a malicious version that guroku will then accept.
- **Per-package trust grants.** Let the user (or the lockfile)
  declare which packages are allowed to run scripts at all, and
  refuse to execute scripts from packages that have not been
  granted that capability.

Until those land, the credential boundary documented here is the
boundary, and the threat model in section 8 is the threat model.
