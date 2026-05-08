use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// The crate-wide error type.
///
/// Every fallible operation in guroku returns `Result<T, GurokuError>`
/// (re-exported as [`crate::Result`]).
///
/// `#[non_exhaustive]` from v1.0 forward: new variants may be added in
/// minor releases. Match with a `_` arm in code that isn't part of the
/// guroku source tree.
#[derive(Debug, Error)]
#[non_exhaustive]
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

    #[error(
        "version conflict for `{name}`: already chose `{chosen}`, but `{requested_by}` requires `{requested}`"
    )]
    ResolutionConflict {
        name: String,
        chosen: String,
        requested: String,
        requested_by: String,
    },

    #[error("lockfile version mismatch: file is v{found}, this guroku understands v{expected}")]
    LockfileVersionMismatch { found: u32, expected: u32 },

    #[error(
        "lockfile is out of date with `package.json` (run `guroku install` without --frozen-lockfile to refresh)"
    )]
    LockfileOutOfDate,

    #[error("script `{script}` exited with status {status}")]
    ScriptFailed { script: String, status: i32 },

    #[error("failed to spawn script `{script}`: {detail}")]
    ScriptSpawnFailed { script: String, detail: String },

    #[error("no `{name}` script in package.json#scripts")]
    NoSuchScript { name: String },

    #[error("workspaces misconfigured: {0}")]
    WorkspaceMisconfigured(String),

    #[error("`{name}` is not on PATH and was not found in node_modules/.bin")]
    BinNotFound { name: String },

    #[error("file dependency at `{path}` has no readable package.json")]
    FileDepMissingManifest { path: String },

    #[error("git command failed for `{url}`: {detail}")]
    GitCommandFailed { url: String, detail: String },

    #[error("audit request failed: {0}")]
    AuditFailed(String),

    #[error("invalid override entry for `{name}`: {detail}")]
    InvalidOverride { name: String, detail: String },

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, GurokuError>;
