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

/// Make sure the CAS entry for `tarball_bytes` exists on disk and return its
/// path. If a prior process already populated the entry, this is a near-noop.
pub fn ensure_extracted(tarball_bytes: &[u8]) -> Result<PathBuf> {
    let hex = sha512_hex(tarball_bytes);
    let target = cache::cas_entry(&hex)?;
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
        // Cleanup of a previously interrupted extraction.
        let _ = fs::remove_dir_all(&tmp);
    }
    tarball::extract(tarball_bytes, &tmp)?;
    write_marker(&tmp)?;

    if target.exists() {
        // Another process won the race; keep their copy.
        let _ = fs::remove_dir_all(&tmp);
    } else {
        match fs::rename(&tmp, &target) {
            Ok(()) => {}
            Err(_) if target.exists() => {
                // Same race, second-place finisher; clean up our tmp.
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

/// Best-effort check that the CAS entry is fully populated. We rely on a
/// `.guroku-cas-ready` marker file written at the end of extraction so we
/// don't read a half-populated tree on a subsequent call.
fn marker_present(dir: &Path) -> bool {
    dir.join(".guroku-cas-ready").is_file()
}

fn write_marker(dir: &Path) -> Result<()> {
    let marker = dir.join(".guroku-cas-ready");
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
