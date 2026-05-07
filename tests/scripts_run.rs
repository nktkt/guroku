use std::fs;

use tempfile::TempDir;

#[cfg(unix)]
#[test]
fn runs_simple_echo() {
    let tmp = TempDir::new().expect("create tempdir");
    guroku::scripts::run_in(tmp.path(), "echo_hi", "echo hi > out.txt", &[])
        .expect("script should succeed");
    let content = fs::read_to_string(tmp.path().join("out.txt")).expect("read out.txt");
    assert_eq!(content, "hi\n");
}

#[cfg(unix)]
#[test]
fn cwd_is_passed_through() {
    let tmp = TempDir::new().expect("create tempdir");
    guroku::scripts::run_in(tmp.path(), "pwd_check", "pwd > out.txt", &[])
        .expect("script should succeed");
    let content = fs::read_to_string(tmp.path().join("out.txt")).expect("read out.txt");
    let expected = fs::canonicalize(tmp.path()).expect("canonicalize tempdir");
    assert_eq!(content.trim(), expected.to_string_lossy());
}

#[cfg(unix)]
#[test]
fn success_returns_ok() {
    let tmp = TempDir::new().expect("create tempdir");
    let result = guroku::scripts::run_in(tmp.path(), "trueish", "true", &[]);
    assert!(result.is_ok(), "expected Ok, got {:?}", result);
}

#[cfg(unix)]
#[test]
fn script_writes_relative_to_cwd() {
    let tmp = TempDir::new().expect("create tempdir");
    fs::create_dir_all(tmp.path().join("relative")).expect("mkdir relative");
    guroku::scripts::run_in(
        tmp.path(),
        "relative_write",
        "echo data > ./relative/file.txt",
        &[],
    )
    .expect("script should succeed");
    let target = tmp.path().join("relative").join("file.txt");
    assert!(target.exists(), "expected {:?} to exist", target);
    let content = fs::read_to_string(&target).expect("read file.txt");
    assert_eq!(content, "data\n");
}

#[cfg(unix)]
#[test]
fn multi_command_pipeline() {
    let tmp = TempDir::new().expect("create tempdir");
    guroku::scripts::run_in(
        tmp.path(),
        "pipeline",
        "echo hello | tr a-z A-Z > out.txt",
        &[],
    )
    .expect("script should succeed");
    let content = fs::read_to_string(tmp.path().join("out.txt")).expect("read out.txt");
    assert_eq!(content, "HELLO\n");
}
