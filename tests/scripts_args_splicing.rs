use std::fs;
use std::path::Path;

use tempfile::TempDir;

#[cfg(unix)]
#[test]
fn passes_simple_args_through_to_script() {
    let tmp = TempDir::new().expect("create tempdir");
    let extra: Vec<String> = vec!["a".to_string(), "b".to_string()];
    let env_extra: &[&Path] = &[];
    guroku::scripts::run_in_with_args(
        tmp.path(),
        "print_args",
        "printf '%s\\n' \"$@\" > out.txt",
        &extra,
        env_extra,
    )
    .expect("script should succeed");
    let content = fs::read_to_string(tmp.path().join("out.txt")).expect("read out.txt");
    assert_eq!(content, "a\nb\n");
}

#[cfg(unix)]
#[test]
fn args_with_spaces_preserved() {
    let tmp = TempDir::new().expect("create tempdir");
    let extra: Vec<String> = vec!["hello world".to_string()];
    let env_extra: &[&Path] = &[];
    guroku::scripts::run_in_with_args(
        tmp.path(),
        "print_args",
        "printf '%s\\n' \"$@\" > out.txt",
        &extra,
        env_extra,
    )
    .expect("script should succeed");
    let content = fs::read_to_string(tmp.path().join("out.txt")).expect("read out.txt");
    assert_eq!(content, "hello world\n");
}

#[cfg(unix)]
#[test]
fn args_with_quotes_preserved() {
    let tmp = TempDir::new().expect("create tempdir");
    let extra: Vec<String> = vec!["it's".to_string()];
    let env_extra: &[&Path] = &[];
    guroku::scripts::run_in_with_args(
        tmp.path(),
        "print_args",
        "printf '%s\\n' \"$@\" > out.txt",
        &extra,
        env_extra,
    )
    .expect("script should succeed");
    let content = fs::read_to_string(tmp.path().join("out.txt")).expect("read out.txt");
    assert_eq!(content, "it's\n");
}

#[cfg(unix)]
#[test]
fn no_args_runs_body_unchanged() {
    let tmp = TempDir::new().expect("create tempdir");
    let extra: Vec<String> = vec![];
    let env_extra: &[&Path] = &[];
    guroku::scripts::run_in_with_args(
        tmp.path(),
        "no_args",
        "echo nothing > out.txt",
        &extra,
        env_extra,
    )
    .expect("script should succeed");
    let content = fs::read_to_string(tmp.path().join("out.txt")).expect("read out.txt");
    assert_eq!(content, "nothing\n");
}

#[cfg(unix)]
#[test]
fn args_with_dollar_signs_not_expanded() {
    let tmp = TempDir::new().expect("create tempdir");
    let extra: Vec<String> = vec!["$HOME".to_string()];
    let env_extra: &[&Path] = &[];
    guroku::scripts::run_in_with_args(
        tmp.path(),
        "print_args",
        "printf '%s\\n' \"$@\" > out.txt",
        &extra,
        env_extra,
    )
    .expect("script should succeed");
    let content = fs::read_to_string(tmp.path().join("out.txt")).expect("read out.txt");
    assert_eq!(content, "$HOME\n");
}
