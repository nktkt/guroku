use crate::error::{GurokuError, Result};
use crate::manifest::Manifest;
use crate::scripts;
use std::path::Path;

pub async fn run(cwd: &Path, name: Option<String>, args: &[String]) -> Result<()> {
    let manifest = Manifest::read_from(&cwd.join("package.json"))?;
    let bin_dir = cwd.join("node_modules").join(".bin");

    match name {
        None => {
            // List available scripts.
            if manifest.scripts.is_empty() {
                println!("no scripts defined in package.json");
            } else {
                println!("available scripts:");
                for (k, v) in &manifest.scripts {
                    println!("  {k}\n    {v}");
                }
            }
            Ok(())
        }
        Some(script_name) => {
            let body = manifest
                .scripts
                .get(&script_name)
                .ok_or_else(|| GurokuError::NoSuchScript {
                    name: script_name.clone(),
                })?
                .clone();
            scripts::run_in_with_args(cwd, &script_name, &body, args, &[bin_dir.as_path()])
        }
    }
}
