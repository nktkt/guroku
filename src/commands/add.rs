use crate::error::Result;
use crate::lockfile::LOCKFILE_NAME;
use crate::manifest::Manifest;
use crate::registry::RegistryClient;
use crate::resolver;
use std::path::Path;

pub async fn run(cwd: &Path, packages: &[String]) -> Result<()> {
    let manifest_path = cwd.join("package.json");
    let mut manifest = if manifest_path.exists() {
        Manifest::read_from(&manifest_path)?
    } else {
        Manifest::default()
    };
    let node_modules = cwd.join("node_modules");
    let lock_path = cwd.join(LOCKFILE_NAME);
    let client = RegistryClient::with_default_registry()?;

    // Stage the new specs onto the manifest first so the resolver sees them
    // alongside the existing dependencies.
    let mut new_entries: Vec<(String, String)> = Vec::with_capacity(packages.len());
    for spec_input in packages {
        let (name, spec) = super::parse_spec(spec_input);
        let spec_for_manifest = if spec == "latest" {
            "latest".to_string()
        } else {
            spec.clone()
        };
        new_entries.push((name.clone(), spec_for_manifest));
        manifest.add_dependency(&name, &spec);
    }

    let roots: Vec<(String, String)> = manifest
        .all_dependencies()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    let resolution = resolver::resolve(&client, &roots).await?;
    super::install::install_from_resolution(&client, &resolution, &node_modules).await?;

    // Replace each just-added spec with the caret-pinned resolved version,
    // matching npm's "guess the user wanted ^X" behaviour.
    for (name, _) in &new_entries {
        if let Some(r) = resolution.packages.get(name) {
            manifest.add_dependency(name, &format!("^{}", r.info.version));
        }
    }
    manifest.write_to(&manifest_path)?;
    super::install::write_lockfile(&resolution, &lock_path)?;

    Ok(())
}
