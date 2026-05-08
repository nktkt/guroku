//! v1.1 CLI shape pinning: no new subcommands, version is 1.1.x, top-level
//! flags are exactly `-C/--cwd`, `-h/--help`, `-V/--version`.
//!
//! The v1.0 CLI surface is SemVer-stable. v1.1 must NOT add or remove any
//! top-level subcommand. If you intend to introduce one, update these tests
//! AND document the addition as a non-breaking minor change in CHANGELOG.

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_guroku");

const V1_SUBCOMMANDS: &[&str] = &[
    "install",
    "add",
    "remove",
    "run",
    "exec",
    "workspaces",
    "audit",
];

fn help_stdout() -> String {
    let out = Command::new(BIN)
        .arg("--help")
        .output()
        .expect("failed to run `guroku --help`");
    assert!(out.status.success(), "`guroku --help` exited non-zero");
    String::from_utf8(out.stdout).expect("`guroku --help` stdout was not UTF-8")
}

#[test]
fn v1_subcommand_inventory_unchanged() {
    let help = help_stdout();
    for sub in V1_SUBCOMMANDS {
        assert!(
            help.contains(sub),
            "`guroku --help` is missing v1 subcommand `{sub}`. \
             v1.1 must not remove subcommands."
        );
    }
}

#[test]
fn version_starts_with_one_one() {
    let out = Command::new(BIN)
        .arg("--version")
        .output()
        .expect("failed to run `guroku --version`");
    assert!(out.status.success(), "`guroku --version` exited non-zero");
    let stdout = String::from_utf8(out.stdout).expect("non-UTF-8 stdout");
    assert!(
        stdout.starts_with("guroku 1."),
        "expected `guroku --version` to start with `guroku 1.`, got: {stdout:?}"
    );
}

#[test]
fn crate_version_is_one_x() {
    let v = env!("CARGO_PKG_VERSION");
    assert!(
        v.starts_with("1."),
        "CARGO_PKG_VERSION should be 1.x.x, got: {v}"
    );
}

#[test]
fn no_unexpected_top_level_flag() {
    let help = help_stdout();
    // The required top-level flags must all be documented.
    for needle in ["-C", "--cwd", "-h", "--help", "-V", "--version"] {
        assert!(
            help.contains(needle),
            "`guroku --help` should document top-level flag `{needle}`"
        );
    }
    // Any other long flag at the top level is a v1.1 surface change.
    let allowed_long = ["--cwd", "--help", "--version"];
    for line in help.lines() {
        let trimmed = line.trim_start();
        for tok in trimmed.split_whitespace() {
            let flag = tok.trim_end_matches([',', '=']);
            if flag.starts_with("--") && flag.len() > 2 && !allowed_long.contains(&flag) {
                panic!(
                    "unexpected top-level long flag `{flag}` in `guroku --help`; \
                     v1.1 allows only {allowed_long:?}"
                );
            }
        }
    }
}
