use crate::error::{GurokuError, Result};
use crate::manifest::Manifest;
use crate::registry::RegistryClient;
use futures::stream::{self, StreamExt};
use std::path::Path;

const CONCURRENCY: usize = 8;

pub async fn run(cwd: &Path) -> Result<()> {
    let manifest_path = cwd.join("package.json");
    let manifest = Manifest::read_from(&manifest_path)?;
    let node_modules = cwd.join("node_modules");

    let client = RegistryClient::default()?;
    let deps: Vec<(String, String)> = manifest
        .all_dependencies()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    if deps.is_empty() {
        tracing::info!("no dependencies declared in {}", manifest_path.display());
        return Ok(());
    }

    tracing::info!("installing {} packages", deps.len());

    let results: Vec<Result<()>> = stream::iter(deps.into_iter())
        .map(|(name, spec)| {
            let client = client.clone();
            let node_modules = node_modules.clone();
            async move {
                super::install_one(&client, &name, &spec, &node_modules)
                    .await
                    .map(|_| ())
            }
        })
        .buffer_unordered(CONCURRENCY)
        .collect()
        .await;

    let mut failures = Vec::new();
    for r in results {
        if let Err(e) = r {
            failures.push(e.to_string());
        }
    }
    if !failures.is_empty() {
        return Err(GurokuError::Other(format!(
            "{} package(s) failed to install: {}",
            failures.len(),
            failures.join("; ")
        )));
    }

    tracing::info!("done");
    Ok(())
}
