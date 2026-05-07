use guroku::http_cache::{read_in, write_in};
use tempfile::TempDir;

#[test]
fn read_returns_none_when_absent() {
    let dir = TempDir::new().unwrap();
    let result = read_in(dir.path(), "lodash").unwrap();
    assert!(result.is_none());
}

#[test]
fn write_then_read_with_etag() {
    let dir = TempDir::new().unwrap();
    write_in(dir.path(), "lodash", b"hello", Some("\"abc\"")).unwrap();
    let result = read_in(dir.path(), "lodash").unwrap();
    let cached = result.expect("expected cached entry");
    assert_eq!(cached.body, b"hello");
    assert_eq!(cached.etag, Some("\"abc\"".to_string()));
}

#[test]
fn write_without_etag_yields_none_etag_on_read() {
    let dir = TempDir::new().unwrap();
    write_in(dir.path(), "lodash", b"hi", None).unwrap();
    let result = read_in(dir.path(), "lodash").unwrap();
    let cached = result.expect("expected cached entry");
    assert_eq!(cached.body, b"hi");
    assert_eq!(cached.etag, None);
}

#[test]
fn write_with_etag_then_overwrite_without_etag_clears_etag() {
    let dir = TempDir::new().unwrap();
    write_in(dir.path(), "lodash", b"first", Some("\"v1\"")).unwrap();
    write_in(dir.path(), "lodash", b"second", None).unwrap();

    let etag_path = dir.path().join("lodash.etag");
    assert!(
        !etag_path.exists(),
        ".etag file should be removed when overwritten without etag"
    );

    let result = read_in(dir.path(), "lodash").unwrap();
    let cached = result.expect("expected cached entry");
    assert_eq!(cached.body, b"second");
    assert_eq!(cached.etag, None);
}

#[test]
fn scoped_name_uses_plus_in_filename() {
    let dir = TempDir::new().unwrap();
    write_in(dir.path(), "@types/node", b"data", None).unwrap();
    let expected = dir.path().join("@types+node.json");
    assert!(expected.exists(), "expected file {:?} to exist", expected);
}

#[test]
fn read_returns_body_bytes_verbatim() {
    let dir = TempDir::new().unwrap();
    let bytes: &[u8] = &[0x00, 0xFF, 0x00, 0xFF];
    write_in(dir.path(), "binpkg", bytes, None).unwrap();
    let result = read_in(dir.path(), "binpkg").unwrap();
    let cached = result.expect("expected cached entry");
    assert_eq!(cached.body, bytes);
}
