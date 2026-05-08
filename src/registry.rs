use crate::error::{GurokuError, Result};
use crate::http_cache;
use crate::npmrc::Npmrc;
use bytes::Bytes;
use serde::Deserialize;
use std::collections::BTreeMap;
use url::Url;

pub const DEFAULT_REGISTRY: &str = "https://registry.npmjs.org";

#[derive(Debug, Clone)]
pub struct RegistryClient {
    base: Url,
    npmrc: Npmrc,
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
            npmrc: Npmrc::default(),
            http,
            use_http_cache: true,
        })
    }

    pub fn with_default_registry() -> Result<Self> {
        Self::new(Url::parse(DEFAULT_REGISTRY).expect("default registry url is valid"))
    }

    /// Build a client from the project + user `.npmrc`. Honours the
    /// `registry`, `<scope>:registry`, and `_authToken` settings.
    pub fn from_npmrc(cwd: &std::path::Path) -> Result<Self> {
        let npmrc = Npmrc::discover(cwd)?;
        let url = Url::parse(npmrc.registry()).map_err(GurokuError::from)?;
        let mut c = Self::new(url)?;
        c.npmrc = npmrc;
        Ok(c)
    }

    pub fn without_http_cache(mut self) -> Self {
        self.use_http_cache = false;
        self
    }

    /// Default registry URL (after `from_npmrc` has resolved any
    /// `registry=` override). Used by `guroku audit`.
    pub fn registry_base(&self) -> &Url {
        &self.base
    }

    /// Decide which registry URL to use for `name`. Scoped names
    /// (`@scope/foo`) consult the npmrc's `<scope>:registry` setting
    /// first; everything else falls through to the default base.
    fn registry_for(&self, name: &str) -> Url {
        if let Some(scope) = scope_of(name) {
            if let Some(url) = self.npmrc.scoped_registry(scope) {
                if let Ok(parsed) = Url::parse(url) {
                    return parsed;
                }
            }
        }
        self.base.clone()
    }

    fn auth_for(&self, url: &Url) -> Option<&str> {
        url.host_str().and_then(|h| self.npmrc.auth_token(h))
    }

    pub async fn fetch_metadata(&self, name: &str) -> Result<PackageMetadata> {
        let base = self.registry_for(name);
        let url = base.join(name)?;

        let cached = if self.use_http_cache {
            http_cache::read(name).ok().flatten()
        } else {
            None
        };

        let mut req = self.http.get(url.clone());
        if let Some(token) = self.auth_for(&url) {
            req = req.bearer_auth(token);
        }
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
        let base = self.registry_for(name);
        let url = base.join(name)?;
        let mut req = self.http.get(url.clone());
        if let Some(token) = self.auth_for(&url) {
            req = req.bearer_auth(token);
        }
        let resp = req.send().await?;
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
        let mut req = self.http.get(url.clone());
        if let Some(token) = self.auth_for(url) {
            req = req.bearer_auth(token);
        }
        let resp = req.send().await?.error_for_status()?;
        Ok(resp.bytes().await?)
    }

    /// Generic JSON POST used by `guroku audit`. Adds bearer auth when an
    /// `_authToken` is configured for the URL's host.
    pub async fn http_post_json(
        &self,
        url: &Url,
        body: Vec<u8>,
    ) -> std::result::Result<reqwest::Response, reqwest::Error> {
        let mut req = self
            .http
            .post(url.clone())
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body);
        if let Some(token) = self.auth_for(url) {
            req = req.bearer_auth(token);
        }
        req.send().await
    }
}

fn parse_metadata_bytes(body: &[u8]) -> Result<PackageMetadata> {
    serde_json::from_slice(body).map_err(GurokuError::from)
}

fn scope_of(name: &str) -> Option<&str> {
    let rest = name.strip_prefix('@')?;
    let (scope, _) = rest.split_once('/')?;
    Some(scope)
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
