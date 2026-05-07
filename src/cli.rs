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
    Install,

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
}

impl Cli {
    pub fn cwd_or_current(&self) -> std::io::Result<PathBuf> {
        match &self.cwd {
            Some(p) => Ok(p.clone()),
            None => std::env::current_dir(),
        }
    }
}
