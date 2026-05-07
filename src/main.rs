use clap::Parser;
use guroku::cli::{Cli, Command};
use guroku::commands;
use guroku::logging;

#[tokio::main]
async fn main() {
    logging::init();
    let cli = Cli::parse();
    let cwd = match cli.cwd_or_current() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("guroku: failed to determine working directory: {e}");
            std::process::exit(2);
        }
    };

    let result = match cli.command.unwrap_or(Command::Install) {
        Command::Install => commands::install::run(&cwd).await,
        Command::Add { packages } => commands::add::run(&cwd, &packages).await,
        Command::Remove { packages } => commands::remove::run(&cwd, &packages).await,
    };

    if let Err(e) = result {
        eprintln!("guroku: {e}");
        std::process::exit(1);
    }
}
