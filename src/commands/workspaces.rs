use crate::error::Result;
use crate::workspaces;
use std::path::Path;

pub async fn run(cwd: &Path) -> Result<()> {
    let found = workspaces::discover(cwd)?;
    if found.is_empty() {
        println!("no workspaces declared in package.json");
        return Ok(());
    }
    println!("found {} workspace package(s):", found.len());
    for ws in &found {
        let name = ws.name().unwrap_or("(unnamed)");
        let version = ws.manifest.version.as_deref().unwrap_or("?");
        let rel = ws
            .root
            .strip_prefix(cwd)
            .unwrap_or(&ws.root)
            .to_string_lossy();
        println!("  {name}@{version}  ({rel})");
    }
    Ok(())
}
