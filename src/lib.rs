//! `guroku` — a fast, Rust-powered package manager for the JavaScript ecosystem.
//!
//! This crate is the implementation behind the `guroku` CLI. It is also exposed
//! as a library so that external tools can drive installs programmatically.
//! The public API is deliberately small in v0.1 and will stabilise around v1.0.

pub mod cache;
pub mod cli;
pub mod commands;
pub mod error;
pub mod http_cache;
pub mod integrity;
pub mod linker;
pub mod lockfile;
pub mod logging;
pub mod manifest;
pub mod registry;
pub mod resolver;
pub mod store;
pub mod tarball;
pub mod version;

pub use error::{GurokuError, Result};
