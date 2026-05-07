use std::io::Write;
use std::path::Path;

use guroku::store::{ensure_extracted_at, CAS_READY_MARKER};
use tempfile::TempDir;

fn make_tgz(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut tar = tar::Builder::new(Vec::new());
    for (path, content) in entries {
        let mut h = tar::Header::new_gnu();
        h.set_path(path).unwrap();
        h.set_size(content.len() as u64);
        h.set_mode(0o644);
        h.set_cksum();
        tar.append(&h, *content).unwrap();
    }
    let bytes = tar.into_inner().unwrap();
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gz.write_all(&bytes).unwrap();
    gz.finish().unwrap()
}

fn sample_tgz() -> Vec<u8> {
    make_tgz(&[
        (
            "package/package.json",
            br#"{"name":"x","version":"1.0.0"}"# as &[u8],
        ),
        ("package/index.js", b"module.exports = 1;\n" as &[u8]),
    ])
}

#[test]
fn extracts_tarball_into_cas_layout() {
    let td = TempDir::new().unwrap();
    let tgz = sample_tgz();
    let path = ensure_extracted_at(td.path(), &tgz).unwrap();

    assert!(
        path.starts_with(td.path()),
        "{:?} not under {:?}",
        path,
        td.path()
    );
    assert!(path.join("package.json").is_file());
    assert!(path.join("index.js").is_file());
    assert!(path.join(CAS_READY_MARKER).exists());
}

#[test]
fn idempotent_second_call_is_fast_noop() {
    let td = TempDir::new().unwrap();
    let tgz = sample_tgz();

    let p1 = ensure_extracted_at(td.path(), &tgz).unwrap();
    let p2 = ensure_extracted_at(td.path(), &tgz).unwrap();

    assert_eq!(p1, p2);
    assert!(p1.join(CAS_READY_MARKER).exists());
    assert!(p2.join("package.json").is_file());
}

#[test]
fn different_bytes_produce_different_paths() {
    let td = TempDir::new().unwrap();
    let tgz_a = make_tgz(&[(
        "package/package.json",
        b"{\"name\":\"a\",\"version\":\"1.0.0\"}" as &[u8],
    )]);
    let tgz_b = make_tgz(&[(
        "package/package.json",
        b"{\"name\":\"b\",\"version\":\"2.0.0\"}" as &[u8],
    )]);

    let pa = ensure_extracted_at(td.path(), &tgz_a).unwrap();
    let pb = ensure_extracted_at(td.path(), &tgz_b).unwrap();

    assert_ne!(pa, pb);

    let prefix_a = pa.parent().unwrap().parent().unwrap();
    let prefix_b = pb.parent().unwrap().parent().unwrap();
    assert_eq!(prefix_a, td.path());
    assert_eq!(prefix_b, td.path());

    let two_a = pa.parent().unwrap().file_name().unwrap();
    let two_b = pb.parent().unwrap().file_name().unwrap();
    assert_ne!(two_a, two_b, "expected different sha-prefix dirs");
}

#[test]
fn marker_present_after_success() {
    let td = TempDir::new().unwrap();
    let tgz = sample_tgz();
    let path = ensure_extracted_at(td.path(), &tgz).unwrap();
    let marker: &Path = &path.join(CAS_READY_MARKER);
    assert!(marker.exists(), "marker {:?} missing", marker);
    assert_eq!(CAS_READY_MARKER, ".guroku-cas-ready");
}

#[test]
fn cas_path_uses_two_char_prefix_dir() {
    let td = TempDir::new().unwrap();
    let tgz = sample_tgz();
    let path = ensure_extracted_at(td.path(), &tgz).unwrap();

    let parent = path.parent().expect("has parent");
    let grandparent = parent.parent().expect("has grandparent");

    assert_eq!(grandparent, td.path());

    let two = parent.file_name().unwrap().to_string_lossy().to_string();
    assert_eq!(two.len(), 2, "expected 2-char prefix dir, got {:?}", two);
    assert!(
        two.chars().all(|c| c.is_ascii_hexdigit()),
        "expected hex prefix, got {:?}",
        two
    );
}
