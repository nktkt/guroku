fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_guroku")
}

fn run_version_flag(flag: &str) -> String {
    let out = std::process::Command::new(bin())
        .arg(flag)
        .output()
        .expect("failed to run guroku --version");
    assert!(out.status.success(), "exited non-zero");
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[test]
fn version_starts_with_v1_2() {
    let stdout = run_version_flag("--version");
    assert!(
        stdout.starts_with("guroku 1.2."),
        "expected stdout to start with 'guroku 1.2.', got: {stdout}"
    );
}

#[test]
fn short_v_flag_starts_with_v1_2() {
    let stdout = run_version_flag("-V");
    assert!(
        stdout.starts_with("guroku 1.2."),
        "expected stdout to start with 'guroku 1.2.', got: {stdout}"
    );
}

#[test]
fn version_contains_cargo_pkg_version() {
    let stdout = run_version_flag("--version");
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "expected stdout to contain CARGO_PKG_VERSION, got: {stdout}"
    );
}

#[test]
fn crate_version_starts_with_one_two() {
    let v = env!("CARGO_PKG_VERSION");
    assert!(
        v.starts_with("1.2."),
        "expected CARGO_PKG_VERSION to start with '1.2.', got: {v}"
    );
}
