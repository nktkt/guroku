//! v1.0-stability check: `guroku install --help` must keep mentioning the
//! v1.0 flags `--frozen-lockfile` and `--ignore-scripts`. v1.0 promised these
//! would not change.
//!
//! v1.1 is purely additive on top of v1.0; it must NOT introduce any of the
//! flags listed in `UNINTRODUCED_V1_1_FLAGS` below until a stability story
//! exists for them.

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_guroku");

fn bin() -> Command {
    Command::new(BIN)
}

fn install_help_combined() -> String {
    let out = bin()
        .args(["install", "--help"])
        .output()
        .expect("failed to run `guroku install --help`");
    // clap may emit help on either stream depending on derive config; check both.
    let mut combined = String::from_utf8_lossy(&out.stdout).into_owned();
    combined.push('\n');
    combined.push_str(&String::from_utf8_lossy(&out.stderr));
    combined
}

#[test]
fn install_help_lists_frozen_lockfile() {
    let help = install_help_combined();
    assert!(
        help.contains("--frozen-lockfile"),
        "`guroku install --help` must mention v1.0 flag `--frozen-lockfile`, got:\n{help}"
    );
}

#[test]
fn install_help_lists_ignore_scripts() {
    let help = install_help_combined();
    assert!(
        help.contains("--ignore-scripts"),
        "`guroku install --help` must mention v1.0 flag `--ignore-scripts`, got:\n{help}"
    );
}

#[test]
fn install_help_does_not_introduce_unintroduced_flags() {
    // These flags are explicitly NOT shipped in v1.1. Guard against accidental
    // additions while we don't have a stability story for them.
    const UNINTRODUCED_V1_1_FLAGS: &[&str] = &["--alias", "--no-overrides", "--explain-resolution"];

    let help = install_help_combined();
    for flag in UNINTRODUCED_V1_1_FLAGS {
        assert!(
            !help.contains(flag),
            "`guroku install --help` unexpectedly introduces `{flag}`; \
             v1.1 surface must remain purely additive without a stability story.\n\
             Help was:\n{help}"
        );
    }
}
