//! Re-exports of the most commonly used types for embedders.
//!
//! ```no_run
//! use guroku::prelude::*;
//! ```
//!
//! From v1.0 onward, items re-exported here are covered by SemVer: their
//! signatures will not change in compatible ways across minor releases.
//! New items may be added.

pub use crate::error::{GurokuError, Result};
pub use crate::lockfile::{Lockfile, PackageLock, LOCKFILE_NAME, LOCKFILE_VERSION};
pub use crate::manifest::Manifest;
pub use crate::registry::{PackageMetadata, RegistryClient, VersionInfo, DEFAULT_REGISTRY};
pub use crate::resolver::{Resolution, Resolved};
pub use crate::specs::{classify as classify_spec, DepSpec, GitRef};
pub use crate::version::{max_satisfying, parse_range, parse_version, Range, Version};
