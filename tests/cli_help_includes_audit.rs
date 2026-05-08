use std::process::Command;
use tempfile::TempDir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_guroku")
}

#[test]
fn top_level_help_lists_audit() {
    let output = Command::new(bin())
        .arg("--help")
        .output()
        .expect("failed to run guroku --help");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("audit"),
        "expected `--help` output to mention 'audit', got: {}",
        stdout
    );
}

#[test]
fn audit_help_describes_advisories() {
    let output = Command::new(bin())
        .arg("help")
        .arg("audit")
        .output()
        .expect("failed to run guroku help audit");
    assert!(
        output.status.success(),
        "expected `help audit` to exit successfully, got {:?}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
    assert!(
        stdout.contains("advisory") || stdout.contains("vulnerabilit"),
        "expected `help audit` output to mention 'advisory' or 'vulnerabilit' (case-insensitive), got: {}",
        stdout
    );
}

#[test]
fn audit_no_args_smoke() {
    let td = TempDir::new().expect("tempdir");
    let output = Command::new(bin())
        .arg("--cwd")
        .arg(td.path())
        .arg("audit")
        .output()
        .expect("failed to run guroku audit");
    assert!(
        !output.status.success(),
        "expected non-zero exit when guroku.lock is absent, got {:?}",
        output.status
    );
}

#[test]
fn version_string_is_v05() {
    let output = Command::new(bin())
        .arg("--version")
        .output()
        .expect("failed to run guroku --version");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected = env!("CARGO_PKG_VERSION");
    assert!(
        stdout.contains(expected),
        "expected `--version` output to contain crate version '{}', got: {}",
        expected,
        stdout
    );
}
