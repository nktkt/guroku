#![cfg(unix)]

use std::process::Command;

use tempfile::TempDir;

fn write_pkg(dir: &std::path::Path, body: &str) {
    std::fs::write(dir.join("package.json"), body).unwrap();
}

#[test]
fn forwards_extra_args_after_dashes() {
    let td = TempDir::new().expect("create tempdir");
    write_pkg(
        td.path(),
        r#"{"scripts":{"echo-args":"printf '%s\n' \"$@\""}}"#,
    );

    let output = Command::new(env!("CARGO_BIN_EXE_guroku"))
        .args(["--cwd"])
        .arg(td.path())
        .args(["run", "echo-args", "--", "foo", "bar"])
        .output()
        .expect("failed to spawn guroku run echo-args");

    assert!(
        output.status.success(),
        "guroku run echo-args exited with {:?}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines.contains(&"foo"),
        "missing line 'foo' in stdout:\n{}",
        stdout
    );
    assert!(
        lines.contains(&"bar"),
        "missing line 'bar' in stdout:\n{}",
        stdout
    );
}

#[test]
fn runs_script_without_args_when_no_dashes() {
    let td = TempDir::new().expect("create tempdir");
    write_pkg(td.path(), r#"{"scripts":{"hello":"echo hello"}}"#);

    let output = Command::new(env!("CARGO_BIN_EXE_guroku"))
        .args(["--cwd"])
        .arg(td.path())
        .args(["run", "hello"])
        .output()
        .expect("failed to spawn guroku run hello");

    assert!(
        output.status.success(),
        "guroku run hello exited with {:?}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("hello"),
        "missing 'hello' in stdout:\n{}",
        stdout
    );
}

#[test]
fn script_failure_propagates_nonzero_exit() {
    let td = TempDir::new().expect("create tempdir");
    write_pkg(td.path(), r#"{"scripts":{"fail":"exit 7"}}"#);

    let output = Command::new(env!("CARGO_BIN_EXE_guroku"))
        .args(["--cwd"])
        .arg(td.path())
        .args(["run", "fail"])
        .output()
        .expect("failed to spawn guroku run fail");

    assert!(
        !output.status.success(),
        "expected non-zero exit, got success\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
