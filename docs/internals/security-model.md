# Security Model

This document describes guroku's threat model and security posture as of
v0.4. It is meant for contributors and security-conscious users who want to
understand exactly what guroku protects against, what it does not, and what
is on the roadmap.

guroku is an npm-style package manager written in Rust. Its security
properties are largely inherited from the npm ecosystem it interoperates
with, augmented by content-addressable storage and lockfile pinning. This
doc enumerates trust boundaries explicitly so users can reason about risk.

## 1. What guroku trusts

guroku assumes the following inputs are trustworthy and operates on them
without further validation:

### 1.1 Bytes returned by the configured registry

When guroku fetches a tarball from the registry, it verifies the bytes
against the SHA-512 hash declared in the package's `dist.integrity` field
(an SRI string of the form `sha512-<base64>`). If the hash matches, guroku
treats the tarball as authentic. This means guroku trusts:

- The registry to serve the manifest honestly (the manifest is what tells
  us the expected hash).
- The cryptographic strength of SHA-512 to prevent collisions.

If a registry serves a malicious manifest with a hash matching a malicious
tarball, guroku has no way to detect that. This is the same trust model as
npm and pnpm. Mitigations like Sigstore-signed manifests are on the
roadmap (see Section 9).

### 1.2 Contents of `~/.guroku/cas`

The content-addressable store (`~/.guroku/cas/<algo>/<aa>/<bb>/<rest>`) is
trusted because:

- guroku wrote those bytes itself.
- Each blob's path is its hash; the hash was verified at insert time
  against `dist.integrity`.

guroku does not re-verify hashes on every read. If an attacker with write
access to the user's home directory tampers with a CAS blob, guroku will
hardlink the tampered bytes into `node_modules` without noticing. This is
considered out of scope: an attacker with write access to `~/.guroku` can
just modify `~/.profile` and win more easily.

### 1.3 The user's lockfile

`guroku.lock` is treated as authoritative. If it pins
`left-pad@1.3.0` to a specific integrity hash, guroku will install
exactly that, refusing alternatives. Users are expected to review
lockfile changes in pull requests, the same as `package-lock.json` or
`pnpm-lock.yaml`.

A compromised lockfile is a compromised project. guroku does not try
to detect "suspicious" lockfile entries.

## 2. What guroku does NOT trust

### 2.1 Tarball entry filenames

When extracting a `.tgz`, every entry path is checked. We reject any path
that:

- Contains `..` components (path traversal).
- Is absolute.
- Contains a NUL byte or other control characters.
- Resolves outside the package's intended extraction root.

A malicious tarball cannot, for example, write to `/etc/passwd` or
`../../../.ssh/authorized_keys` via this vector. See `extract.rs` and
the dedicated tests in `tests/tarball_safety.rs`.

### 2.2 Registry responses without integrity

If a manifest does not include `dist.integrity` and does not include a
`shasum`, guroku errors out rather than trusting the tarball. Older
registry entries that only have `shasum` (SHA-1) are accepted with a
warning, since SHA-1 is collision-broken but not preimage-broken; we plan
to drop this fallback in v0.6.

We do not silently accept "no integrity" as "OK". This is a behavioural
difference from very old npm clients.

## 3. The big elephant: lifecycle scripts

The single largest attack vector in the npm ecosystem is **lifecycle
scripts**: `preinstall`, `install`, `postinstall`, and `prepare`. These
fields in `package.json` execute arbitrary shell commands with the full
privileges of the invoking user, at install time.

A malicious package can:

- Read environment variables (`AWS_SECRET_ACCESS_KEY`, `GITHUB_TOKEN`,
  `~/.aws/credentials`).
- Read or modify any file the user can.
- Make network connections to exfiltrate data.
- Persist (e.g. write to `~/.bashrc`, install a launchd agent, etc.).

guroku v0.4 ships lifecycle scripts **on by default** to match the
default behavior of npm, pnpm, and bun. Doing otherwise would break the
overwhelming majority of real-world packages (`node-gyp`-based native
modules, `husky`, `puppeteer`, etc.).

This is a deliberate trade-off. We document it prominently here, in the
README, and via a one-line warning printed during `guroku install` the
first time it executes a script in a fresh project.

## 4. Mitigations available today

### 4.1 `--ignore-scripts`

Users can disable all lifecycle scripts for an install:

```
guroku install --ignore-scripts
```

This is recommended for CI environments and for installing untrusted
dependencies for inspection. The flag matches the equivalent
`npm install --ignore-scripts`.

It can also be set persistently in `.npmrc` via `ignore-scripts=true`,
which guroku honors.

### 4.2 Per-package script failure is warn-only

If a single package's `postinstall` exits non-zero, guroku logs a warning
and continues. The install does not abort. The rationale:

- Limits blast radius of a flaky-but-not-malicious script (e.g. a native
  build that fails on an unsupported platform).
- The package is still extracted; the user can inspect it.
- A genuinely critical script is rare; most are optional optimizations
  or development helpers.

This differs from npm, which historically aborts. We believe warn-only
is better for partial-install recoverability and matches what users
actually want.

Note that warn-only does **not** mean the script's side effects are
undone. If a malicious `postinstall` already exfiltrated your env vars,
guroku has no way to retract that.

## 5. What's NOT mitigated

guroku v0.4 does not currently protect against:

### 5.1 Malicious `postinstall` exfiltration

A `postinstall` script can read `process.env`, `~/.ssh/`, `~/.aws/`, your
git config, browser cookies, etc., and POST them to an attacker server.
There is no sandboxing in v0.4. The only defense today is
`--ignore-scripts`.

### 5.2 Sandboxing scripts

guroku does not currently run lifecycle scripts under a sandbox. This is
planned for v0.5+. The intended approach:

- **Linux**: integrate with `firejail` or `bubblewrap` to drop network,
  restrict filesystem access to the package's own directory, and strip
  most env vars.
- **macOS**: use `sandbox-exec` with a curated profile (it is deprecated
  but still functional and there is no good replacement yet).
- **Windows**: AppContainer or Job Objects; design TBD.

The challenge is that legitimate scripts (notably `node-gyp`) need
network access to download prebuilds and broad fs access to invoke
compilers. A workable sandbox profile is non-trivial.

### 5.3 `guroku trust <pkg>`

A planned per-package opt-in model:

- By default, scripts are sandboxed (see 5.2).
- `guroku trust <pkg>` marks a specific package's scripts as trusted
  (e.g. `node-gyp`, `husky`).
- The trust list is stored alongside the lockfile and reviewed in PRs.

This is a v0.6+ feature. Until then, trust is all-or-nothing per
invocation.

## 6. Network attacks

### 6.1 TLS

guroku makes HTTPS connections to `https://registry.npmjs.org` by
default. We use the system root certificate store (via `rustls` with
`rustls-native-certs`). We do **not** pin the registry's certificate.
Consequences:

- A user who has a malicious CA installed in their system store can be
  MITM'd.
- A compromised public CA could MITM in principle (this is what
  Certificate Transparency partially addresses).

Pinning is not currently planned because it imposes operational burden
on registry operators (rotation requires a guroku release).

### 6.2 ETag / 304 cache poisoning

guroku caches manifests using `If-None-Match` / ETag. A malicious origin
that returns `304 Not Modified` when content has actually changed can
pin a user to old (potentially vulnerable) content. The mitigation:

- The lockfile pins exact versions and integrity hashes, so a 304-pinned
  stale manifest cannot cause guroku to install different bytes than
  expected for a locked version.
- For unlocked installs (first install, `guroku add foo`), the user
  could be pinned to an older `latest`. This is a low-impact attack;
  users running `guroku update` periodically will see the update.

We do not attempt to detect this; it is documented in
`docs/internals/http-cache.md`.

### 6.3 `_authToken`

guroku v0.4 parses `_authToken` from `.npmrc` but does not yet send it
on requests. This means:

- Private/authenticated registries do not work in v0.4 (intentional).
- No accidental token leakage in v0.4.

In v0.5, the token will be sent as `Authorization: Bearer <token>`.
Note that the token then travels in plaintext **inside** the TLS
connection. If you operate a custom registry (corporate Artifactory,
Verdaccio, etc.), the registry sees the token in cleartext after TLS
termination. Treat tokens as registry-readable, and rotate them like
any other shared secret.

## 7. Local attacks

### 7.1 Filesystem permissions on `~/.guroku/`

The CAS and metadata are stored under `~/.guroku/`, which inherits the
user's umask (typically `0755` directories, `0644` files). On a
multi-user machine:

- Other unprivileged users cannot read another user's CAS unless they
  are in a shared group with permissive umask, which is unusual.
- The root user can read any user's CAS. This is true of essentially
  any file under `$HOME` and is not a guroku-specific issue.

We do not currently chmod the CAS to `0700`. We may consider this in a
future version.

### 7.2 Hardlink hazards

The CAS uses hardlinks to populate `node_modules`. This means:

```
node_modules/foo/index.js   # hardlink
~/.guroku/cas/sha512/aa/bb/...  # same inode
```

If a user (or a tool) edits `node_modules/foo/index.js` in place, **the
CAS blob is mutated**. The next install of any package that hardlinks
the same blob will see the modified content.

This is documented loudly in `docs/storage.md` and surfaces in two
places:

- `guroku install` prints a one-time hint: "guroku uses hardlinks; do
  not edit files inside node_modules in place."
- `guroku doctor` can detect tampered blobs by re-hashing CAS contents
  on demand.

Tools that legitimately need to modify a vendored file should copy it
out first. See `docs/internals/hardlinks.md` for the full discussion.

## 8. Reporting vulnerabilities

We use **private GitHub Security Advisories**. Please do not file public
issues for security-sensitive reports.

See `SECURITY.md` at the repository root for the current contact and
response-time expectations. A summary:

- Use the "Report a vulnerability" button on the GitHub repo's
  Security tab.
- We aim to acknowledge within 72 hours and ship a fix within 30 days
  for high-severity issues.
- Coordinated disclosure timelines are negotiable for genuinely tricky
  issues; please ask.

We do not currently have a paid bounty program.

## 9. Roadmap

The following are planned, in rough order:

### v0.5

- **Registry auth via `_authToken`**. Send `Authorization: Bearer` on
  registry requests. Required for private registries.
- **Signed packages (read-only)**. Verify Sigstore signatures on npm
  packages where present (npm has begun publishing these). guroku will
  log signature status and refuse to install on signature mismatch
  when a signature is expected.

### Future (v0.6+)

- **Sandboxed lifecycle scripts** (Section 5.2). Drop network, restrict
  filesystem to the package directory, strip env. Per-platform
  implementations.
- **`guroku trust` model** (Section 5.3). Per-package opt-in to
  unsandboxed scripts.
- **CAS permission hardening**. `chmod 0700 ~/.guroku/cas` by default.
- **Optional pinned-cert mode** for `registry.npmjs.org` for users who
  want it.
- **Audit subcommand** (`guroku audit`) querying the npm advisory
  database against the resolved dep graph.

## Summary

guroku v0.4 protects against tarball path traversal, registry
content tampering (via SHA-512), and lockfile-divergent installs. It
does **not** protect against malicious lifecycle scripts beyond
offering `--ignore-scripts`. This is the same security posture as npm
and pnpm, and the same caveats apply: treat `npm install` (and
`guroku install`) as running untrusted code, especially for transitive
dependencies you have not personally audited.

If you are installing untrusted packages for inspection, always use
`--ignore-scripts`, and ideally do so inside a container or VM.
