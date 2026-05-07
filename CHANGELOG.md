# Changelog

All notable changes to the guroku project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
