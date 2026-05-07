use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use guroku::linker::{populate_node_modules, LinkedPackage};
use tempfile::TempDir;

fn fake_pkg(dir: &Path) -> PathBuf {
    let pkg = dir.join("src-A");
    std::fs::create_dir_all(&pkg).unwrap();
    std::fs::write(
        pkg.join("package.json"),
        br#"{"name":"A","version":"1.0.0"}"#,
    )
    .unwrap();
    pkg
}

fn a_pkg(source_dir: PathBuf) -> LinkedPackage {
    LinkedPackage {
        name: "A".to_string(),
        version: "1.0.0".to_string(),
        source_dir,
        dependencies: BTreeMap::new(),
    }
}

fn inner_pkg_path(node_modules: &Path) -> PathBuf {
    node_modules
        .join(".guroku")
        .join("A@1.0.0")
        .join("node_modules")
        .join("A")
}

#[test]
fn replaces_existing_top_level_dir() {
    let tmp = TempDir::new().unwrap();
    let src = fake_pkg(tmp.path());
    let node_modules = tmp.path().join("node_modules");

    let stray_dir = node_modules.join("A");
    std::fs::create_dir_all(&stray_dir).unwrap();
    let stray_file = stray_dir.join("stray.txt");
    std::fs::write(&stray_file, b"junk").unwrap();
    assert!(stray_file.is_file());

    let pkgs = vec![a_pkg(src)];
    populate_node_modules(&pkgs, &node_modules, &["A".to_string()]).unwrap();

    let top = node_modules.join("A");
    assert!(top.is_symlink(), "{} should be a symlink", top.display());
    assert!(
        !stray_file.exists(),
        "stray file {} should be gone",
        stray_file.display()
    );
}

#[cfg(unix)]
#[test]
fn replaces_existing_top_level_symlink() {
    use std::os::unix::fs::symlink;

    let tmp = TempDir::new().unwrap();
    let src = fake_pkg(tmp.path());
    let node_modules = tmp.path().join("node_modules");
    std::fs::create_dir_all(&node_modules).unwrap();

    let irrelevant = tmp.path().join("somewhere-else");
    std::fs::create_dir_all(&irrelevant).unwrap();
    let top = node_modules.join("A");
    symlink(&irrelevant, &top).unwrap();
    assert!(top.is_symlink());

    let pkgs = vec![a_pkg(src)];
    populate_node_modules(&pkgs, &node_modules, &["A".to_string()]).unwrap();

    assert!(
        top.is_symlink(),
        "{} should still be a symlink",
        top.display()
    );
    let resolved = std::fs::canonicalize(&top).unwrap();
    let expected = std::fs::canonicalize(inner_pkg_path(&node_modules)).unwrap();
    assert_eq!(resolved, expected);
}

#[test]
fn replaces_existing_top_level_file() {
    let tmp = TempDir::new().unwrap();
    let src = fake_pkg(tmp.path());
    let node_modules = tmp.path().join("node_modules");
    std::fs::create_dir_all(&node_modules).unwrap();

    let top = node_modules.join("A");
    std::fs::write(&top, b"oops, accidental touch\n").unwrap();
    assert!(top.is_file());

    let pkgs = vec![a_pkg(src)];
    populate_node_modules(&pkgs, &node_modules, &["A".to_string()]).unwrap();

    assert!(top.is_symlink(), "{} should be a symlink", top.display());
}

#[test]
fn replaces_pre_existing_inner_package_dir() {
    let tmp = TempDir::new().unwrap();
    let src = fake_pkg(tmp.path());
    let node_modules = tmp.path().join("node_modules");

    let inner = inner_pkg_path(&node_modules);
    std::fs::create_dir_all(&inner).unwrap();
    let junk = inner.join("junk.txt");
    std::fs::write(&junk, b"leftover").unwrap();
    std::fs::write(inner.join("package.json"), b"{\"name\":\"WRONG\"}").unwrap();

    let pkgs = vec![a_pkg(src)];
    populate_node_modules(&pkgs, &node_modules, &["A".to_string()]).unwrap();

    let pkg_json = inner.join("package.json");
    assert!(pkg_json.is_file(), "{} should exist", pkg_json.display());
    let contents = std::fs::read_to_string(&pkg_json).unwrap();
    assert_eq!(contents, r#"{"name":"A","version":"1.0.0"}"#);
    assert!(
        !junk.exists(),
        "stray junk file {} should be gone",
        junk.display()
    );
}
