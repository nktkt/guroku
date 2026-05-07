use guroku::linker::link_hardlink_tree;
use guroku::store::CAS_READY_MARKER;
use std::fs;
use tempfile::TempDir;

#[test]
fn hardlinks_a_single_file() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("foo.txt"), "hi").unwrap();

    link_hardlink_tree(&src, &dst).unwrap();

    let linked = dst.join("foo.txt");
    assert!(linked.exists(), "dst/foo.txt should exist");
    assert_eq!(fs::read_to_string(&linked).unwrap(), "hi");
}

#[cfg(unix)]
#[test]
fn hardlinks_share_inode_on_unix() {
    use std::os::unix::fs::MetadataExt;

    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("foo.txt"), "hi").unwrap();

    link_hardlink_tree(&src, &dst).unwrap();

    let src_ino = fs::metadata(src.join("foo.txt")).unwrap().ino();
    let dst_ino = fs::metadata(dst.join("foo.txt")).unwrap().ino();
    assert_eq!(src_ino, dst_ino, "hardlinked files should share an inode");
}

#[test]
fn recreates_subdirectories() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("lib").join("sub")).unwrap();
    fs::write(src.join("lib").join("utils.js"), "u").unwrap();
    fs::write(src.join("lib").join("sub").join("inner.js"), "i").unwrap();

    link_hardlink_tree(&src, &dst).unwrap();

    assert!(dst.join("lib").join("utils.js").exists());
    assert!(dst.join("lib").join("sub").join("inner.js").exists());
    assert_eq!(
        fs::read_to_string(dst.join("lib").join("utils.js")).unwrap(),
        "u"
    );
    assert_eq!(
        fs::read_to_string(dst.join("lib").join("sub").join("inner.js")).unwrap(),
        "i"
    );
}

#[test]
fn skips_cas_ready_marker() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join(CAS_READY_MARKER), "").unwrap();
    fs::write(src.join("keep.txt"), "k").unwrap();

    link_hardlink_tree(&src, &dst).unwrap();

    assert!(
        !dst.join(CAS_READY_MARKER).exists(),
        "CAS ready marker should be skipped"
    );
    assert!(dst.join("keep.txt").exists());
}

#[test]
fn creates_dst_if_missing() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("does").join("not").join("exist");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.txt"), "a").unwrap();

    assert!(!dst.exists());
    link_hardlink_tree(&src, &dst).unwrap();

    assert!(dst.exists(), "dst should be created");
    assert!(dst.join("a.txt").exists());
}

#[test]
fn preserves_file_contents() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();

    let bytes: Vec<u8> = (0..=255u8).cycle().take(4096).collect();
    fs::write(src.join("blob.bin"), &bytes).unwrap();

    link_hardlink_tree(&src, &dst).unwrap();

    let read_back = fs::read(dst.join("blob.bin")).unwrap();
    assert_eq!(read_back, bytes, "binary contents should match exactly");
}
