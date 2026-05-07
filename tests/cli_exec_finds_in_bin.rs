#![cfg(unix)]

use std::process::Command;
use tempfile::TempDir;

#[cfg(unix)]
fn write_bin(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(dir).unwrap();
    let p = dir.join(name);
    std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
    let mut perms = std::fs::metadata(&p).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&p, perms).unwrap();
    p
}

fn guroku() -> Command {
    Command::new(env!("CARGO_BIN_EXE_guroku"))
}

#[test]
fn runs_binary_in_node_modules_dot_bin() {
    let td = TempDir::new().unwrap();
    let bin_dir = td.path().join("node_modules").join(".bin");
    write_bin(&bin_dir, "widget", "echo BIN-WIDGET");

    let output = guroku()
        .arg("--cwd")
        .arg(td.path())
        .args(["exec", "widget"])
        .output()
        .expect("failed to spawn guroku exec");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "guroku exec failed: {:?}\nstdout: {}\nstderr: {}",
        output.status,
        stdout,
        stderr
    );
    assert!(
        stdout.contains("BIN-WIDGET"),
        "stdout missing 'BIN-WIDGET':\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );
}

#[test]
fn args_forwarded_to_bin() {
    let td = TempDir::new().unwrap();
    let bin_dir = td.path().join("node_modules").join(".bin");
    write_bin(&bin_dir, "widget", r#"printf '%s\n' "$@""#);

    let output = guroku()
        .arg("--cwd")
        .arg(td.path())
        .args(["exec", "widget", "arg1", "arg2"])
        .output()
        .expect("failed to spawn guroku exec");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "guroku exec failed: {:?}\nstdout: {}\nstderr: {}",
        output.status,
        stdout,
        stderr
    );
    assert!(
        stdout.contains("arg1\narg2"),
        "stdout missing 'arg1\\narg2':\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );
}

#[test]
fn falls_back_to_path_for_system_command() {
    let td = TempDir::new().unwrap();

    let output = guroku()
        .arg("--cwd")
        .arg(td.path())
        .args(["exec", "ls"])
        .output()
        .expect("failed to spawn guroku exec");

    assert!(
        output.status.success(),
        "guroku exec ls failed: {:?}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn unknown_command_errors_clearly() {
    let td = TempDir::new().unwrap();

    let output = guroku()
        .arg("--cwd")
        .arg(td.path())
        .args(["exec", "definitely-not-a-command-xyzzy-9999"])
        .output()
        .expect("failed to spawn guroku exec");

    assert!(
        !output.status.success(),
        "expected non-zero exit, got success\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not on PATH") || stderr.contains("node_modules/.bin"),
        "stderr missing expected error text:\nstderr: {}",
        stderr
    );
}
