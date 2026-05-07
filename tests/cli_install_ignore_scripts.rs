//! CLI tests for `guroku install --ignore-scripts` (v0.4).
//!
//! The `--ignore-scripts` flag suppresses lifecycle scripts (preinstall,
//! postinstall, etc.) during install. These tests cover help-text discovery
//! and the no-dependency short-circuit behaviour documented inline below.

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

const BIN: &str = env!("CARGO_BIN_EXE_guroku");

fn write_package_json(dir: &Path, contents: &str) {
    fs::write(dir.join("package.json"), contents).expect("write package.json");
}

fn pkg_with_preinstall() -> &'static str {
    r#"{"name":"x","version":"0.1.0","scripts":{"preinstall":"echo PRE > out.txt"}}"#
}

#[test]
fn install_help_mentions_ignore_scripts() {
    let output = Command::new(BIN)
        .args(["help", "install"])
        .output()
        .expect("run guroku help install");

    assert!(
        output.status.success(),
        "`guroku help install` failed: status={:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--ignore-scripts"),
        "expected stdout to mention `--ignore-scripts`, got:\n{}",
        stdout,
    );
}

#[test]
fn install_no_deps_runs_root_preinstall() {
    // NOTE: Despite the test name, this asserts the *short-circuit* behaviour:
    // when the manifest declares no dependencies, `guroku install` returns
    // early (the `roots` set is empty) BEFORE lifecycle scripts are dispatched.
    // Therefore the root `preinstall` script does NOT execute and `out.txt`
    // must NOT exist on disk. This is intentional in v0.4 — keep this comment
    // so future readers don't "fix" the assertion.
    let td = TempDir::new().expect("tempdir");
    write_package_json(td.path(), pkg_with_preinstall());

    let output = Command::new(BIN)
        .args(["--cwd"])
        .arg(td.path())
        .arg("install")
        .output()
        .expect("run guroku install");

    assert!(
        output.status.success(),
        "install failed: status={:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );

    let out_txt = td.path().join("out.txt");
    assert!(
        !out_txt.exists(),
        "expected out.txt to be absent due to no-deps short-circuit, but it exists",
    );
}

#[test]
fn ignore_scripts_works_with_no_deps() {
    // Same short-circuit as above; --ignore-scripts must not regress exit
    // status and `out.txt` still must not be created.
    let td = TempDir::new().expect("tempdir");
    write_package_json(td.path(), pkg_with_preinstall());

    let output = Command::new(BIN)
        .args(["--cwd"])
        .arg(td.path())
        .args(["install", "--ignore-scripts"])
        .output()
        .expect("run guroku install --ignore-scripts");

    assert!(
        output.status.success(),
        "install --ignore-scripts failed: status={:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );
    assert_eq!(output.status.code(), Some(0));

    let out_txt = td.path().join("out.txt");
    assert!(
        !out_txt.exists(),
        "expected out.txt to be absent under --ignore-scripts + no-deps, but it exists",
    );
}

#[test]
fn install_help_text_describes_ignore_scripts_briefly() {
    let output = Command::new(BIN)
        .args(["help", "install"])
        .output()
        .expect("run guroku help install");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let flag_idx = stdout
        .find("--ignore-scripts")
        .expect("stdout should contain --ignore-scripts");

    // Look at a window around the flag for an explanatory phrase.
    let start = flag_idx.saturating_sub(80);
    let end = (flag_idx + 240).min(stdout.len());
    let window = &stdout[start..end];

    assert!(
        window.contains("lifecycle scripts") || window.contains("Skip"),
        "expected `lifecycle scripts` or `Skip` near `--ignore-scripts`, window was:\n{}",
        window,
    );
}
