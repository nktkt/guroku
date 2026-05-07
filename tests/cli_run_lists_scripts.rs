use std::process::Command;

use tempfile::TempDir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_guroku")
}

fn write_pkg(dir: &std::path::Path, body: &str) {
    std::fs::write(dir.join("package.json"), body).unwrap();
}

#[test]
fn run_with_no_name_lists_scripts() {
    let tmp = TempDir::new().expect("failed to create tempdir");
    write_pkg(
        tmp.path(),
        r#"{"scripts": {"build": "tsc", "test": "echo"}}"#,
    );

    let output = Command::new(bin())
        .args([
            "--cwd",
            tmp.path().to_str().expect("tempdir path is valid utf8"),
            "run",
        ])
        .output()
        .expect("failed to spawn guroku run");

    assert!(
        output.status.success(),
        "guroku run exited with {:?}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("build"),
        "expected 'build' in stdout, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("test"),
        "expected 'test' in stdout, got:\n{}",
        stdout
    );
}

#[test]
fn run_with_no_name_no_scripts_says_so() {
    let tmp = TempDir::new().expect("failed to create tempdir");
    write_pkg(tmp.path(), r#"{"name":"x","version":"0.0.1"}"#);

    let output = Command::new(bin())
        .args([
            "--cwd",
            tmp.path().to_str().expect("tempdir path is valid utf8"),
            "run",
        ])
        .output()
        .expect("failed to spawn guroku run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("no scripts"),
        "expected 'no scripts' in stdout, got:\nstdout: {}\nstderr: {}",
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn run_unknown_script_errors() {
    let tmp = TempDir::new().expect("failed to create tempdir");
    write_pkg(tmp.path(), r#"{"scripts":{"build":"tsc"}}"#);

    let output = Command::new(bin())
        .args([
            "--cwd",
            tmp.path().to_str().expect("tempdir path is valid utf8"),
            "run",
            "unknown",
        ])
        .output()
        .expect("failed to spawn guroku run unknown");

    assert!(
        !output.status.success(),
        "expected non-zero exit for unknown script, got success\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no `unknown` script"),
        "expected \"no `unknown` script\" in stderr, got:\n{}",
        stderr
    );
}
