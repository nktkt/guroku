use std::io::Write;
use std::path::Path;

use flate2::write::GzEncoder;
use flate2::Compression;
use tar::{Builder, Header};
use tempfile::TempDir;

fn make_tgz(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut tar = Builder::new(Vec::new());
    for (path, content) in entries {
        let mut header = Header::new_gnu();
        header.set_path(path).unwrap();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append(&header, *content).unwrap();
    }
    let tar_bytes = tar.into_inner().unwrap();
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    gz.write_all(&tar_bytes).unwrap();
    gz.finish().unwrap()
}

fn assert_file_contents(path: &Path, expected: &[u8]) {
    let actual =
        std::fs::read(path).unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e));
    assert_eq!(actual, expected, "contents differ at {}", path.display());
}

#[test]
fn extracts_and_strips_package_prefix() {
    let pkg_json = br#"{"name":"demo","version":"1.0.0"}"#;
    let index_js = b"console.log('hi');\n";
    let tgz = make_tgz(&[
        ("package/package.json", pkg_json),
        ("package/index.js", index_js),
    ]);

    let dest = TempDir::new().unwrap();
    guroku::tarball::extract(&tgz, dest.path()).expect("extract should succeed");

    assert_file_contents(&dest.path().join("package.json"), pkg_json);
    assert_file_contents(&dest.path().join("index.js"), index_js);
    assert!(
        !dest.path().join("package").exists(),
        "the `package/` prefix should have been stripped"
    );
}

#[test]
fn extracts_nested_directories() {
    let util_js = b"module.exports = {};\n";
    let inner_js = b"// inner\n";
    let tgz = make_tgz(&[
        ("package/lib/util.js", util_js),
        ("package/lib/sub/inner.js", inner_js),
    ]);

    let dest = TempDir::new().unwrap();
    guroku::tarball::extract(&tgz, dest.path()).expect("extract should succeed");

    assert_file_contents(&dest.path().join("lib").join("util.js"), util_js);
    assert_file_contents(
        &dest.path().join("lib").join("sub").join("inner.js"),
        inner_js,
    );
}

// Build a tarball whose entry path bytes are written directly into the
// header, bypassing `tar::Header::set_path`'s refusal to handle `..`.
fn make_tgz_unsafe(path_bytes: &[u8], content: &[u8]) -> Vec<u8> {
    let mut tar = Builder::new(Vec::new());
    let mut header = Header::new_gnu();
    {
        let name_field = &mut header.as_old_mut().name;
        let len = path_bytes.len().min(name_field.len());
        name_field[..len].copy_from_slice(&path_bytes[..len]);
    }
    header.set_size(content.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append(&header, content).unwrap();
    let tar_bytes = tar.into_inner().unwrap();
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    gz.write_all(&tar_bytes).unwrap();
    gz.finish().unwrap()
}

#[test]
fn rejects_path_traversal() {
    // After `package/` is stripped, the entry path becomes `../etc/passwd`,
    // which would escape the destination directory.
    let tgz = make_tgz_unsafe(b"package/../etc/passwd", b"pwned\n");

    let dest = TempDir::new().unwrap();
    let result = guroku::tarball::extract(&tgz, dest.path());
    assert!(
        result.is_err(),
        "extract should reject path traversal entries"
    );
}
