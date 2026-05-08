//! Embedding example for guroku 1.0.
//!
//! Run from a directory containing a `package.json`:
//!
//! ```sh
//! cd examples/embedding-rust
//! cargo run
//! ```
//!
//! The default cwd has no package.json; pass one via `cargo run -- <path>`.
//!
//! What this demonstrates:
//! - Reading a manifest.
//! - Building a registry client honouring `.npmrc`.
//! - Calling the resolver and walking the result.
//!
//! Real installs would also call `commands::install::install_from_resolution`
//! to actually download + link; this example stops at resolution to keep
//! the network footprint minimal.

use guroku::prelude::*;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("GUROKU_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .without_time()
        .init();

    let cwd: PathBuf = match std::env::args().nth(1) {
        Some(p) => PathBuf::from(p),
        None => std::env::current_dir()?,
    };
    println!("project root: {}", cwd.display());

    // 1. Manifest.
    let manifest_path = cwd.join("package.json");
    let manifest = Manifest::read_from(&manifest_path)?;
    let project_name = manifest.name.as_deref().unwrap_or("(unnamed)");
    println!("project: {}", project_name);

    // 2. Registry client (honours <cwd>/.npmrc and ~/.npmrc).
    let client = RegistryClient::from_npmrc(&cwd)?;

    // 3. Roots from the manifest.
    let roots: Vec<(String, String)> = manifest
        .all_dependencies()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    if roots.is_empty() {
        println!("no dependencies declared.");
        return Ok(());
    }
    println!("declared {} root packages", roots.len());

    // 4. Resolve.
    let resolution = guroku::resolver::resolve(&client, &roots).await?;
    println!("resolved {} packages total:", resolution.len());
    for (name, r) in resolution.iter() {
        let suffix = if r.local_source.is_some() {
            " [local]"
        } else {
            ""
        };
        println!("  {}@{}{}", name, r.info.version, suffix);
    }

    Ok(())
}
