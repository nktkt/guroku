use crate::error::{GurokuError, Result};
use std::path::PathBuf;

/// Root of the per-user guroku cache: `$HOME/.guroku`.
pub fn home() -> Result<PathBuf> {
    let base = dirs::home_dir().ok_or(GurokuError::NoCacheDir)?;
    Ok(base.join(".guroku"))
}

/// `~/.guroku/store` — the per-name/version layout used by v0.1 and v0.2.
/// v0.3 still exposes this path for backward compatibility but the install
/// pipeline writes into the CAS directly.
pub fn store_dir() -> Result<PathBuf> {
    Ok(home()?.join("store"))
}

pub fn package_dir(name: &str, version: &str) -> Result<PathBuf> {
    Ok(store_dir()?.join(safe_segment(name)).join(version))
}

/// `~/.guroku/cas` — the content-addressable store added in v0.3. Each
/// extracted package lives under `cas/<sha512[0:2]>/<sha512[2:]>/`. Two
/// publishings of identical bytes share a single CAS entry.
pub fn cas_dir() -> Result<PathBuf> {
    Ok(home()?.join("cas"))
}

pub fn cas_entry(sha512_hex: &str) -> Result<PathBuf> {
    if sha512_hex.len() < 4 {
        return Err(GurokuError::Other(format!(
            "sha512 hex too short: `{sha512_hex}`"
        )));
    }
    let (prefix, rest) = sha512_hex.split_at(2);
    Ok(cas_dir()?.join(prefix).join(rest))
}

/// `~/.guroku/cache/tarballs` — raw downloaded `.tgz` files (reserved).
pub fn tarball_cache_dir() -> Result<PathBuf> {
    Ok(home()?.join("cache").join("tarballs"))
}

/// `~/.guroku/cache/metadata` — ETag-aware registry-metadata cache (v0.3).
/// `<name>.json` is the response body; `<name>.etag` holds the ETag string.
/// Scoped names are flattened the same way as `package_dir`.
pub fn metadata_cache_dir() -> Result<PathBuf> {
    Ok(home()?.join("cache").join("metadata"))
}

pub fn metadata_cache_entry(name: &str) -> Result<PathBuf> {
    Ok(metadata_cache_dir()?.join(format!("{}.json", safe_segment(name))))
}

pub fn metadata_etag_entry(name: &str) -> Result<PathBuf> {
    Ok(metadata_cache_dir()?.join(format!("{}.etag", safe_segment(name))))
}

/// Replace `/` in scoped package names (`@scope/name`) so that they map to a
/// single directory level on disk.
pub fn safe_segment(name: &str) -> String {
    name.replace('/', "+")
}
