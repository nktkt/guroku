use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_guroku"))
}

#[test]
fn prints_version_with_long_flag() {
    let output = bin()
        .arg("--version")
        .output()
        .expect("failed to execute guroku");
    assert!(
        output.status.success(),
        "expected success, got {:?}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "stdout did not contain version {}: {}",
        env!("CARGO_PKG_VERSION"),
        stdout
    );
}

#[test]
fn prints_version_with_short_flag() {
    let output = bin().arg("-V").output().expect("failed to execute guroku");
    assert!(
        output.status.success(),
        "expected success, got {:?}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "stdout did not contain version {}: {}",
        env!("CARGO_PKG_VERSION"),
        stdout
    );
}

#[test]
fn nonzero_exit_for_unknown_subcommand() {
    let output = bin()
        .arg("nonexistent-subcommand")
        .output()
        .expect("failed to execute guroku");
    assert!(
        !output.status.success(),
        "expected non-zero exit, got {:?}",
        output.status
    );
}
