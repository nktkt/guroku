//! Run shell scripts from `package.json#scripts` and lifecycle hooks.
//!
//! Scripts are executed via the system shell — `sh -c "<line>"` on Unix and
//! `cmd /c "<line>"` on Windows. The environment is augmented so that
//! `<project>/node_modules/.bin/` is the first entry on `PATH`, matching
//! how npm and pnpm run scripts.
//!
//! Failure modes follow npm's defaults: any non-zero exit code is fatal
//! unless the caller is in the per-package "best-effort" mode used during
//! the install pipeline (where a postinstall failure becomes a warning).

use crate::error::{GurokuError, Result};
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

/// Run a script body in `cwd`. `script_name` is purely cosmetic — used in
/// log lines and error messages.
pub fn run_in(cwd: &Path, script_name: &str, body: &str, env_path_extra: &[&Path]) -> Result<()> {
    let mut cmd = build_shell_command(body);
    cmd.current_dir(cwd);
    set_path(&mut cmd, env_path_extra);

    tracing::info!("> {script_name}: {body}");
    let status = cmd.status().map_err(|e| GurokuError::ScriptSpawnFailed {
        script: script_name.to_string(),
        detail: e.to_string(),
    })?;
    if !status.success() {
        return Err(GurokuError::ScriptFailed {
            script: script_name.to_string(),
            status: status.code().unwrap_or(-1),
        });
    }
    Ok(())
}

/// Like `run_in` but pass the user's extra args appended to the script body
/// (the `guroku run <script> -- <args>` shape).
pub fn run_in_with_args(
    cwd: &Path,
    script_name: &str,
    body: &str,
    extra_args: &[String],
    env_path_extra: &[&Path],
) -> Result<()> {
    if extra_args.is_empty() {
        return run_in(cwd, script_name, body, env_path_extra);
    }
    // Quote each arg with shell-safe escaping and append. This matches
    // npm's "run --" convention.
    let mut full = String::with_capacity(body.len() + 4 * extra_args.len());
    full.push_str(body);
    for a in extra_args {
        full.push(' ');
        full.push_str(&shell_quote(a));
    }
    run_in(cwd, script_name, &full, env_path_extra)
}

#[cfg(unix)]
fn build_shell_command(body: &str) -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(body);
    cmd
}

#[cfg(windows)]
fn build_shell_command(body: &str) -> Command {
    let mut cmd = Command::new("cmd");
    cmd.arg("/c").arg(body);
    cmd
}

fn set_path(cmd: &mut Command, extras: &[&Path]) {
    let existing = std::env::var_os("PATH").unwrap_or_default();
    let mut entries: Vec<OsString> = extras.iter().map(|p| p.as_os_str().to_owned()).collect();
    if !existing.is_empty() {
        entries.extend(std::env::split_paths(&existing).map(|p| p.into_os_string()));
    }
    if let Ok(joined) = std::env::join_paths(entries) {
        cmd.env("PATH", joined);
    }
}

#[cfg(unix)]
fn shell_quote(s: &str) -> String {
    if s.chars().all(|c| {
        c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | '=' | ':' | ',' | '@')
    }) {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}

#[cfg(windows)]
fn shell_quote(s: &str) -> String {
    // cmd.exe quoting is a swamp. Use double quotes and escape " and ^.
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '^' | '%' => {
                out.push('^');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}
