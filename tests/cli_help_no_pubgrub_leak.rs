fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_guroku")
}

fn run_help_or_version(args: &[&str]) -> String {
    let out = std::process::Command::new(bin())
        .args(args)
        .output()
        .expect("failed to run guroku");
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    s
}

fn assert_no_pubgrub(text: &str, what: &str) {
    let lower = text.to_lowercase();
    assert!(
        !lower.contains("pubgrub"),
        "{what} mentions 'pubgrub' (implementation leak): {text}"
    );
}

#[test]
fn top_level_help_no_pubgrub() {
    let out = run_help_or_version(&["--help"]);
    assert_no_pubgrub(&out, "guroku --help");
}

#[test]
fn install_help_no_pubgrub() {
    let out = run_help_or_version(&["install", "--help"]);
    assert_no_pubgrub(&out, "guroku install --help");
}

#[test]
fn version_string_no_pubgrub() {
    let out = run_help_or_version(&["--version"]);
    assert_no_pubgrub(&out, "guroku --version");
}
