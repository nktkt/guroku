use crate::error::{GurokuError, Result};
use bytes::Bytes;
use serde::Deserialize;
use std::collections::BTreeMap;
use url::Url;

pub const DEFAULT_REGISTRY: &str = "https://registry.npmjs.org";

#[derive(Debug, Clone)]
pub struct RegistryClient {
    base: Url,
    http: reqwest::Client,
}

impl RegistryClient {
    pub fn new(base: Url) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("guroku/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { base, http })
    }

    pub fn default() -> Result<Self> {
        Self::new(Url::parse(DEFAULT_REGISTRY).expect("default registry url is valid"))
    }

    pub async fn fetch_metadata(&self, name: &str) -> Result<PackageMetadata> {
        let url = self.base.join(name)?;
        let resp = self.http.get(url).send().await?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(GurokuError::PackageNotFound {
                name: name.to_string(),
            });
        }
        let resp = resp.error_for_status()?;
        let metadata: PackageMetadata = resp.json().await?;
        Ok(metadata)
    }

    pub async fn fetch_tarball(&self, url: &Url) -> Result<Bytes> {
        let resp = self.http.get(url.clone()).send().await?.error_for_status()?;
        Ok(resp.bytes().await?)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PackageMetadata {
    pub name: String,
    #[serde(default)]
    pub versions: BTreeMap<String, VersionInfo>,
    #[serde(default, rename = "dist-tags")]
    pub dist_tags: BTreeMap<String, String>,
}

impl PackageMetadata {
    /// Resolve a version spec against the metadata. v0.1 supports:
    ///   - exact version: "1.2.3"
    ///   - dist-tag: "latest", "next"
    ///   - "*" or empty → latest
    /// Anything else falls back to "latest" (resolution is the v0.2 milestone).
    pub fn resolve(&self, spec: &str) -> Result<&VersionInfo> {
        if let Some(v) = self.versions.get(spec) {
            return Ok(v);
        }
        if let Some(target) = self.dist_tags.get(spec) {
            if let Some(v) = self.versions.get(target) {
                return Ok(v);
            }
        }
        if spec.is_empty() || spec == "*" || spec == "latest" {
            if let Some(target) = self.dist_tags.get("latest") {
                if let Some(v) = self.versions.get(target) {
                    return Ok(v);
                }
            }
        }
        // Best-effort fallback: use latest dist-tag if present.
        if let Some(target) = self.dist_tags.get("latest") {
            if let Some(v) = self.versions.get(target) {
                tracing::warn!(
                    "version spec `{spec}` for `{}` not understood by v0.1; falling back to {target}",
                    self.name
                );
                return Ok(v);
            }
        }
        Err(GurokuError::NoMatchingVersion {
            name: self.name.clone(),
            spec: spec.to_string(),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionInfo {
    pub name: String,
    pub version: String,
    pub dist: Dist,
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Dist {
    pub tarball: Url,
    #[serde(default)]
    pub integrity: Option<String>,
    #[serde(default)]
    pub shasum: Option<String>,
}
