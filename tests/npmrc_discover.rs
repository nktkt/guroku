use guroku::error::GurokuError;
use guroku::npmrc::Npmrc;
use std::fs;
use tempfile::TempDir;

#[test]
fn read_from_existing_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(".npmrc");
    fs::write(&path, "registry=https://corp\n").unwrap();

    let rc = Npmrc::read_from(&path).expect("read_from should succeed");
    assert_eq!(rc.entries["registry"], "https://corp");
}

#[test]
fn read_from_with_comments_and_blank_lines() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(".npmrc");
    let body = "\
; a semicolon comment
# a hash comment

registry=https://corp

# trailing comment
";
    fs::write(&path, body).unwrap();

    let rc = Npmrc::read_from(&path).expect("read_from should succeed");
    assert_eq!(rc.entries.len(), 1);
    assert_eq!(rc.entries["registry"], "https://corp");
}

#[test]
fn read_from_missing_file_returns_err() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("does-not-exist.npmrc");

    let err = Npmrc::read_from(&path).expect_err("missing file should error");
    assert!(matches!(err, GurokuError::Io { .. }));
}

#[test]
fn read_from_empty_file_returns_empty_entries() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(".npmrc");
    fs::write(&path, "").unwrap();

    let rc = Npmrc::read_from(&path).expect("read_from should succeed");
    assert!(rc.entries.is_empty());
}

#[test]
fn read_from_handles_crlf_line_endings() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(".npmrc");
    fs::write(
        &path,
        "registry=https://corp\r\n@scope:registry=https://s\r\n",
    )
    .unwrap();

    let rc = Npmrc::read_from(&path).expect("read_from should succeed");
    assert_eq!(rc.entries["registry"], "https://corp");
    assert_eq!(rc.entries["@scope:registry"], "https://s");
}
