use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "guroku",
    version,
    about = "A fast, Rust-powered package manager for the JavaScript ecosystem.",
    long_about = None,
    arg_required_else_help = false,
)]
pub struct Cli {
    /// Project directory (defaults to the current working directory).
    #[arg(long, short = 'C', global = true)]
    pub cwd: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Install all dependencies declared in `package.json`.
    #[command(alias = "i")]
    Install {
        /// Refuse to refresh the lockfile; fail if `guroku.lock` is out of
        /// date with `package.json`.
        #[arg(long)]
        frozen_lockfile: bool,

        /// Skip lifecycle scripts (`preinstall`, `postinstall`, etc.).
        #[arg(long)]
        ignore_scripts: bool,
    },

    /// Add one or more packages to `dependencies` and install them.
    Add {
        /// Package specifiers, e.g. `lodash` or `lodash@4.17.21`.
        #[arg(required = true)]
        packages: Vec<String>,
    },

    /// Remove one or more packages from `package.json` and `node_modules`.
    #[command(alias = "rm")]
    Remove {
        #[arg(required = true)]
        packages: Vec<String>,
    },

    /// Run a script defined in `package.json#scripts`.
    Run {
        /// Script name. With no name, lists available scripts.
        name: Option<String>,
        /// Trailing args forwarded to the script after `--`.
        #[arg(last = true)]
        args: Vec<String>,
    },

    /// Execute a binary that's already on PATH or under `node_modules/.bin/`.
    Exec {
        /// Command to run.
        #[arg(required = true)]
        command: String,
        /// Args forwarded to the command.
        args: Vec<String>,
    },

    /// List discovered workspace packages.
    Workspaces,

    /// Query the registry's advisory database for known vulnerabilities
    /// in `guroku.lock`.
    Audit,
}

impl Cli {
    pub fn cwd_or_current(&self) -> std::io::Result<PathBuf> {
        match &self.cwd {
            Some(p) => Ok(p.clone()),
            None => std::env::current_dir(),
        }
    }
}
