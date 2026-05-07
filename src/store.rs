//! Content-addressable storage for tarball-extracted package contents.
//!
//! v0.3 keeps a single global store at `~/.guroku/cas/<sha[0:2]>/<sha[2:]>/`,
//! keyed by the SHA-512 of the original tarball. Two registry records that
//! happen to ship the same bytes share an entry on disk.
//!
//! Inserts are atomic: we extract into a sibling temporary directory and
//! `rename` the result into place. This means concurrent installs of the
//! same package can race without corrupting each other.

use crate::cache;
use crate::error::{GurokuError, Result};
use crate::integrity::sha512_hex;
use crate::tarball;
use std::fs;
use std::path::{Path, PathBuf};

pub const CAS_READY_MARKER: &str = ".guroku-cas-ready";

/// Convenience wrapper using the user's `~/.guroku/cas` directory.
pub fn ensure_extracted(tarball_bytes: &[u8]) -> Result<PathBuf> {
    ensure_extracted_at(&cache::cas_dir()?, tarball_bytes)
}

/// Make sure the CAS entry for `tarball_bytes` exists under `cas_root` and
/// return its path. Tests can pass a `TempDir` as the root.
pub fn ensure_extracted_at(cas_root: &Path, tarball_bytes: &[u8]) -> Result<PathBuf> {
    let hex = sha512_hex(tarball_bytes);
    let target = cas_entry_under(cas_root, &hex)?;
    if marker_present(&target) {
        return Ok(target);
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| GurokuError::Io {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }

    let tmp = with_suffix(&target, ".tmp");
    if tmp.exists() {
        let _ = fs::remove_dir_all(&tmp);
    }
    tarball::extract(tarball_bytes, &tmp)?;
    write_marker(&tmp)?;

    if target.exists() {
        let _ = fs::remove_dir_all(&tmp);
    } else {
        match fs::rename(&tmp, &target) {
            Ok(()) => {}
            Err(_) if target.exists() => {
                let _ = fs::remove_dir_all(&tmp);
            }
            Err(e) => {
                return Err(GurokuError::Io {
                    path: target.clone(),
                    source: e,
                });
            }
        }
    }
    Ok(target)
}

fn cas_entry_under(cas_root: &Path, sha512_hex: &str) -> Result<PathBuf> {
    if sha512_hex.len() < 4 {
        return Err(GurokuError::Other(format!(
            "sha512 hex too short: `{sha512_hex}`"
        )));
    }
    let (prefix, rest) = sha512_hex.split_at(2);
    Ok(cas_root.join(prefix).join(rest))
}

fn marker_present(dir: &Path) -> bool {
    dir.join(CAS_READY_MARKER).is_file()
}

fn write_marker(dir: &Path) -> Result<()> {
    let marker = dir.join(CAS_READY_MARKER);
    fs::write(&marker, b"1").map_err(|e| GurokuError::Io {
        path: marker,
        source: e,
    })
}

fn with_suffix(p: &Path, suffix: &str) -> PathBuf {
    let mut s = p.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}
