use std::process::Command;

#[test]
fn help_lists_subcommands_and_usage() {
    let output = Command::new(env!("CARGO_BIN_EXE_guroku"))
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
        stdout.contains("Usage:"),
        "missing 'Usage:' in:\n{}",
        stdout
    );
    assert!(
        stdout.contains("install"),
        "missing 'install' in:\n{}",
        stdout
    );
    assert!(stdout.contains("add"), "missing 'add' in:\n{}", stdout);
    assert!(
        stdout.contains("remove"),
        "missing 'remove' in:\n{}",
        stdout
    );
}

#[test]
fn help_install_describes_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_guroku"))
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
        stdout.contains("Install all dependencies"),
        "missing 'Install all dependencies' in:\n{}",
        stdout
    );
}
