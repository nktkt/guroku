use crate::error::{GurokuError, Result};
use crate::http_cache;
use bytes::Bytes;
use serde::Deserialize;
use std::collections::BTreeMap;
use url::Url;

pub const DEFAULT_REGISTRY: &str = "https://registry.npmjs.org";

#[derive(Debug, Clone)]
pub struct RegistryClient {
    base: Url,
    http: reqwest::Client,
    use_http_cache: bool,
}

impl RegistryClient {
    pub fn new(base: Url) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("guroku/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            base,
            http,
            use_http_cache: true,
        })
    }

    pub fn with_default_registry() -> Result<Self> {
        Self::new(Url::parse(DEFAULT_REGISTRY).expect("default registry url is valid"))
    }

    /// Build a client honouring the `registry=` setting in `<cwd>/.npmrc`
    /// or `~/.npmrc`. Falls back to `with_default_registry` when neither
    /// file is present or sets `registry`.
    pub fn from_npmrc(cwd: &std::path::Path) -> Result<Self> {
        let rc = crate::npmrc::Npmrc::discover(cwd)?;
        let url = Url::parse(rc.registry()).map_err(crate::error::GurokuError::from)?;
        Self::new(url)
    }

    /// Disable the on-disk ETag-aware metadata cache. Useful for tests.
    pub fn without_http_cache(mut self) -> Self {
        self.use_http_cache = false;
        self
    }

    pub async fn fetch_metadata(&self, name: &str) -> Result<PackageMetadata> {
        let url = self.base.join(name)?;

        // v0.3 ETag-aware cache: send If-None-Match if we have one cached.
        let cached = if self.use_http_cache {
            http_cache::read(name).ok().flatten()
        } else {
            None
        };

        let mut req = self.http.get(url);
        if let Some(c) = &cached {
            if let Some(etag) = &c.etag {
                req = req.header(reqwest::header::IF_NONE_MATCH, etag);
            }
        }

        let resp = req.send().await?;
        let status = resp.status();

        if status == reqwest::StatusCode::NOT_MODIFIED {
            if let Some(c) = cached {
                tracing::debug!("metadata cache hit (304) for {name}");
                return parse_metadata_bytes(&c.body);
            }
            // The server says 304 but we have no cached body. Force a refetch.
            return self.fetch_metadata_uncached(name).await;
        }
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(GurokuError::PackageNotFound {
                name: name.to_string(),
            });
        }
        let resp = resp.error_for_status()?;
        let etag = resp
            .headers()
            .get(reqwest::header::ETAG)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let body = resp.bytes().await?;
        let metadata = parse_metadata_bytes(&body)?;
        if self.use_http_cache {
            if let Err(e) = http_cache::write(name, &body, etag.as_deref()) {
                tracing::debug!("failed to write metadata cache for {name}: {e}");
            }
        }
        Ok(metadata)
    }

    async fn fetch_metadata_uncached(&self, name: &str) -> Result<PackageMetadata> {
        let url = self.base.join(name)?;
        let resp = self.http.get(url).send().await?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(GurokuError::PackageNotFound {
                name: name.to_string(),
            });
        }
        let resp = resp.error_for_status()?;
        let body = resp.bytes().await?;
        parse_metadata_bytes(&body)
    }

    pub async fn fetch_tarball(&self, url: &Url) -> Result<Bytes> {
        let resp = self
            .http
            .get(url.clone())
            .send()
            .await?
            .error_for_status()?;
        Ok(resp.bytes().await?)
    }
}

fn parse_metadata_bytes(body: &[u8]) -> Result<PackageMetadata> {
    serde_json::from_slice(body).map_err(GurokuError::from)
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
    /// Resolve a version spec against the metadata.
    ///
    /// Order of operations:
    ///   1. Exact version match in `versions`.
    ///   2. dist-tag lookup (`latest`, `next`, ...).
    ///   3. Parse as an npm semver range and pick the highest matching
    ///      version (this is what handles `^1.2.3`, `~1.0`, `>=1 <2`, etc.).
    ///
    /// Returns `NoMatchingVersion` if nothing matches.
    pub fn resolve(&self, spec: &str) -> Result<&VersionInfo> {
        if let Some(v) = self.versions.get(spec) {
            return Ok(v);
        }
        if let Some(target) = self.dist_tags.get(spec) {
            if let Some(v) = self.versions.get(target) {
                return Ok(v);
            }
        }
        if let Ok(range) = crate::version::parse_range(spec) {
            let candidates = self
                .versions
                .keys()
                .filter(|k| crate::version::parse_version(k).is_ok())
                .map(String::as_str);
            if let Some(picked) = crate::version::max_satisfying(candidates, &range) {
                let key = picked.to_string();
                if let Some(v) = self.versions.get(&key) {
                    return Ok(v);
                }
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
