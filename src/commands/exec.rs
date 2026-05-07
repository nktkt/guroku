use crate::error::{GurokuError, Result};
use std::path::Path;
use std::process::Command;

pub async fn run(cwd: &Path, command: &str, args: &[String]) -> Result<()> {
    let bin_dir = cwd.join("node_modules").join(".bin");
    let candidate = bin_dir.join(command);

    let mut cmd = if candidate.exists() {
        let mut c = Command::new(&candidate);
        c.args(args);
        c
    } else {
        // Fall back to PATH lookup. We *don't* prepend bin_dir to PATH for
        // exec-on-PATH invocations because the user clearly meant a system
        // tool (otherwise they'd have used `guroku run` or installed it).
        let mut c = Command::new(command);
        c.args(args);
        c
    };
    cmd.current_dir(cwd);
    // Always make node_modules/.bin available to the spawned process's
    // children, since most JS CLIs shell out to siblings.
    if bin_dir.is_dir() {
        let existing = std::env::var_os("PATH").unwrap_or_default();
        let mut entries = vec![bin_dir.clone().into_os_string()];
        entries.extend(std::env::split_paths(&existing).map(|p| p.into_os_string()));
        if let Ok(joined) = std::env::join_paths(entries) {
            cmd.env("PATH", joined);
        }
    }

    match cmd.status() {
        Ok(status) => {
            if status.success() {
                Ok(())
            } else {
                Err(GurokuError::ScriptFailed {
                    script: command.to_string(),
                    status: status.code().unwrap_or(-1),
                })
            }
        }
        Err(e) => {
            // ENOENT: the command isn't on PATH and isn't in .bin/.
            if e.kind() == std::io::ErrorKind::NotFound {
                Err(GurokuError::BinNotFound {
                    name: command.to_string(),
                })
            } else {
                Err(GurokuError::ScriptSpawnFailed {
                    script: command.to_string(),
                    detail: e.to_string(),
                })
            }
        }
    }
}
