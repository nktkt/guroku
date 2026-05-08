//! Subprocess-driven git clone for git-dependency support (v0.5).
//!
//! Clones land at `~/.guroku/cache/git/<sha>/` keyed by the canonical
//! repo URL. A second `git checkout <ref>` is run for each unique ref,
//! producing a directory at `<sha>/<ref-or-default>/`.

use crate::cache;
use crate::error::{GurokuError, Result};
use crate::specs::GitRef;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Clone `git_ref` into the on-disk cache and return the working-tree path.
/// Subsequent calls with the same `(url, revision)` short-circuit.
pub fn ensure_cloned(git_ref: &GitRef) -> Result<PathBuf> {
    let cache_root = cache::git_cache_dir()?;
    fs::create_dir_all(&cache_root).map_err(|e| GurokuError::Io {
        path: cache_root.clone(),
        source: e,
    })?;

    let key = url_key(&git_ref.url);
    let revision = git_ref.revision.as_deref().unwrap_or("HEAD");
    let target = cache_root.join(&key).join(safe_ref(revision));

    if target.join(".git-ready").is_file() {
        return Ok(target);
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| GurokuError::Io {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }

    // Clean any stale partial clone.
    if target.exists() {
        let _ = fs::remove_dir_all(&target);
    }

    let url = git_ref.url.clone();
    let target_str = target.to_string_lossy().into_owned();
    let primary = if revision != "HEAD" {
        run_git(&[
            "clone",
            "--depth",
            "1",
            &format!("--branch={revision}"),
            &url,
            &target_str,
        ])
    } else {
        run_git(&["clone", "--depth", "1", &url, &target_str])
    };
    if primary.is_err() {
        // Fallback: full clone + checkout (handles arbitrary commit-ish).
        if target.exists() {
            let _ = fs::remove_dir_all(&target);
        }
        run_git(&["clone", &url, &target_str])?;
        if revision != "HEAD" {
            run_git_at(&target, &["checkout", "--quiet", revision])?;
        }
    }

    fs::write(target.join(".git-ready"), b"1").map_err(|e| GurokuError::Io {
        path: target.join(".git-ready"),
        source: e,
    })?;
    Ok(target)
}

fn url_key(url: &str) -> String {
    let mut h = Sha256::new();
    h.update(url.as_bytes());
    let digest = h.finalize();
    hex::encode(&digest[..8])
}

fn safe_ref(r: &str) -> String {
    r.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn run_git(args: &[&str]) -> Result<()> {
    // Drop the empty-string sentinel produced by the `--branch={...}` shortcut.
    let filtered: Vec<&str> = args.iter().filter(|s| !s.is_empty()).copied().collect();
    let output = Command::new("git").args(&filtered).output().map_err(|e| {
        GurokuError::GitCommandFailed {
            url: filtered.join(" "),
            detail: e.to_string(),
        }
    })?;
    if !output.status.success() {
        return Err(GurokuError::GitCommandFailed {
            url: filtered.join(" "),
            detail: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(())
}

fn run_git_at(cwd: &std::path::Path, args: &[&str]) -> Result<()> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| GurokuError::GitCommandFailed {
            url: args.join(" "),
            detail: e.to_string(),
        })?;
    if !output.status.success() {
        return Err(GurokuError::GitCommandFailed {
            url: args.join(" "),
            detail: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(())
}
