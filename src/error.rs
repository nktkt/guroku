use std::io;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GurokuError {
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("io error: {0}")]
    IoBare(#[from] io::Error),

    #[error("failed to parse {path}: {source}")]
    ParseManifest {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("invalid json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("invalid url: {0}")]
    Url(#[from] url::ParseError),

    #[error("package `{name}` not found in registry")]
    PackageNotFound { name: String },

    #[error("no matching version for `{name}@{spec}`")]
    NoMatchingVersion { name: String, spec: String },

    #[error("invalid version spec `{spec}` for package `{name}`")]
    InvalidVersionSpec { name: String, spec: String },

    #[error("integrity check failed for `{name}@{version}`: {detail}")]
    IntegrityMismatch {
        name: String,
        version: String,
        detail: String,
    },

    #[error("unsupported integrity algorithm: `{0}`")]
    UnsupportedIntegrity(String),

    #[error("invalid integrity string: `{0}`")]
    InvalidIntegrity(String),

    #[error("tarball error: {0}")]
    Tarball(String),

    #[error("could not determine cache directory")]
    NoCacheDir,

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, GurokuError>;
