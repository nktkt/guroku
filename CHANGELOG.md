# Changelog

All notable changes to the guroku project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2026-05-08

The "It's stable" milestone. First release with a SemVer commitment.

### Added
- `guroku::prelude` module re-exporting the v1.0 stable surface (`Lockfile`, `Manifest`, `RegistryClient`, `Resolution`, `Resolved`, `DepSpec`, `GitRef`, `classify_spec`, `Range`, `Version`, `parse_range`, `parse_version`, `max_satisfying`, plus `LOCKFILE_NAME`, `LOCKFILE_VERSION`, `DEFAULT_REGISTRY`, `GurokuError`, `Result`).
- Comprehensive crate-level rustdoc on `src/lib.rs` covering CLI quickstart, embedding quickstart, the module map, and the stability commitment.
- `[[bench]]` entries and criterion-based benchmark scaffolds for `lockfile_parse`, `manifest_parse`, `spec_classify`, `version_satisfies`.
- Pre-built Windows binaries (x86_64-pc-windows-msvc, aarch64-pc-windows-msvc) in the release workflow.
- New CI workflows: `semver-checks.yml` (cargo-semver-checks against the published baseline), `cross-platform-test.yml` (explicit Win/Mac/Linux matrix), `bench-baseline.yml` (criterion artifact uploads).
- Stability tests: `tests/api_stability_re_exports.rs`, `tests/api_stability_prelude.rs`, `tests/lockfile_v1_compat.rs`, `tests/lockfile_unknown_field.rs`, `tests/lockfile_format_stability.rs`, `tests/manifest_unknown_field.rs`, `tests/manifest_full_round_trip.rs`, `tests/cli_help_v1.rs`, `tests/cli_subcommand_inventory.rs`, `tests/cli_version_includes_v1.rs`, `tests/version_v1_constants.rs`, `tests/integrity_known_vector.rs`, `tests/error_kind_classification.rs`, `tests/cache_paths_v1.rs`, `tests/registry_user_agent.rs`, `tests/minimal_imports.rs`, `tests/lib_doctest_smoke.rs`.
- v1 fixtures: `tests/fixtures/v1_minimal_lockfile.json`, `v1_realistic_lockfile.json`, `v1_manifest_full.json`.
- Stability docs: `docs/STABILITY.md`, `docs/MSRV.md`, `docs/deprecation-policy.md`, `docs/api-overview.md`, `docs/embedding-guroku.md`, `docs/internals/api-design.md`, `docs/contributing/api-stability.md`, `docs/contributing/v1.0-features-overview.md`, `docs/internals/v1.0-checklist.md`.
- Migration guide `docs/migration/v0.5-to-v1.0.md` and prose release notes `docs/v1.0-release-notes.md`.
- Embedding example `examples/embedding-rust/` (Cargo.toml + src/main.rs + README + .gitignore).
- Cross-installer benchmark fixture `examples/benchmark-target/` (package.json + README) plus `docs/benchmark-methodology.md`.
- Top-level `ANNOUNCEMENT.md` and `COMPATIBILITY.md`.
- ASCII assets: `assets/v1.0-banner.txt`, `api-surface.txt`, `install-pipeline-v1.0.txt`.
- PR template variant `PR_TEMPLATE/api_change.md` and issue template `ISSUE_TEMPLATE/api_compatibility.yml`.

### Changed
- `GurokuError` is now `#[non_exhaustive]`. External `match` blocks must include a `_` arm. New error variants in future minor releases will not break consumers.
- Crate version bumped to 1.0.0.

### Stability commitments (new)
- Lockfile schema (`lockfileVersion: 1`) is SemVer-stable for the v1.x line. Forward-compatibility tests baked in via `tests/lockfile_unknown_field.rs`.
- Items re-exported by `guroku::prelude` are SemVer-stable. Renames require the deprecation cycle described in `docs/deprecation-policy.md`.
- The CLI surface (every subcommand, flag, and exit code documented as of v1.0) is SemVer-stable.

### Known limitations
- PubGrub-based resolver still future work (BFS sticky-first remains).
- Workspace inter-dep linking still pending.
- `--audit-level`, `--json`, `audit fix` deferred.
- Macrobench harness vs npm/pnpm/bun/yarn deferred (microbenches scaffolded, methodology documented).
- npm Basic auth, `${VAR}` interpolation in `.npmrc`, and `npm_config_*` env vars not yet supported.

## [0.5.0] - 2026-05-08

The "It plays nice" milestone.

### Added
- Private-registry support: `_authToken` from `.npmrc` is sent as `Authorization: Bearer <token>` on outgoing HTTP. `<scope>:registry=<url>` actually routes scoped fetches.
- `RegistryClient` now carries an `Npmrc`. `from_npmrc(cwd)` is the production constructor; `registry_for(name)` and `auth_for(url)` handle routing and auth.
- `file:./path` dependencies install via local-source linking (no CAS, no integrity check).
- `git+https://...`, `git+ssh://...`, `github:user/repo[#ref]`, `git://...` dependencies. Subprocess `git clone` into `~/.guroku/cache/git/<sha>/<safe-rev>/`. Idempotent via a `.git-ready` marker.
- `package.json#overrides` (npm 8+) and `resolutions` (yarn classic) for transitive version pinning. Simple flat `name → exact-version` form. Overrides win on conflict with resolutions.
- `guroku audit` — POSTs the lockfile package set to `<registry>/-/npm/v1/security/advisories/bulk` and prints a report. Non-zero exit on findings.
- New public modules: `guroku::specs`, `guroku::overrides`, `guroku::git`, `guroku::audit`, `guroku::commands::audit`.
- `Manifest` gained typed `overrides` and `resolutions` BTreeMap fields.
- `Resolved` gained `local_source: Option<PathBuf>` so the install pipeline can skip the CAS for file:/git: deps.
- New errors: `FileDepMissingManifest`, `GitCommandFailed`, `AuditFailed`, `InvalidOverride`.
- `cache::git_cache_dir()` helper.

### Changed
- `RegistryClient` internal layout (now holds `Npmrc`). Public constructors are unchanged.
- `Resolved` API changed: `Resolved { info, local_source }`. Library-API consumers that constructed `Resolved` directly need to add `local_source: None`.
- `resolver::resolve_with_overrides` is the new public entry point used when the caller has overrides; `resolver::resolve` still works (calls `resolve_with_overrides` with an empty map).

### Known limitations
- Path-keyed overrides (`"foo > bar"`) and yarn glob keys (`**/foo`) parse but don't match anything yet.
- npm Basic auth (`auth=`, `_password=`) is not honoured. Bearer tokens only.
- `${VAR}` interpolation in `.npmrc` not yet supported.
- `npm_config_*` environment variables not yet read.
- Git submodules and sparse checkouts not supported.
- `--audit-level`, `--json`, `audit fix` not yet supported.
- Workspace inter-dep linking still pending (deferred to v0.6).

## [0.4.0] - 2026-05-06

The "It's usable" milestone.

### Added
- Lifecycle scripts during `guroku install`: root-level `preinstall`, `install`, `postinstall`, `prepare`; per-package `preinstall`/`install`/`postinstall` (best-effort, warn on failure).
- `--ignore-scripts` flag for `guroku install`.
- `guroku run [<script>] [-- <args>]` — list scripts, run by name, forward args.
- `guroku exec <command> [args...]` — run a binary, looking at `node_modules/.bin/` first then PATH.
- `guroku workspaces` — list discovered workspace packages.
- `node_modules/.bin/` symlinks for direct deps' `bin` entries (string and object forms).
- `.npmrc` reading: project-local `<cwd>/.npmrc` plus user `~/.npmrc`; `registry=` and `<scope>:registry=` honoured. `_authToken=` parsed but not yet sent (v0.5).
- New public modules: `guroku::scripts`, `guroku::npmrc`, `guroku::workspaces`, `guroku::commands::{run,exec,workspaces}`.
- `Manifest::bin_entries()` and `Manifest::workspace_globs()` helpers normalising the npm/pnpm shape variants.
- `RegistryClient::from_npmrc(cwd)` constructor used by all install paths.
- `linker::populate_bin_dir`.
- New error variants: `ScriptFailed`, `ScriptSpawnFailed`, `NoSuchScript`, `WorkspaceMisconfigured`, `BinNotFound`.
- New deps: `glob`, `shell-words`.

### Changed
- `Manifest` gained typed `scripts`, `bin`, `workspaces` fields. JSON `scripts` no longer lands in `manifest.other`.
- `LinkedPackage` gained `bin_entries`. v0.3 callers that constructed it directly need to add `bin_entries: vec![]` (or any populated value).
- `commands::install::run` now takes `(cwd, frozen_lockfile, ignore_scripts)`.

### Known limitations
- Workspace inter-dep linking not yet wired (planned v0.5).
- `.npmrc` `_authToken` parsed but unused on outgoing requests (planned v0.5).
- No `npm_*` env vars exported to scripts (planned v0.4.x).
- `${VAR}` interpolation in `.npmrc` not supported (planned v0.4.x).
- No `guroku dlx` (planned v0.4.x).
- Resolver still BFS sticky-first (PubGrub still future).

## [0.3.0] - 2026-05-06

The "It's fast" milestone.

### Added
- Content-addressable store at `~/.guroku/cas/<sha[0:2]>/<sha[2:]>`, keyed by tarball SHA-512. Atomic inserts via tmp-then-rename + a `.guroku-cas-ready` marker.
- `src/store.rs`: `ensure_extracted` / `ensure_extracted_at`, `CAS_READY_MARKER`.
- Hardlink-based linker (`linker::link_hardlink_tree`), with a copy fallback for cross-filesystem cases.
- Strict pnpm-style `node_modules/.guroku/<name>@<version>/node_modules/<name>/` layout (`linker::populate_node_modules`). Sibling symlinks for declared deps; surface symlinks for direct deps; full handling of scoped names (`@scope+name@<version>` on disk, `@scope/name` in node_modules).
- `src/http_cache.rs`: ETag-aware metadata cache at `~/.guroku/cache/metadata/`. `RegistryClient::fetch_metadata` now sends `If-None-Match` and treats `304 Not Modified` as a cache hit.
- `RegistryClient::without_http_cache()` builder method (test/library opt-out).
- `src/cache.rs` helpers: `cas_dir`, `cas_entry`, `metadata_cache_dir`, `metadata_cache_entry`, `metadata_etag_entry`, `safe_segment` (now `pub`).
- Parallel root-metadata prefetch in `resolver::resolve` (`FuturesUnordered`).
- New public modules: `guroku::store`, `guroku::http_cache`.

### Changed
- `commands::install::install_from_resolution` rewritten around `fetch_into_cas` + `populate_node_modules`. Both `install` and `install --frozen-lockfile` paths now share the CAS code path.
- `commands::install::run` produces the strict `node_modules` layout instead of a flat copy.
- The previous `~/.guroku/store/<name>/<version>` directory is no longer written (still readable by older guroku for back-compat). `cache::store_dir` and `cache::package_dir` are retained as deprecated helpers.

### Fixed
- Concurrent installs of the same package no longer race-corrupt the store. The atomic-rename pattern ensures only one writer wins.

### Known limitations
- No `guroku store gc` yet — the CAS grows unboundedly. Workaround: `rm -rf ~/.guroku/cas`.
- Per-tarball CAS only (not per-file like pnpm). Two patch-versions of the same package don't share bytes.
- Strict layout on Windows requires Developer Mode (or admin) for symlinks. See `docs/internals/strict-layout-windows.md`.
- Resolver still BFS sticky-first; PubGrub still on a future milestone.

## [0.2.0] - 2026-05-06

The "It resolves correctly" milestone.

### Added
- npm-style semver range resolution: `^1.2.3`, `~1.0`, `>=1 <2`, `1.x`, `^1 || ^2`, dist-tags (`latest`, `next`, ...). Backed by the `node-semver` crate.
- `guroku.lock` lockfile (JSON, `lockfileVersion: 1`). Written on every non-frozen install; read on every install.
- `guroku install --frozen-lockfile` — refuses to refresh; fails with `LockfileOutOfDate` if the lockfile and `package.json` have drifted. Recommended for CI.
- `Manifest` now reads and round-trips `peerDependencies` and `optionalDependencies`. (The resolver does not yet *install* peers or optionals.)
- New public modules: `guroku::version`, `guroku::resolver`, `guroku::lockfile`.
- New error variants: `ResolutionConflict`, `LockfileVersionMismatch`, `LockfileOutOfDate`, `InvalidVersionSpec`.
- New library API: `RegistryClient::with_default_registry`, `resolver::resolve`, `resolver::prefetch`, `Lockfile::{new,read_from,write_to,insert,contains,key}`, `version::{parse_range,parse_version,max_satisfying}`.

### Changed
- `PackageMetadata::resolve(spec)` now does proper semver matching instead of the v0.1 "fall back to latest" hack. Specs that match nothing return `NoMatchingVersion` instead of silently returning the latest version.
- `RegistryClient::default` was renamed to `RegistryClient::with_default_registry` (avoids `clippy::should_implement_trait`). Library-API consumers will need to update call sites.
- `guroku install` grew a `--frozen-lockfile` flag.
- `Manifest::remove_dependency` now also searches `optionalDependencies`.

### Fixed
- A spec like `^99` against a registry that only has `1.x` versions used to silently install `latest`. It now returns an error.

### Known limitations
- The resolver is breadth-first sticky-first-choice; it does NOT backtrack on conflicts (see `docs/internals/algorithm-notes.md`). PubGrub integration tracked for v0.3.
- Peer dependencies are not auto-installed.
- Optional dependencies are recorded but not installed.
- `node_modules` is still a flat copy. Content-addressable store + hardlinks land in v0.3.
- No lifecycle scripts. Lands in v0.4.

## [0.1.0] - 2026-05-06

### Added
- Initial release: the "It installs something" milestone.
- CLI with `install`, `add`, and `remove` subcommands (`clap`).
- `package.json` parser and writer (`Manifest`).
- npm registry HTTP client targeting `https://registry.npmjs.org` (`reqwest`).
- npm-style tarball extraction with `package/`-prefix stripping and path-traversal rejection (`flate2` + `tar`).
- SHA-512 integrity verification of `dist.integrity` (`sha2`).
- Per-user content store at `~/.guroku/store/<name>/<version>/`.
- Naive flat `node_modules` writer (recursive copy from store).
- Concurrent installs via `futures::stream::buffer_unordered` (concurrency = 8).
- Structured logging via `tracing` (filter via `GUROKU_LOG`).
- Public Rust library API for embedders.

### Known limitations
- No real semver resolution yet — non-exact specs fall back to `latest`. (Resolver lands in v0.2.)
- No lockfile yet. (`guroku.lock` lands in v0.2.)
- No content-addressable store or hardlinks. (Both land in v0.3.)
- No lifecycle scripts (`postinstall` etc.). (Lands in v0.4.)
- No workspaces, no peer/optional dependency handling.
- Windows is unexercised; macOS and Linux are the supported targets.

### Security
- Tarball extractor rejects entries with `..` components.
- Every download is verified against its registry-declared `sha512` integrity before extraction.

[Unreleased]: https://github.com/nktkt/guroku/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/nktkt/guroku/releases/tag/v0.1.0
