# Changelog

All notable changes to the guroku project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.2.0] - 2026-05-08

The "It backtracks properly" milestone. First feature minor that integrates pubgrub-the-crate as the default dependency resolver. The v1.0 stability surface (`guroku::prelude`, lockfile schema, CLI surface) is unchanged; v1.1's BFS + single-step backtracking remains available via opt-out.

### Added
- **`guroku::pubgrub_resolver`** module:
  - `NpmVersion` newtype around `node_semver::Version` implementing `pubgrub::version::Version` (`lowest()` returns `0.0.0`; `bump()` returns patch+1, stripping pre-release tags).
  - `resolve_with_pubgrub(client, roots, manifest)` — async entry point. Two-phase: BFS-prefetches every package name reachable in the dep graph, then runs `pubgrub::solver::resolve` synchronously against the prefetched cache.
  - File:/git: roots transparently fall back to `resolver::resolve_with_manifest_overrides` (the v1.1 BFS path).
- **`commands::install::run`** uses pubgrub by default. Set `GUROKU_RESOLVER=bfs` to force the v1.1 BFS path.
- **Range translation**: pubgrub `Range<NpmVersion>` is built as a union of singletons over the prefetched candidate set. Correct against the candidate set; documented in `docs/internals/range-conversion.md`.
- **Conflict explainer**: pubgrub's `DefaultStringReporter::report` is rendered into `GurokuError::ResolutionConflict.requested_by` for pubgrub-produced conflicts. The v1.1 path-formatted conflict format is preserved for the BFS path.
- New tests: `tests/pubgrub_npm_version*.rs`, `tests/pubgrub_resolver_simple.rs`, `tests/pubgrub_diamond_conflict.rs`, `tests/pubgrub_cascade_backtrack.rs`, `tests/pubgrub_conflict_report_format.rs`, `tests/pubgrub_overrides_applied.rs`, `tests/pubgrub_alias_root.rs`, `tests/pubgrub_file_root_falls_back.rs`, `tests/pubgrub_resolver_smoke_lib.rs`, `tests/pubgrub_does_not_export_pubgrub_crate.rs`, `tests/pubgrub_smoke_async_runtime.rs`, `tests/pubgrub_resolver_module_listed.rs`, `tests/pubgrub_npm_version_pubgrub_dep_visible.rs`, `tests/cargo_toml_pubgrub_dep.rs`, `tests/cli_help_no_pubgrub_leak.rs`, `tests/cli_help_v1_2_no_new_subcommands.rs`, `tests/cli_install_help_v1_2.rs`, `tests/cli_version_includes_v1_2.rs`, `tests/lib_v1_2_smoke.rs`, `tests/lockfile_v1_2_compat.rs`, `tests/api_stability_v1_2_prelude.rs`, `tests/error_kind_v1_2.rs`, `tests/manifest_unchanged_v1_2.rs`, `tests/specs_unchanged_v1_2.rs`, `tests/overrides_unchanged_v1_2.rs`, `tests/version_unchanged_v1_2.rs`, `tests/registry_unchanged_v1_2.rs`, `tests/release_notes_present_v1_2.rs`.
- Examples: `examples/with-cascade-backtrack/` (package.json, README.md, .gitignore).
- Docs: `docs/v1.2-release-notes.md`, `docs/migration/v1.1-to-v1.2.md`, `docs/pubgrub-resolver.md` (user-facing). Internals: `docs/internals/{pubgrub-integration, range-conversion, two-phase-resolver, pubgrub-version-trait, pubgrub-conflict-explainer, pubgrub-error-translation, v1.2-checklist, v1.2-architecture-decisions}.md`. Contributing: `docs/contributing/{v1.2-features-overview, pubgrub-debugging}.md`.
- Assets: `assets/v1.2-banner.txt`, `pubgrub-flow.txt`, `install-pipeline-v1.2.txt`, `v1.2-summary.txt`.
- Templates: `.github/PULL_REQUEST_TEMPLATE/pubgrub_change.md`, `.github/ISSUE_TEMPLATE/pubgrub_resolution_failure.yml`.
- CI: `.github/workflows/pubgrub-fuzz.yml`.

### Changed
- New runtime dependency: `pubgrub = "0.2"`.
- Crate version bumped to 1.2.0.
- `commands::install::run` resolver dispatch (env-var-gated; no CLI surface change).

### Stability commitments (additive)
- `guroku::prelude` items, lockfile schema (`lockfileVersion: 1`), and CLI surface unchanged from v1.0.
- v1.0/v1.1 lockfiles read by v1.2 unchanged; lockfile bytes for unchanged inputs are bit-compatible.
- `Resolved`, `Resolution`, `ResolutionConflict` shapes unchanged from v1.1.

### Known limitations
- pubgrub 0.2 pinned. v1.3 will track pubgrub 0.3 once released.
- Range translation is candidate-set-based, not structural. Means we prefetch the closure of all reachable names; future v1.3 may cut this for performance via structural translation.
- `aliased_from`-style transitive aliasing still pending.
- File:/git: roots use the BFS resolver internally; v1.3 may unify.
- Workspace inter-dep linking still pending (v1.3).
- Macrobench harness vs npm/pnpm/bun/yarn deferred (microbenches scaffolded since v1.0).

## [1.1.0] - 2026-05-08

The "It resolves better" milestone. First feature minor after v1.0's stability commitment, additive on the v1.0 surface.

### Added
- `DepSpec::Alias { real_name, inner }` — npm-style dependency aliases like `"react-old": "npm:react@^16"`. The classifier splits on the LAST `@` so scoped real names (`@types/node`) round-trip. `unparse` emits `npm:<real>@<inner>`.
- `overrides::lookup_with_path(&Manifest, &[&str]) -> Option<String>` — path-aware override lookup. Honours path-keyed `"a > b > c"` overrides and yarn-style `**/<name>` glob resolutions. Whitespace around `>` tolerated.
- Override precedence ladder (highest first): exact-path overrides → flat overrides → exact-path resolutions → flat resolutions → `**/<name>` resolutions.
- `overrides::OverrideEntry`, `OverrideKind` (Flat/Path/Glob/Unknown), `OverrideSource`, `classify_entries()` — for diagnostics.
- `resolver::resolve_with_manifest_overrides(client, roots, manifest)` — preferred entry point that wires path-keyed and glob overrides through the resolver.
- Resolver now tracks the dep-graph path as `Vec<String>` per queue item; conflicts surface a "a > b > c"-formatted path in `ResolutionConflict.requested_by`.
- `Resolved.aliased_from: Option<String>` — populated with the registry name for aliased entries; None otherwise.
- Single-step backtracking on diamond conflicts: when a transitive's existing pick can't satisfy a newly-arrived range, the resolver walks the candidate list highest-first to find a version satisfying both ranges. Honest-scoped: full PubGrub integration is deferred to v1.2.
- New tests covering aliases, path-keyed overrides, glob resolutions, single-step backtracking, conflict path formatting, and v1.0 compatibility of the `lookup` shim.
- New fixtures: `tests/fixtures/manifest_with_npm_alias.json`, `manifest_with_path_keyed_overrides.json`, `manifest_with_glob_resolutions.json`.
- New examples: `examples/with-npm-alias/`, `examples/with-path-override/`, `examples/with-glob-resolution/`.
- Docs: `docs/aliases.md`, `docs/path-keyed-overrides.md`, `docs/glob-resolutions.md`, `docs/v1.1-release-notes.md`, `docs/migration/v1.0-to-v1.1.md`. Internals: `docs/internals/{npm-aliases, path-keyed-overrides, glob-resolutions, single-step-backtrack, path-tracking-in-resolver, aliasing-and-the-linker, v1.1-checklist}.md`. Contributing: `docs/contributing/v1.1-features-overview.md`.
- Assets: `assets/v1.1-banner.txt`, `resolver-backtracking-flow.txt`, `install-pipeline-v1.1.txt`, `override-precedence-table.txt`.
- Templates: `.github/PULL_REQUEST_TEMPLATE/resolver_change.md`, `.github/ISSUE_TEMPLATE/resolution_conflict.yml`.
- CI: `.github/workflows/resolver-fuzz.yml` runs resolver/override tests with `--nocapture` and smoke-runs the version_satisfies bench.

### Changed
- `DepSpec` is now `#[non_exhaustive]`. External `match` blocks must include a `_` arm.
- `Resolved` gained an `aliased_from` field. v1.0 callers that constructed `Resolved` directly need to add `aliased_from: None`.
- `ResolutionConflict.requested_by` now carries a `>`-joined dep-graph path instead of just a flat name.
- CAS map keys (`install::install_from_resolution`) are the LOCAL dependency name (the `Resolution` map key) rather than `info.name`. Aliased entries previously would have mis-keyed; in v1.0 there were no aliases so this was unobservable.
- `LinkedPackage.name` is now the LOCAL name. Same reason.
- `commands::install::run` now calls `resolver::resolve_with_manifest_overrides` instead of `resolver::resolve_with_overrides`. The latter remains available; the former is preferred.
- Crate version bumped to 1.1.0.

### Stability commitments (additive)
- Items in `guroku::prelude` and the v1.0 CLI surface are unchanged. v1.0 callers compile against v1.1 with at most adding a `_` arm to `match` on `DepSpec` and adding `aliased_from: None` if constructing `Resolved` literals.
- Lockfile schema is unchanged. v1.0 lockfiles are read by v1.1 with no migration.

### Known limitations
- Full PubGrub-the-crate integration deferred to v1.2 (npm-semver ↔ pubgrub-Range conversion non-trivial; trait is sync-only). v1.1 ships single-step backtracking only.
- Path-keyed overrides do not yet support wildcards within a path (`a > * > b`), OR patterns, or negation.
- Glob resolutions only honour the literal `**/<name>` form. `pkg/**/foo`, `*-helper`, brace expansion not supported.
- Aliases only propagate through ROOT deps. Transitive aliasing requires full dep-tree rewriting (v1.x backlog).
- Workspace inter-dep linking still pending.

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

[Unreleased]: https://github.com/nktkt/guroku/compare/v1.2.0...HEAD
[1.2.0]: https://github.com/nktkt/guroku/releases/tag/v1.2.0
[1.1.0]: https://github.com/nktkt/guroku/releases/tag/v1.1.0
[1.0.0]: https://github.com/nktkt/guroku/releases/tag/v1.0.0
[0.5.0]: https://github.com/nktkt/guroku/releases/tag/v0.5.0
[0.4.0]: https://github.com/nktkt/guroku/releases/tag/v0.4.0
[0.3.0]: https://github.com/nktkt/guroku/releases/tag/v0.3.0
[0.2.0]: https://github.com/nktkt/guroku/releases/tag/v0.2.0
[0.1.0]: https://github.com/nktkt/guroku/releases/tag/v0.1.0
