# Changelog

All notable changes to the guroku project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
