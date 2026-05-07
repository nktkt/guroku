use std::fs;
use std::process::Command;

use tempfile::TempDir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_guroku")
}

#[test]
fn install_help_mentions_frozen_lockfile() {
    let output = Command::new(bin())
        .args(["help", "install"])
        .output()
        .expect("failed to spawn guroku help install");

    assert!(
        output.status.success(),
        "guroku help install exited with {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--frozen-lockfile"),
        "expected '--frozen-lockfile' in help output, got:\n{}",
        stdout
    );
}

#[test]
fn install_help_describes_install_behaviour() {
    let output = Command::new(bin())
        .args(["help", "install"])
        .output()
        .expect("failed to spawn guroku help install");

    assert!(
        output.status.success(),
        "guroku help install exited with {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Install all dependencies") || stdout.contains("package.json"),
        "expected install behaviour description in:\n{}",
        stdout
    );
}

#[test]
fn top_level_help_lists_install() {
    let output = Command::new(bin())
        .arg("--help")
        .output()
        .expect("failed to spawn guroku --help");

    assert!(
        output.status.success(),
        "guroku --help exited with {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("install"),
        "expected 'install' in --help output:\n{}",
        stdout
    );
}

#[test]
fn install_alias_i_appears_in_help() {
    let output = Command::new(bin())
        .arg("--help")
        .output()
        .expect("failed to spawn guroku --help");
    let stdout = String::from_utf8_lossy(&output.stdout);

    if stdout.contains("[aliases: i]") {
        return;
    }

    let alias_run = Command::new(bin())
        .args(["i", "--help"])
        .output()
        .expect("failed to spawn guroku i --help");

    assert!(
        alias_run.status.success(),
        "neither '[aliases: i]' present in top-level help nor 'guroku i --help' succeeded.\n--help stdout:\n{}\ni --help status: {:?}\ni --help stderr: {}",
        stdout,
        alias_run.status,
        String::from_utf8_lossy(&alias_run.stderr)
    );
}

#[test]
fn frozen_lockfile_without_lockfile_fails() {
    let tmp = TempDir::new().expect("failed to create tempdir");
    let pkg = r#"{"name":"x","version":"0.0.1","dependencies":{"lodash":"^4"}}"#;
    fs::write(tmp.path().join("package.json"), pkg).expect("failed to write package.json");

    assert!(
        !tmp.path().join("guroku.lock").exists(),
        "tempdir should not contain a guroku.lock"
    );

    let output = Command::new(bin())
        .args([
            "--cwd",
            tmp.path().to_str().expect("tempdir path is valid utf8"),
            "install",
            "--frozen-lockfile",
        ])
        .output()
        .expect("failed to spawn guroku install --frozen-lockfile");

    assert!(
        !output.status.success(),
        "expected non-zero exit when --frozen-lockfile is used without a lockfile, got success\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("lockfile"),
        "expected 'lockfile' in stderr, got:\n{}",
        stderr
    );
}
