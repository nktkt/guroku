use flate2::write::GzEncoder;
use flate2::Compression;
use guroku::store::{ensure_extracted_at, CAS_READY_MARKER};
use std::fs;
use tempfile::TempDir;

#[test]
fn marker_filename_is_dotted_and_lowercase() {
    assert_eq!(CAS_READY_MARKER, ".guroku-cas-ready");
}

#[test]
fn marker_starts_with_dot() {
    assert!(CAS_READY_MARKER.starts_with('.'));
}

#[test]
fn marker_is_not_empty() {
    assert!(CAS_READY_MARKER.len() > 4);
}

#[test]
fn marker_contains_guroku_brand() {
    assert!(CAS_READY_MARKER.contains("guroku"));
}

fn build_minimal_tgz() -> Vec<u8> {
    let pkg_json = br#"{"name":"tiny","version":"0.0.1"}"#;
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    {
        let mut tar = tar::Builder::new(&mut gz);
        let mut header = tar::Header::new_gnu();
        header.set_path("package/package.json").unwrap();
        header.set_size(pkg_json.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append(&header, &pkg_json[..]).unwrap();
        tar.finish().unwrap();
    }
    gz.finish().unwrap()
}

#[test]
fn marker_written_after_ensure_extracted() {
    let tmp = TempDir::new().unwrap();
    let tgz = build_minimal_tgz();

    ensure_extracted_at(tmp.path(), &tgz).expect("ensure_extracted_at should succeed");

    let mut found = false;
    for entry in walk(tmp.path()) {
        if entry.file_name().and_then(|s| s.to_str()) == Some(CAS_READY_MARKER) {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "expected a file named {CAS_READY_MARKER} somewhere under {:?}",
        tmp.path()
    );
}

fn walk(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(p) = stack.pop() {
        if let Ok(rd) = fs::read_dir(&p) {
            for e in rd.flatten() {
                let path = e.path();
                if path.is_dir() {
                    stack.push(path.clone());
                }
                out.push(path);
            }
        }
    }
    out
}
