use std::process::Command;
use tempfile::TempDir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_guroku")
}

#[test]
fn audit_without_lockfile_fails() {
    let td = TempDir::new().expect("tempdir");
    let output = Command::new(bin())
        .arg("--cwd")
        .arg(td.path())
        .arg("audit")
        .output()
        .expect("failed to run guroku");
    assert!(
        !output.status.success(),
        "expected non-zero exit, got {:?}",
        output.status
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("guroku.lock not found"),
        "stderr did not contain 'guroku.lock not found': {}",
        stderr
    );
}

#[test]
fn audit_subcommand_listed_in_help() {
    let output = Command::new(bin())
        .arg("--help")
        .output()
        .expect("failed to run guroku");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("audit"),
        "expected --help output to mention 'audit', got: {}",
        stdout
    );
}

#[test]
fn audit_help_describes_purpose() {
    let output = Command::new(bin())
        .arg("help")
        .arg("audit")
        .output()
        .expect("failed to run guroku");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("advisory") || stdout.contains("vulnerabilit"),
        "expected `help audit` output to mention 'advisory' or 'vulnerabilit', got: {}",
        stdout
    );
}
