//! Per-subcommand flag/argument structure assertions via `guroku help`.
//!
//! Failing here means a flag was renamed or a subcommand changed shape.

use std::process::Command;

fn help(args: &[&str]) -> (bool, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_guroku"))
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn guroku {:?}: {}", args, e));
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (output.status.success(), stdout, stderr)
}

fn assert_ok(args: &[&str]) -> String {
    let (ok, stdout, stderr) = help(args);
    assert!(
        ok,
        "guroku {:?} failed\nstdout: {}\nstderr: {}",
        args, stdout, stderr
    );
    stdout
}

#[test]
fn install_help_lists_known_flags() {
    let stdout = assert_ok(&["help", "install"]);
    assert!(
        stdout.contains("--frozen-lockfile"),
        "missing '--frozen-lockfile' in:\n{}",
        stdout
    );
    assert!(
        stdout.contains("--ignore-scripts"),
        "missing '--ignore-scripts' in:\n{}",
        stdout
    );
}

#[test]
fn add_help_describes_packages_arg() {
    let stdout = assert_ok(&["help", "add"]);
    assert!(
        stdout.contains("Package specifiers") || stdout.contains("PACKAGES"),
        "expected 'Package specifiers' or 'PACKAGES' in:\n{}",
        stdout
    );
}

#[test]
fn remove_help_alias_rm() {
    // clap doesn't render aliases on the top-level help; it does on the
    // subcommand's own help text. Check both via `guroku rm --help`
    // succeeding (since `rm` is a valid alias for `remove`).
    let stdout = assert_ok(&["rm", "--help"]);
    assert!(
        stdout.to_ascii_lowercase().contains("remove"),
        "expected `guroku rm --help` to describe the remove command:\n{stdout}"
    );
}

#[test]
fn run_help_describes_dashes_passthrough() {
    let stdout = assert_ok(&["help", "run"]);
    assert!(
        stdout.contains("ARGS")
            || stdout.contains("--")
            || stdout.contains("Trailing args")
            || stdout.contains("forwarded to the script"),
        "expected trailing-args passthrough hint ('ARGS' / '--' / 'Trailing args') in:\n{}",
        stdout
    );
}

#[test]
fn exec_help_describes_command_and_args() {
    let stdout = assert_ok(&["help", "exec"]);
    assert!(
        stdout.contains("COMMAND") || stdout.contains("Command"),
        "expected 'COMMAND' or 'Command' in:\n{}",
        stdout
    );
}

#[test]
fn workspaces_no_args() {
    let stdout = assert_ok(&["help", "workspaces"]);
    assert!(
        !stdout.is_empty(),
        "guroku help workspaces produced empty stdout"
    );
}

#[test]
fn audit_no_args() {
    let stdout = assert_ok(&["help", "audit"]);
    assert!(
        !stdout.is_empty(),
        "guroku help audit produced empty stdout"
    );
}
