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

#[derive(Debug, Clone)]
pub struct CachedMetadata {
    pub body: Vec<u8>,
    pub etag: Option<String>,
}

pub fn read(name: &str) -> Result<Option<CachedMetadata>> {
    let body_path = cache::metadata_cache_entry(name)?;
    if !body_path.is_file() {
        return Ok(None);
    }
    let body = fs::read(&body_path).map_err(|e| GurokuError::Io {
        path: body_path,
        source: e,
    })?;
    let etag_path = cache::metadata_etag_entry(name)?;
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

pub fn write(name: &str, body: &[u8], etag: Option<&str>) -> Result<()> {
    let body_path = cache::metadata_cache_entry(name)?;
    if let Some(parent) = body_path.parent() {
        fs::create_dir_all(parent).map_err(|e| GurokuError::Io {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }
    fs::write(&body_path, body).map_err(|e| GurokuError::Io {
        path: body_path.clone(),
        source: e,
    })?;
    let etag_path = cache::metadata_etag_entry(name)?;
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
