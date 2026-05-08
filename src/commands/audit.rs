use crate::audit;
use crate::error::{GurokuError, Result};
use crate::lockfile::{Lockfile, LOCKFILE_NAME};
use crate::registry::RegistryClient;
use std::path::Path;

pub async fn run(cwd: &Path) -> Result<()> {
    let lock_path = cwd.join(LOCKFILE_NAME);
    if !lock_path.exists() {
        return Err(GurokuError::Other(
            "guroku.lock not found — run `guroku install` first".into(),
        ));
    }
    let lock = Lockfile::read_from(&lock_path)?;
    let client = RegistryClient::from_npmrc(cwd)?;
    let report = audit::audit(&client, &lock).await?;
    audit::print_report(&report);
    if report.is_empty() {
        Ok(())
    } else {
        Err(GurokuError::Other(format!(
            "{} known vulnerabilit{} reported",
            report.count(),
            if report.count() == 1 { "y" } else { "ies" }
        )))
    }
}
