//! npm audit — query the public advisories API and report vulnerabilities.
//!
//! v0.5 uses the bulk endpoint at
//! `<registry>/-/npm/v1/security/advisories/bulk`. The request body is a
//! JSON map from package name to a list of installed versions; the
//! response is a map from package name to a list of advisories.

use crate::error::{GurokuError, Result};
use crate::lockfile::Lockfile;
use crate::registry::RegistryClient;
use serde::Deserialize;
use std::collections::BTreeMap;

/// One vulnerability advisory, as returned by the npm registry.
#[derive(Debug, Clone, Deserialize)]
pub struct Advisory {
    #[serde(default)]
    pub id: serde_json::Value,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub severity: String,
    #[serde(default)]
    pub url: String,
    #[serde(default, rename = "vulnerable_versions")]
    pub vulnerable_versions: String,
    #[serde(default, rename = "patched_versions")]
    pub patched_versions: String,
}

#[derive(Debug, Default, Clone)]
pub struct AuditReport {
    pub findings: BTreeMap<String, Vec<Advisory>>,
}

impl AuditReport {
    pub fn is_empty(&self) -> bool {
        self.findings.is_empty()
    }

    pub fn count(&self) -> usize {
        self.findings.values().map(|v| v.len()).sum()
    }
}

/// Audit every package present in `lockfile` against the registry's
/// advisory database. Returns a map of `name → [Advisory; ..]` keyed by
/// the names that came back vulnerable.
pub async fn audit(client: &RegistryClient, lockfile: &Lockfile) -> Result<AuditReport> {
    let mut by_name: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for key in lockfile.packages.keys() {
        let Some((name, version)) = key.rsplit_once('@') else {
            continue;
        };
        by_name
            .entry(name.to_string())
            .or_default()
            .push(version.to_string());
    }
    if by_name.is_empty() {
        return Ok(AuditReport::default());
    }

    let body = serde_json::to_vec(&by_name)?;
    let url = client
        .registry_base()
        .join("-/npm/v1/security/advisories/bulk")
        .map_err(GurokuError::from)?;

    let resp = client
        .http_post_json(&url, body)
        .await
        .map_err(|e| GurokuError::AuditFailed(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(GurokuError::AuditFailed(format!(
            "HTTP {} from advisories endpoint",
            resp.status()
        )));
    }
    let parsed: BTreeMap<String, Vec<Advisory>> = resp
        .json()
        .await
        .map_err(|e| GurokuError::AuditFailed(e.to_string()))?;

    Ok(AuditReport { findings: parsed })
}

/// Render an audit report to stdout. Prints "no known vulnerabilities" if
/// empty.
pub fn print_report(report: &AuditReport) {
    if report.is_empty() {
        println!("no known vulnerabilities");
        return;
    }
    println!(
        "found {} advisor{} across {} package(s):",
        report.count(),
        if report.count() == 1 { "y" } else { "ies" },
        report.findings.len()
    );
    for (name, advisories) in &report.findings {
        for a in advisories {
            println!("  [{}] {}@{}", a.severity, name, a.vulnerable_versions);
            if !a.title.is_empty() {
                println!("        {}", a.title);
            }
            if !a.url.is_empty() {
                println!("        {}", a.url);
            }
            if !a.patched_versions.is_empty() {
                println!("        patched: {}", a.patched_versions);
            }
        }
    }
}
