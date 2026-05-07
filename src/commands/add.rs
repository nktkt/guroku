use crate::error::Result;
use crate::manifest::Manifest;
use crate::registry::RegistryClient;
use std::path::Path;

pub async fn run(cwd: &Path, packages: &[String]) -> Result<()> {
    let manifest_path = cwd.join("package.json");
    let mut manifest = if manifest_path.exists() {
        Manifest::read_from(&manifest_path)?
    } else {
        Manifest::default()
    };
    let node_modules = cwd.join("node_modules");
    let client = RegistryClient::with_default_registry()?;

    for spec_input in packages {
        let (name, spec) = super::parse_spec(spec_input);
        tracing::info!("adding {}@{}", name, spec);
        let installed = super::install_one(&client, &name, &spec, &node_modules).await?;
        manifest.add_dependency(&name, &format!("^{installed}"));
    }

    manifest.write_to(&manifest_path)?;
    Ok(())
}
