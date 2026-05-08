//! `guroku` — a fast, Rust-powered package manager for the JavaScript ecosystem.
//!
//! `guroku` is the library underneath the `guroku` CLI. It is also designed to
//! be embedded directly: build tools, CI runners, and bespoke installers can
//! drive resolution, fetching, and linking from Rust without spawning the
//! binary. v1.0 is the first release with a stability commitment for the
//! public Rust API and the `guroku.lock` format.
//!
//! # Quickstart (CLI)
//!
//! ```sh
//! cd my-project   # contains a package.json
//! guroku install
//! guroku run test -- --watch
//! guroku audit
//! ```
//!
//! # Quickstart (embedding)
//!
//! ```no_run
//! # async fn run() -> guroku::Result<()> {
//! use guroku::prelude::*;
//!
//! // Read the project manifest.
//! let manifest = Manifest::read_from(std::path::Path::new("./package.json"))?;
//!
//! // Build a registry client honouring the local .npmrc.
//! let client = RegistryClient::from_npmrc(std::path::Path::new("."))?;
//!
//! // Resolve every declared dependency.
//! let roots: Vec<(String, String)> = manifest
//!     .all_dependencies()
//!     .map(|(k, v)| (k.clone(), v.clone()))
//!     .collect();
//! let resolution = guroku::resolver::resolve(&client, &roots).await?;
//! println!("resolved {} packages", resolution.len());
//! # Ok(())
//! # }
//! ```
//!
//! # Module map
//!
//! The crate is laid out as one module per concern. Most embedders only need
//! the [`prelude`].
//!
//! - [`manifest`] — read and write `package.json`.
//! - [`registry`] — npm registry HTTP client (with `.npmrc` integration).
//! - [`resolver`] — BFS dependency resolver with overrides support.
//! - [`lockfile`] — `guroku.lock` reader/writer.
//! - [`linker`] — strict pnpm-style `node_modules` writer.
//! - [`store`] — content-addressable store at `~/.guroku/cas/`.
//! - [`integrity`] — SHA-512 verification of registry tarballs.
//! - [`tarball`] — `.tgz` extraction with path-traversal guards.
//! - [`http_cache`] — ETag-aware metadata cache.
//! - [`cache`] — on-disk path helpers (`~/.guroku/...`).
//! - [`scripts`] — lifecycle script runner.
//! - [`workspaces`] — workspace discovery.
//! - [`npmrc`] — `.npmrc` parser.
//! - [`audit`] — npm advisory lookup.
//! - [`overrides`] — `package.json#overrides` lookup.
//! - [`specs`] — dependency-spec classifier (range / file: / git+).
//! - [`git`] — git-clone driver for git deps.
//! - [`version`] — npm semver wrappers (re-exports `node-semver`).
//! - [`error`] — [`GurokuError`] and the crate-wide [`Result`].
//! - [`cli`] — clap definitions for the `guroku` binary (used by `main.rs`).
//! - [`commands`] — command handlers behind each CLI subcommand.
//!
//! # Stability
//!
//! From v1.0 onward, the lockfile schema (`lockfileVersion: 1`) and the
//! signatures of items in [`prelude`] are covered by SemVer. Breaking
//! changes to either bump the major version. Internals (anything not in
//! `prelude` and any item marked `#[doc(hidden)]`) may evolve in minor
//! releases. See `docs/STABILITY.md`.
//!
//! # Minimum supported Rust version
//!
//! 1.75. Bumps follow a 6-month deprecation window — see `docs/MSRV.md`.

pub mod audit;
pub mod cache;
pub mod cli;
pub mod commands;
pub mod error;
pub mod git;
pub mod http_cache;
pub mod integrity;
pub mod linker;
pub mod lockfile;
pub mod logging;
pub mod manifest;
pub mod npmrc;
pub mod overrides;
pub mod prelude;
pub mod registry;
pub mod resolver;
pub mod scripts;
pub mod specs;
pub mod store;
pub mod tarball;
pub mod version;
pub mod workspaces;

pub use error::{GurokuError, Result};
