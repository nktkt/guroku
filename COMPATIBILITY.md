# Compatibility Matrix

This document is the canonical reference for what guroku v1.0 supports and what
it does not. It covers operating systems, Rust toolchain pins, registries,
lockfiles, lifecycle scripts, filesystems, and known incompatibilities. It also
documents grace periods and forward-compatibility commitments so downstream
users can plan upgrades with confidence.

For deeper, per-topic detail, follow the cross-references to documents under
`docs/`.

---

## 1. Platform support

guroku v1.0 publishes pre-built binaries for the following targets. Tier 1
(`yes` / `yes`) targets are exercised on every CI run and gate releases. Tier 2
("best-effort") targets compile on every release but may have looser test
coverage. Tier 3 ("no") targets are produced by cross-compilation and are
smoke-tested manually before release.

| OS | Architecture | Pre-built binary | CI tested |
|---|---|---|---|
| Linux | x86_64 | yes | yes |
| Linux | aarch64 | yes (cross) | yes |
| macOS | x86_64 (Intel) | yes | yes |
| macOS | aarch64 (Apple Silicon) | yes | yes |
| Windows | x86_64 | yes (v1.0+) | best-effort |
| Windows | aarch64 | yes (v1.0+) | no |

Notes:

- Linux aarch64 binaries are produced via cross-compilation from an x86_64
  builder. They are exercised under QEMU during CI and on a self-hosted
  Graviton runner for release candidates.
- Windows support graduated to "shipped" in v1.0. Pre-1.0 versions emitted a
  preview build only.
- FreeBSD, OpenBSD, illumos, Linux 32-bit, and 32-bit ARM are not in scope for
  v1.0. Users on these platforms can build from source; we accept patches but
  do not gate releases on them.

---

## 2. Rust toolchain support

- **MSRV (Minimum Supported Rust Version): 1.75**
- See `docs/MSRV.md` for the policy on bumping the MSRV and the cadence we
  follow.

The MSRV is enforced via `rust-version = "1.75"` in `Cargo.toml` and a CI job
that pins `rustup default 1.75.0`. Pre-1.0 versions also tracked
`rust-version = 1.75`; there is no pre-1.0 -> 1.0 toolchain bump to plan for.

We will not bump the MSRV within a v1.x minor release without one full minor
of advance notice in `CHANGELOG.md` and on the project Discussions board. A
patch release (1.x.y) will never increase the MSRV.

If you build guroku from source with a newer toolchain, that is fine and
supported; the MSRV is a *floor*, not a ceiling.

---

## 3. Node.js compatibility

guroku does **not** bundle Node.js and does **not** require any particular
version of Node.js to install packages. The resolver, fetcher, extractor, and
linker are pure Rust and have no Node dependency.

Node is only invoked when:

- A package declares `scripts.preinstall`, `scripts.install`, `scripts.postinstall`,
  or `scripts.prepare` and the script body shells out to `node` (directly or
  transitively via `npm`, `npx`, etc.).
- The user runs `guroku exec <bin>` and the resolved bin is a Node script
  (which is the common case for npm-style packages).
- The user runs `guroku run <script>` and the script body invokes `node`.

In all of these cases guroku looks up `node` on `PATH` at the moment of
execution. If `node` is not on `PATH`, lifecycle scripts and `guroku exec`
will fail with a clear error pointing at the missing binary; the install
itself (download, extract, link) still succeeds.

We do not pin a Node version range. If your project needs a specific Node,
manage it with `nvm`, `fnm`, `volta`, `mise`, `asdf`, or your distribution's
package manager.

---

## 4. Registry compatibility

guroku speaks the npm registry HTTP protocol. The following registries have
been tested at least once against the v1.0 release candidate.

| Registry | Metadata | Tarballs | Auth tokens | Audit (`/-/npm/v1/security/advisories/bulk`) |
|---|---|---|---|---|
| npm.com (registry.npmjs.org) | yes | yes | yes | yes |
| Verdaccio | yes | yes | yes | partial (must be configured) |
| JFrog Artifactory | yes | yes | yes | typically not proxied |
| Sonatype Nexus | yes | yes | yes | typically not proxied |
| GitHub Packages | yes | yes (with extra Accept header — TODO) | yes | not proxied |
| Cloudsmith | yes | yes | yes | depends |

Notes per registry:

- **registry.npmjs.org**: the reference implementation. Everything works,
  including the bulk advisories endpoint used by `guroku audit`.
- **Verdaccio**: works out of the box for metadata, tarballs, and tokens. The
  bulk advisories endpoint is only available if the operator has installed
  and configured a compatible plugin; otherwise `guroku audit` will fall back
  to per-package queries.
- **JFrog Artifactory**: full proxy of metadata and tarballs. The audit bulk
  endpoint is generally not proxied; expect `guroku audit` to either degrade
  or to require pointing at registry.npmjs.org explicitly via
  `audit-registry`.
- **Sonatype Nexus**: same story as Artifactory.
- **GitHub Packages**: requires a non-default `Accept` header for tarball
  downloads. v1.0 does not yet send this header automatically; tracked as a
  TODO in the registry client. Workaround: set
  `npm.pkg.github.com/:_authToken` and use `--registry` per-scope.
- **Cloudsmith**: works for metadata, tarballs, and tokens. Audit support
  depends on the customer's plan and configuration.

If your registry is not on this list and speaks the npm protocol, it will
probably work. Please file an issue with the `compat` label so we can add it
to the matrix.

---

## 5. Lockfile compatibility

- `guroku.lock` declares `lockfileVersion: 1`. This is the only schema
  supported in v1.0 and is stable for the entire v1.x series.
- The lockfile is **forward-compatible within v1.x**: older guroku will
  tolerate (and round-trip) unknown fields added by newer guroku. See
  `docs/STABILITY.md`.
- The lockfile is line-stable: regenerating with the same inputs produces a
  byte-identical file. Diffs in code review correspond to real dependency
  changes.

guroku does **not** consume `package-lock.json`, `pnpm-lock.yaml`, or
`yarn.lock`. There is no implicit migration.

An importer (`guroku import`) that converts the above formats into
`guroku.lock` is on the v1.x roadmap. There is no committed ETA. Until then,
projects migrating from npm/pnpm/yarn should run `guroku install` once on a
clean checkout to materialize a fresh `guroku.lock`.

---

## 6. Lifecycle script compatibility

guroku runs the following lifecycle scripts:

- **Root project**: `preinstall`, `install`, `postinstall`, `prepare`.
- **Per dependency**: `preinstall`, `install`, `postinstall`.

Notes and limitations:

- `prepublish`, `prepublishOnly`, `prepack`, `postpack`, `dependencies`,
  `version`, and the various `pre*` / `post*` hooks for `test`, `start`,
  `restart`, `stop` are not part of the install pipeline. Some are still
  invoked when the user explicitly runs `guroku run <name>`; install-time
  hooks beyond the four root + three per-dep scripts above are not invoked
  automatically.
- guroku does **not yet set `npm_*` environment variables** (such as
  `npm_package_name`, `npm_package_version`, `npm_lifecycle_event`,
  `npm_config_*`). Scripts that introspect `process.env.npm_*` may
  misbehave. A compatibility shim that injects a documented subset is
  planned; track it in `docs/ROADMAP.md`.
- Scripts run with `PATH` extended to include `node_modules/.bin` of the
  current package and all ancestors, matching npm's behaviour.
- `--ignore-scripts` disables every lifecycle script for the install. CI
  pipelines that pull untrusted dependencies should pass it.

---

## 7. Filesystem compatibility

The linker creates `node_modules` using hardlinks where possible and
symlinks where required (for `.bin/` shims and for nested layouts). The
following table summarizes filesystem support:

- **ext4, btrfs, xfs (Linux)**: full support. Hardlinks and symlinks both
  work. This is the fast path.
- **APFS (macOS)**: full support.
- **NTFS (Windows)**: full support, subject to the symlink caveat below.
- **exFAT**: hardlinks are not supported by the filesystem; guroku
  transparently falls back to copying. Installs will be slower and use more
  disk space. There is no warning by default.
- **Network filesystems (NFS, SMB / CIFS)**: best-effort. Hardlinks across
  the network are usually supported but may be slow. fsync semantics on some
  NFS servers are weaker than guroku assumes; if you observe corrupted
  installs, file an issue with the server type and version.
- **Windows symlinks**: creating symlinks on Windows requires either
  Developer Mode (Settings -> For developers -> Developer Mode) or running
  guroku as Administrator. Without one of these, `.bin/` shim creation will
  fall back to copying the target executable, which works but defeats
  dedup.

The cache (`~/.cache/guroku` on Linux, equivalents on macOS/Windows) is
expected to live on the same filesystem as the project's `node_modules` for
hardlink dedup to work. If they are on different filesystems, guroku falls
back to copying transparently. Keep both on the same disk for best
performance.

---

## 8. Glibc / musl

- **glibc**: pre-built Linux x86_64 binaries are linked against glibc 2.28
  or newer. This corresponds to Ubuntu 20.04, Debian 11, RHEL 8, and
  equivalents. Older distributions (Ubuntu 18.04, CentOS 7) are not
  supported; they will fail at startup with a `GLIBC_2.28 not found` error.
- **musl**: there is no pre-built musl binary in v1.0. To run on Alpine
  Linux or in a `scratch` / `distroless` container based on musl, build
  from source on the target distribution:

  ```sh
  apk add rust cargo git build-base openssl-dev
  cargo install --locked --path . --target x86_64-unknown-linux-musl
  ```

  A pre-built musl binary is on the v1.x roadmap once we have a
  reproducible static build pipeline. There is no committed ETA.

---

## 9. Forward-compatibility commitments

The full policy lives in `docs/STABILITY.md`. Summary:

- **Lockfile**: forward-compatible. Older guroku tolerates unknown fields
  added by newer guroku within the v1.x series. Older guroku will preserve
  unknown fields when round-tripping. Newer guroku will not introduce a
  breaking lockfile change without bumping `lockfileVersion`, which would
  require a major release.
- **Rust API**: the `guroku-core` crate follows semver. v1.x is API-stable
  on its public surface. Internal crates (`guroku-resolver-internals`,
  `guroku-fetch-internals`, etc.) carry no stability guarantee and may
  change between any two minor versions.
- **CLI**: stdout for documented commands is treated as a stable interface.
  Adding new fields to JSON output (`--format=json`) is non-breaking;
  removing or renaming fields requires a major release. Stderr is for
  humans and may change in any release.
- **Configuration files**: `guroku.toml`, `.guroku/config.toml`, and the
  npmrc-style `~/.npmrc` reader are all forward-compatible: unknown keys
  are ignored with a warning rather than rejected.

---

## 10. Known incompatibilities

The following npm/yarn/pnpm features are **not supported** in v1.0. Each
entry includes the workaround we recommend until support lands.

- **npm aliases (`alias@npm:real@^1`)**: not supported. Manifests using
  aliased dependencies will fail to resolve with a clear error pointing at
  the offending entry. Workaround: depend on the real package directly. We
  intend to add aliases in v1.x; tracked in the roadmap.
- **Workspace protocol (`workspace:*`, `workspace:^`, `workspace:~`)**:
  not supported until inter-dep linking ("workspaces") lands. Workspaces
  are a v1.x roadmap item. Workaround: use `file:./path/to/package`, which
  will create a symlink-style dependency that works for most local-link
  use cases.
- **`link:./path` (yarn-style protocol)**: not supported. Use
  `file:./path` instead. The semantics are slightly different (`file:`
  copies on install in npm; guroku always symlinks for paths) but the
  end result is comparable.
- **`bundleDependencies` / `bundledDependencies`**: not honoured during
  install. Packages that ship a `bundleDependencies` field will have those
  dependencies *re-resolved* and downloaded fresh rather than used from
  the tarball. This is safe but wastes bandwidth. A flag to honour
  bundled deps is on the v1.x roadmap.
- **`peerDependenciesMeta.optional`**: partially supported. guroku
  installs peer deps marked optional when they are present in the
  resolution graph but does not warn loudly when they are absent. The
  warning level matches npm 8, not npm 10.
- **`overrides` with nested selectors**: top-level overrides are
  supported. Nested selectors (`{ "foo": { "bar": "1.2.3" } }`) work for
  one level of nesting; deeper nesting is parsed but not always applied.
  Track via the `overrides` label on the issue tracker.
- **Custom install scripts that rebuild against system libraries**:
  `node-gyp` is invoked transparently when a package uses it, but guroku
  does not ship Python or a C/C++ toolchain. Users must provide both.
  This matches npm.

---

## 11. Reporting compatibility issues

If you hit a real-world compatibility problem that this document does not
already cover, please open an issue with the `compat` label. Include:

1. **Operating system and architecture** (e.g. `Ubuntu 22.04 x86_64`,
   `macOS 14.4 aarch64`, `Windows 11 23H2 x86_64`).
2. **guroku version** (`guroku --version`).
3. **Rust toolchain** if you built from source (`rustc --version`).
4. **Registry type and version** if relevant (e.g. `Verdaccio 5.29.0`,
   `Artifactory 7.77.x`, `GitHub Packages`).
5. **Filesystem** if the issue is install-time and you suspect linking
   (e.g. `ext4 on LUKS`, `APFS`, `SMB share to Synology DSM 7.2`).
6. **A minimal reproducer**: ideally a `guroku.toml` and `guroku.lock`
   pair that exhibits the issue with a single `guroku install`. If the
   issue depends on a private registry, please describe the relevant
   configuration without leaking secrets.
7. **Expected vs. actual behaviour**, plus the full error output. Run
   with `RUST_LOG=guroku=debug` if the failure is non-obvious.

We triage `compat`-labelled issues weekly. Issues that block users on a
Tier 1 platform (Linux x86_64/aarch64, macOS x86_64/aarch64) are
prioritized; Tier 2 and Tier 3 issues are addressed as time permits.

For security-sensitive compatibility issues (e.g. a registry that returns
malformed metadata that crashes the parser), please follow `SECURITY.md`
instead of filing a public issue.

---

## Appendix: cross-references

- `docs/MSRV.md` — Minimum Supported Rust Version policy and bump cadence.
- `docs/STABILITY.md` — Forward-compatibility and semver guarantees for
  the lockfile, Rust API, CLI, and configuration files.
- `ROADMAP.md` — Planned compatibility additions (workspace protocol,
  npm aliases, lockfile importer, musl binary, GitHub Packages tarball
  header).
- `CHANGELOG.md` — Per-release compatibility deltas. MSRV bumps and new
  registry support always appear here.
- `SECURITY.md` — How to report security-sensitive compatibility bugs.
