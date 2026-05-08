//! v1.2: `guroku install --help` surface is unchanged.
//!
//! These tests guard the user-facing CLI from accidental drift in v1.2.
//! The resolver switch to PubGrub is a library-level change and an
//! environment-variable opt-in; it MUST NOT introduce new install flags
//! and MUST NOT leak the implementation choice into help text.

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_guroku")
}

fn install_help() -> String {
    let out = std::process::Command::new(bin())
        .args(["install", "--help"])
        .output()
        .expect("failed to run guroku install --help");
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    s
}

#[test]
fn install_help_lists_frozen_lockfile() {
    let help = install_help();
    assert!(
        help.contains("--frozen-lockfile"),
        "expected `--frozen-lockfile` in `guroku install --help`, got:\n{help}"
    );
}

#[test]
fn install_help_lists_ignore_scripts() {
    let help = install_help();
    assert!(
        help.contains("--ignore-scripts"),
        "expected `--ignore-scripts` in `guroku install --help`, got:\n{help}"
    );
}

#[test]
fn install_help_does_not_introduce_resolver_flag() {
    let help = install_help();
    for flag in ["--resolver", "--pubgrub", "--bfs"] {
        assert!(
            !help.contains(flag),
            "v1.2 must not introduce {flag}; resolver is selected via env var. help:\n{help}"
        );
    }
}

#[test]
fn install_help_does_not_introduce_explain_resolution() {
    let help = install_help();
    assert!(
        !help.contains("--explain-resolution"),
        "`--explain-resolution` was rejected in v1.1 and must remain absent in v1.2. help:\n{help}"
    );
}

#[test]
fn install_help_does_not_mention_pubgrub_directly() {
    let help = install_help();
    assert!(
        !help.contains("pubgrub"),
        "help text must not leak `pubgrub` (implementation detail). help:\n{help}"
    );
    assert!(
        !help.contains("PubGrub"),
        "help text must not leak `PubGrub` (implementation detail). help:\n{help}"
    );
}
