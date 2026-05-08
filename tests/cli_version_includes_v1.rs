use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_guroku")
}

#[test]
fn long_flag_prints_v1() {
    let output = Command::new(bin())
        .arg("--version")
        .output()
        .expect("failed to run guroku --version");
    assert!(output.status.success(), "guroku --version exited non-zero");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("1.0.0"),
        "expected stdout to contain '1.0.0', got: {stdout}"
    );
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "expected stdout to contain CARGO_PKG_VERSION, got: {stdout}"
    );
}

#[test]
fn short_flag_prints_v1() {
    let output = Command::new(bin())
        .arg("-V")
        .output()
        .expect("failed to run guroku -V");
    assert!(output.status.success(), "guroku -V exited non-zero");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("1.0.0"),
        "expected stdout to contain '1.0.0', got: {stdout}"
    );
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "expected stdout to contain CARGO_PKG_VERSION, got: {stdout}"
    );
}

#[test]
fn version_includes_guroku_name() {
    let output = Command::new(bin())
        .arg("--version")
        .output()
        .expect("failed to run guroku --version");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("guroku "),
        "expected stdout to contain 'guroku ', got: {stdout}"
    );
}

#[test]
fn crate_version_starts_with_one() {
    let v = env!("CARGO_PKG_VERSION");
    assert!(
        v.starts_with("1."),
        "expected CARGO_PKG_VERSION to start with '1.', got: {v}"
    );
}
