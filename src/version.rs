//! Thin wrapper over `node_semver` so the rest of the crate doesn't have to
//! import it directly. We re-export `Version` and `Range` and add a couple of
//! small helpers for picking the highest version in a range.

use crate::error::{GurokuError, Result};
pub use node_semver::{Range, Version};

/// Parse an npm-style range like `^1.2.3`, `~1.0`, `>=1 <2`, `1.x`,
/// `1.2 || 2.0`, or `*`. The empty string is treated as `*`.
pub fn parse_range(spec: &str) -> Result<Range> {
    let trimmed = spec.trim();
    let s = if trimmed.is_empty() { "*" } else { trimmed };
    s.parse::<Range>()
        .map_err(|_| GurokuError::InvalidVersionSpec {
            name: String::new(),
            spec: spec.to_string(),
        })
}

pub fn parse_version(s: &str) -> Result<Version> {
    s.parse::<Version>()
        .map_err(|_| GurokuError::Other(format!("invalid version: `{s}`")))
}

/// Return the highest `Version` from `candidates` that satisfies `range`.
/// `candidates` is iterated as version strings. Invalid version strings are
/// silently skipped (npm registries occasionally include malformed entries).
pub fn max_satisfying<'a, I>(candidates: I, range: &Range) -> Option<Version>
where
    I: IntoIterator<Item = &'a str>,
{
    candidates
        .into_iter()
        .filter_map(|s| s.parse::<Version>().ok())
        .filter(|v| range.satisfies(v))
        .max()
}
