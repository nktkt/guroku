//! ETag-based HTTP response cache for npm-registry metadata.
//!
//! Layout, under `~/.guroku/cache/metadata/`:
//!   - `<name>.json`  — the cached response body
//!   - `<name>.etag`  — the ETag header from the matching response
//!
//! Reads return an `Option<CachedMetadata>`; missing files are not an error.
//! Writes are best-effort: on disk failure we log and continue (callers
//! should treat the cache as advisory, not authoritative).

use crate::cache;
use crate::error::{GurokuError, Result};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct CachedMetadata {
    pub body: Vec<u8>,
    pub etag: Option<String>,
}

pub fn read(name: &str) -> Result<Option<CachedMetadata>> {
    let dir = cache::metadata_cache_dir()?;
    read_in(&dir, name)
}

pub fn write(name: &str, body: &[u8], etag: Option<&str>) -> Result<()> {
    let dir = cache::metadata_cache_dir()?;
    write_in(&dir, name, body, etag)
}

/// Test-friendly variant of `read` that takes the metadata-cache directory
/// explicitly. The `name` is used as a flat filename: scoped names are
/// flattened the same way as `cache::safe_segment`.
pub fn read_in(dir: &Path, name: &str) -> Result<Option<CachedMetadata>> {
    let safe = cache::safe_segment(name);
    let body_path = dir.join(format!("{safe}.json"));
    if !body_path.is_file() {
        return Ok(None);
    }
    let body = fs::read(&body_path).map_err(|e| GurokuError::Io {
        path: body_path,
        source: e,
    })?;
    let etag_path = dir.join(format!("{safe}.etag"));
    let etag = fs::read_to_string(&etag_path).ok().and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    Ok(Some(CachedMetadata { body, etag }))
}

pub fn write_in(dir: &Path, name: &str, body: &[u8], etag: Option<&str>) -> Result<()> {
    fs::create_dir_all(dir).map_err(|e| GurokuError::Io {
        path: dir.to_path_buf(),
        source: e,
    })?;
    let safe = cache::safe_segment(name);
    let body_path = dir.join(format!("{safe}.json"));
    fs::write(&body_path, body).map_err(|e| GurokuError::Io {
        path: body_path.clone(),
        source: e,
    })?;
    let etag_path = dir.join(format!("{safe}.etag"));
    match etag {
        Some(value) => {
            fs::write(&etag_path, value).map_err(|e| GurokuError::Io {
                path: etag_path,
                source: e,
            })?;
        }
        None => {
            let _ = fs::remove_file(&etag_path);
        }
    }
    Ok(())
}
