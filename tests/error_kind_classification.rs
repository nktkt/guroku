//! Pins the v1.0 error-variant taxonomy.
//!
//! If a future PR removes a variant, this file fails to compile (the explicit
//! arm in `kind()` references a name that no longer exists). If a future PR
//! adds a variant, the wildcard `_ => "unknown"` arm catches it, so additions
//! are non-breaking.
//!
//! `GurokuError` is `#[non_exhaustive]`, so every external match must include
//! a `_` arm — `non_exhaustive_match_compiles` documents that contract.

use guroku::error::GurokuError;
use std::path::PathBuf;

fn kind(e: &GurokuError) -> &'static str {
    match e {
        GurokuError::Io { .. } => "io",
        GurokuError::IoBare(_) => "io_bare",
        GurokuError::ParseManifest { .. } => "parse_manifest",
        GurokuError::Json(_) => "json",
        GurokuError::Http(_) => "http",
        GurokuError::Url(_) => "url",
        GurokuError::PackageNotFound { .. } => "package_not_found",
        GurokuError::NoMatchingVersion { .. } => "no_matching_version",
        GurokuError::InvalidVersionSpec { .. } => "invalid_version_spec",
        GurokuError::IntegrityMismatch { .. } => "integrity_mismatch",
        GurokuError::UnsupportedIntegrity(_) => "unsupported_integrity",
        GurokuError::InvalidIntegrity(_) => "invalid_integrity",
        GurokuError::Tarball(_) => "tarball",
        GurokuError::NoCacheDir => "no_cache_dir",
        GurokuError::ResolutionConflict { .. } => "resolution_conflict",
        GurokuError::LockfileVersionMismatch { .. } => "lockfile_version_mismatch",
        GurokuError::LockfileOutOfDate => "lockfile_out_of_date",
        GurokuError::ScriptFailed { .. } => "script_failed",
        GurokuError::ScriptSpawnFailed { .. } => "script_spawn_failed",
        GurokuError::NoSuchScript { .. } => "no_such_script",
        GurokuError::WorkspaceMisconfigured(_) => "workspace_misconfigured",
        GurokuError::BinNotFound { .. } => "bin_not_found",
        GurokuError::FileDepMissingManifest { .. } => "file_dep_missing_manifest",
        GurokuError::GitCommandFailed { .. } => "git_command_failed",
        GurokuError::AuditFailed(_) => "audit_failed",
        GurokuError::InvalidOverride { .. } => "invalid_override",
        GurokuError::Other(_) => "other",
        // Future-added variants land here. Adding a variant is non-breaking;
        // removing one breaks an explicit arm above (intentional).
        _ => "unknown",
    }
}

#[test]
fn every_known_variant_classifiable() {
    // Skipped (no easy synthesis without I/O / network):
    //   IoBare(io::Error)        — needs a real io::Error
    //   Json(serde_json::Error)  — needs a real parse failure
    //   Http(reqwest::Error)     — reqwest::Error has no public ctor
    //   ParseManifest{source}    — same story as Json
    // These are still pinned by the `kind()` match above; if they're removed,
    // the file stops compiling.
    let cases: Vec<(GurokuError, &'static str)> = vec![
        (
            GurokuError::Io {
                path: PathBuf::from("/x"),
                source: std::io::Error::other("e"),
            },
            "io",
        ),
        (GurokuError::Url(url::ParseError::EmptyHost), "url"),
        (
            GurokuError::PackageNotFound {
                name: "lodash".into(),
            },
            "package_not_found",
        ),
        (
            GurokuError::NoMatchingVersion {
                name: "a".into(),
                spec: "^9".into(),
            },
            "no_matching_version",
        ),
        (
            GurokuError::InvalidVersionSpec {
                name: "a".into(),
                spec: "??".into(),
            },
            "invalid_version_spec",
        ),
        (
            GurokuError::IntegrityMismatch {
                name: "a".into(),
                version: "1.0.0".into(),
                detail: "d".into(),
            },
            "integrity_mismatch",
        ),
        (
            GurokuError::UnsupportedIntegrity("sha1".into()),
            "unsupported_integrity",
        ),
        (
            GurokuError::InvalidIntegrity("xyz".into()),
            "invalid_integrity",
        ),
        (GurokuError::Tarball("bad header".into()), "tarball"),
        (GurokuError::NoCacheDir, "no_cache_dir"),
        (
            GurokuError::ResolutionConflict {
                name: "a".into(),
                chosen: "1.0.0".into(),
                requested: "2.0.0".into(),
                requested_by: "b".into(),
            },
            "resolution_conflict",
        ),
        (
            GurokuError::LockfileVersionMismatch {
                found: 2,
                expected: 1,
            },
            "lockfile_version_mismatch",
        ),
        (GurokuError::LockfileOutOfDate, "lockfile_out_of_date"),
        (
            GurokuError::ScriptFailed {
                script: "build".into(),
                status: 1,
            },
            "script_failed",
        ),
        (
            GurokuError::ScriptSpawnFailed {
                script: "build".into(),
                detail: "ENOENT".into(),
            },
            "script_spawn_failed",
        ),
        (
            GurokuError::NoSuchScript {
                name: "build".into(),
            },
            "no_such_script",
        ),
        (
            GurokuError::WorkspaceMisconfigured("loop".into()),
            "workspace_misconfigured",
        ),
        (
            GurokuError::BinNotFound { name: "tsc".into() },
            "bin_not_found",
        ),
        (
            GurokuError::FileDepMissingManifest {
                path: "../x".into(),
            },
            "file_dep_missing_manifest",
        ),
        (
            GurokuError::GitCommandFailed {
                url: "u".into(),
                detail: "d".into(),
            },
            "git_command_failed",
        ),
        (GurokuError::AuditFailed("503".into()), "audit_failed"),
        (
            GurokuError::InvalidOverride {
                name: "a".into(),
                detail: "d".into(),
            },
            "invalid_override",
        ),
        (GurokuError::Other("misc".into()), "other"),
    ];
    for (err, want) in &cases {
        assert_eq!(kind(err), *want, "wrong kind for {err:?}");
    }
}

#[test]
fn non_exhaustive_match_compiles() {
    // The wildcard `_` arm is the whole point: future variants must not
    // break downstream pattern matches. This test passes by compiling.
    let e = GurokuError::Other("x".into());
    let label = match &e {
        GurokuError::Other(_) => "other",
        _ => "future",
    };
    assert_eq!(label, "other");
    assert!(matches!(&e, GurokuError::Other(_)));
}

#[test]
fn display_strings_are_non_empty() {
    let samples: [GurokuError; 10] = [
        GurokuError::PackageNotFound { name: "a".into() },
        GurokuError::NoMatchingVersion {
            name: "a".into(),
            spec: "^1".into(),
        },
        GurokuError::InvalidVersionSpec {
            name: "a".into(),
            spec: "?".into(),
        },
        GurokuError::UnsupportedIntegrity("sha1".into()),
        GurokuError::InvalidIntegrity("zzz".into()),
        GurokuError::Tarball("e".into()),
        GurokuError::NoCacheDir,
        GurokuError::LockfileOutOfDate,
        GurokuError::NoSuchScript {
            name: "build".into(),
        },
        GurokuError::Other("x".into()),
    ];
    for e in &samples {
        assert!(!format!("{}", e).is_empty(), "empty Display for {e:?}");
    }
}

#[test]
fn error_implements_std_error() {
    // Compile-only: the coercion fails to type-check unless GurokuError: std::error::Error.
    let e = GurokuError::Other("x".into());
    let _: &dyn std::error::Error = &e;
}

#[test]
fn result_alias_works() {
    let r: guroku::Result<()> = Err(GurokuError::Other("x".into()));
    assert!(r.is_err());
}
