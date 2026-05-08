//! Stability tests for the v1.0 `guroku` CLI surface.
//!
//! These tests guard the public CLI shape. A failure here means the binary
//! grew a backwards-incompatible change between v1.0 releases.

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_guroku");

const SUBCOMMANDS: &[&str] = &[
    "install",
    "add",
    "remove",
    "run",
    "exec",
    "workspaces",
    "audit",
];

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(BIN)
        .args(args)
        .output()
        .expect("failed to spawn guroku binary");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    (out.status.success(), stdout, stderr)
}

#[test]
fn help_lists_every_subcommand() {
    let (ok, stdout, stderr) = run(&["--help"]);
    assert!(ok, "`guroku --help` failed; stderr: {stderr}");
    for sub in SUBCOMMANDS {
        assert!(
            stdout.contains(sub),
            "`guroku --help` stdout missing subcommand `{sub}`.\n--- stdout ---\n{stdout}"
        );
    }
}

#[test]
fn version_prints_one_dot_zero() {
    let pkg_version = env!("CARGO_PKG_VERSION");
    assert!(
        pkg_version.starts_with("1."),
        "CARGO_PKG_VERSION `{pkg_version}` is not a 1.x release; v1.0 stability tests do not apply"
    );

    let (ok, stdout, stderr) = run(&["--version"]);
    assert!(ok, "`guroku --version` failed; stderr: {stderr}");
    assert!(
        stdout.contains(pkg_version),
        "`guroku --version` stdout `{stdout}` does not contain crate version `{pkg_version}`"
    );
}

#[test]
fn cwd_global_flag_documented() {
    let (ok, stdout, stderr) = run(&["--help"]);
    assert!(ok, "`guroku --help` failed; stderr: {stderr}");
    assert!(
        stdout.contains("--cwd") || stdout.contains("-C"),
        "`guroku --help` stdout does not document the global -C/--cwd flag.\n--- stdout ---\n{stdout}"
    );
}

#[test]
fn each_subcommand_help_succeeds() {
    for sub in SUBCOMMANDS {
        let (ok, _stdout, stderr) = run(&["help", sub]);
        assert!(
            ok,
            "`guroku help {sub}` did not exit successfully; stderr: {stderr}"
        );
    }
}
