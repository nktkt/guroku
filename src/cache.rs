use crate::error::{GurokuError, Result};
use std::path::PathBuf;

/// Root of the per-user guroku cache: `$HOME/.guroku`.
pub fn home() -> Result<PathBuf> {
    let base = dirs::home_dir().ok_or(GurokuError::NoCacheDir)?;
    Ok(base.join(".guroku"))
}

/// `~/.guroku/store` — the content-addressable store (extracted packages).
/// In v0.1 it is just a flat layout: `store/<name>/<version>/`.
pub fn store_dir() -> Result<PathBuf> {
    Ok(home()?.join("store"))
}

pub fn package_dir(name: &str, version: &str) -> Result<PathBuf> {
    Ok(store_dir()?.join(safe_segment(name)).join(version))
}

/// `~/.guroku/cache/tarballs` — raw downloaded `.tgz` files (optional).
pub fn tarball_cache_dir() -> Result<PathBuf> {
    Ok(home()?.join("cache").join("tarballs"))
}

/// Replace `/` in scoped package names (`@scope/name`) so that they map to a
/// single directory level on disk.
fn safe_segment(name: &str) -> String {
    name.replace('/', "+")
}
