use std::process::Command;

fn run_guroku(args: &[&str]) -> String {
    let bin = env!("CARGO_BIN_EXE_guroku");
    let output = Command::new(bin)
        .args(args)
        .output()
        .expect("failed to run guroku");
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    combined
}

fn extract_subcommands(help: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_commands = false;
    for line in help.lines() {
        if line.trim_start().starts_with("Commands:") {
            in_commands = true;
            continue;
        }
        if in_commands {
            let trimmed = line.trim_start();
            if trimmed.is_empty() || !line.starts_with(' ') {
                break;
            }
            if let Some(name) = trimmed.split_whitespace().next() {
                out.push(name.to_string());
            }
        }
    }
    out
}

#[test]
fn version_starts_with_one_two() {
    let output = run_guroku(&["--version"]);
    assert!(
        output.starts_with("guroku 1.2."),
        "expected version output to start with `guroku 1.2.`, got: {:?}",
        output
    );
}

#[test]
fn crate_version_is_one_two() {
    let v = env!("CARGO_PKG_VERSION");
    assert!(
        v.starts_with("1.2."),
        "expected CARGO_PKG_VERSION to start with `1.2.`, got: {:?}",
        v
    );
}

#[test]
fn no_new_top_level_subcommand_in_v1_2() {
    let help = run_guroku(&["--help"]);
    let mut subcommands = extract_subcommands(&help);
    subcommands.sort();
    subcommands.dedup();

    let allowed: Vec<String> = [
        "add",
        "audit",
        "exec",
        "help",
        "install",
        "remove",
        "run",
        "workspaces",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    assert_eq!(
        subcommands, allowed,
        "v1.2 must not introduce or remove top-level subcommands; v1.0 froze the CLI surface. \
         expected {:?}, got {:?}. full help:\n{}",
        allowed, subcommands, help
    );
}
