use crate::error::{GurokuError, Result};
use flate2::read::GzDecoder;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use tar::Archive;

/// Extract an npm-style tarball (a `.tgz` whose entries live under a top-level
/// `package/` directory) into `dest`. The leading `package/` segment is stripped.
pub fn extract(bytes: &[u8], dest: &Path) -> Result<()> {
    fs::create_dir_all(dest).map_err(|e| GurokuError::Io {
        path: dest.to_path_buf(),
        source: e,
    })?;

    let gz = GzDecoder::new(Cursor::new(bytes));
    let mut archive = Archive::new(gz);
    archive.set_preserve_permissions(false);
    archive.set_overwrite(true);

    for entry in archive
        .entries()
        .map_err(|e| GurokuError::Tarball(e.to_string()))?
    {
        let mut entry = entry.map_err(|e| GurokuError::Tarball(e.to_string()))?;
        let path = entry
            .path()
            .map_err(|e| GurokuError::Tarball(e.to_string()))?
            .into_owned();

        let stripped = strip_leading_segment(&path);
        let Some(rel) = stripped else {
            continue;
        };
        if rel.as_os_str().is_empty() {
            continue;
        }
        if !is_safe(&rel) {
            return Err(GurokuError::Tarball(format!(
                "tarball contains unsafe path: {}",
                path.display()
            )));
        }

        let out = dest.join(&rel);
        let entry_type = entry.header().entry_type();

        if entry_type.is_dir() {
            fs::create_dir_all(&out).map_err(|e| GurokuError::Io {
                path: out.clone(),
                source: e,
            })?;
            continue;
        }

        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent).map_err(|e| GurokuError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry
            .read_to_end(&mut buf)
            .map_err(|e| GurokuError::Tarball(e.to_string()))?;
        fs::write(&out, &buf).map_err(|e| GurokuError::Io {
            path: out.clone(),
            source: e,
        })?;
    }

    Ok(())
}

fn strip_leading_segment(path: &Path) -> Option<PathBuf> {
    let mut comps = path.components();
    comps.next()?;
    Some(comps.as_path().to_path_buf())
}

fn is_safe(path: &Path) -> bool {
    use std::path::Component;
    for c in path.components() {
        match c {
            Component::Normal(_) | Component::CurDir => {}
            _ => return false,
        }
    }
    true
}
