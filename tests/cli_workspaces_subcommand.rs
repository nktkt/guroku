use std::process::Command;

use tempfile::TempDir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_guroku")
}

fn write(p: &std::path::Path, body: &str) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, body).unwrap();
}

#[test]
fn workspaces_with_no_field_says_so() {
    let td = TempDir::new().expect("failed to create tempdir");
    write(
        &td.path().join("package.json"),
        r#"{"name":"r","version":"0.1.0"}"#,
    );

    let output = Command::new(bin())
        .args([
            "--cwd",
            td.path().to_str().expect("tempdir path is valid utf8"),
            "workspaces",
        ])
        .output()
        .expect("failed to spawn guroku workspaces");

    assert!(
        output.status.success(),
        "guroku workspaces exited with {:?}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("no workspaces declared"),
        "expected 'no workspaces declared' in stdout, got:\n{}",
        stdout
    );
}

#[test]
fn workspaces_lists_discovered_packages() {
    let td = TempDir::new().expect("failed to create tempdir");
    write(
        &td.path().join("package.json"),
        r#"{"name":"r","version":"0.1.0","workspaces":["packages/*"]}"#,
    );
    write(
        &td.path().join("packages/a/package.json"),
        r#"{"name":"@acme/a","version":"1.0.0"}"#,
    );
    write(
        &td.path().join("packages/b/package.json"),
        r#"{"name":"@acme/b","version":"2.0.0"}"#,
    );

    let output = Command::new(bin())
        .args([
            "--cwd",
            td.path().to_str().expect("tempdir path is valid utf8"),
            "workspaces",
        ])
        .output()
        .expect("failed to spawn guroku workspaces");

    assert!(
        output.status.success(),
        "guroku workspaces exited with {:?}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    for needle in [
        "@acme/a",
        "@acme/b",
        "1.0.0",
        "2.0.0",
        "found 2 workspace package(s)",
    ] {
        assert!(
            stdout.contains(needle),
            "expected '{}' in stdout, got:\n{}",
            needle,
            stdout
        );
    }
}

#[test]
fn workspaces_subcommand_appears_in_top_level_help() {
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
        stdout.contains("workspaces"),
        "expected 'workspaces' in --help output:\n{}",
        stdout
    );
}

#[test]
fn workspaces_help_subcommand_smoke() {
    let output = Command::new(bin())
        .args(["help", "workspaces"])
        .output()
        .expect("failed to spawn guroku help workspaces");

    assert!(
        output.status.success(),
        "guroku help workspaces exited with {:?}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
