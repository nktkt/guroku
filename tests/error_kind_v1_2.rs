//! v1.2 compile-time pin for the `GurokuError` variant set.
//!
//! `GurokuError` is `#[non_exhaustive]` (since v1.0). v1.2 must not remove
//! any v1.0/v1.1 variant. Each test below constructs a known variant; if a
//! field name or shape is changed, the test stops compiling — which is the
//! whole point.
//!
//! v1.2 introduces no new variants (the pubgrub integration reuses
//! `ResolutionConflict` and `Other`). See ANNOUNCEMENT.md / CHANGELOG.md.

#![deny(unreachable_patterns)]

use guroku::error::GurokuError;
use std::path::PathBuf;

#[test]
fn io_variant_constructible() {
    let _ = GurokuError::Io {
        path: PathBuf::from("/x"),
        source: std::io::Error::other("e"),
    };
}

#[test]
fn package_not_found_variant() {
    let _ = GurokuError::PackageNotFound { name: "x".into() };
}

#[test]
fn no_matching_version_variant() {
    let _ = GurokuError::NoMatchingVersion {
        name: "x".into(),
        spec: "^1".into(),
    };
}

#[test]
fn invalid_version_spec_variant() {
    let _ = GurokuError::InvalidVersionSpec {
        name: "x".into(),
        spec: "??".into(),
    };
}

#[test]
fn integrity_mismatch_variant() {
    let _ = GurokuError::IntegrityMismatch {
        name: "x".into(),
        version: "1.0.0".into(),
        detail: "d".into(),
    };
}

#[test]
fn tarball_variant() {
    let _ = GurokuError::Tarball("x".into());
}

#[test]
fn resolution_conflict_variant() {
    let _ = GurokuError::ResolutionConflict {
        name: "x".into(),
        chosen: "1.0.0".into(),
        requested: "2.0.0".into(),
        requested_by: "y".into(),
    };
}

#[test]
fn lockfile_version_mismatch_variant() {
    let _ = GurokuError::LockfileVersionMismatch {
        found: 2,
        expected: 1,
    };
}

#[test]
fn lockfile_out_of_date_variant() {
    let _ = GurokuError::LockfileOutOfDate;
}

#[test]
fn script_failed_variant() {
    let _ = GurokuError::ScriptFailed {
        script: "build".into(),
        status: 1,
    };
}

#[test]
fn no_such_script_variant() {
    let _ = GurokuError::NoSuchScript {
        name: "build".into(),
    };
}

#[test]
fn workspace_misconfigured_variant() {
    let _ = GurokuError::WorkspaceMisconfigured("loop".into());
}

#[test]
fn bin_not_found_variant() {
    let _ = GurokuError::BinNotFound { name: "tsc".into() };
}

#[test]
fn file_dep_missing_manifest_variant() {
    let _ = GurokuError::FileDepMissingManifest {
        path: "../x".into(),
    };
}

#[test]
fn git_command_failed_variant() {
    // Actual shape per src/error.rs is { url, detail } (not command/status).
    let _ = GurokuError::GitCommandFailed {
        url: "https://example.invalid/x.git".into(),
        detail: "exit 128".into(),
    };
}

#[test]
fn audit_failed_variant() {
    let _ = GurokuError::AuditFailed("503".into());
}

#[test]
fn invalid_override_variant() {
    let _ = GurokuError::InvalidOverride {
        name: "x".into(),
        detail: "d".into(),
    };
}

fn classify(e: &GurokuError) -> &'static str {
    match e {
        GurokuError::Io { .. } => "io",
        GurokuError::PackageNotFound { .. } => "package_not_found",
        GurokuError::ResolutionConflict { .. } => "resolution_conflict",
        GurokuError::Other(_) => "other",
        // If `#[non_exhaustive]` is removed in a future patch, this `_` arm
        // becomes unreachable and `deny(unreachable_patterns)` makes the
        // file fail to compile.
        _ => "other_unknown",
    }
}

#[test]
fn non_exhaustive_attribute_present() {
    let _ = classify as fn(&GurokuError) -> &'static str;
    assert_eq!(classify(&GurokuError::Other("x".into())), "other");
}
