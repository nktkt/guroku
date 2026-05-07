use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use guroku::linker::{populate_node_modules, LinkedPackage};
use tempfile::TempDir;

fn fake_pkg(dir: &Path) -> PathBuf {
    let pkg = dir.join("src-foo");
    std::fs::create_dir_all(&pkg).unwrap();
    std::fs::write(
        pkg.join("package.json"),
        br#"{"name":"foo","version":"1.0.0"}"#,
    )
    .unwrap();
    std::fs::write(pkg.join("index.js"), b"module.exports={};\n").unwrap();
    pkg
}

fn foo_pkg(source_dir: PathBuf) -> LinkedPackage {
    LinkedPackage {
        name: "foo".to_string(),
        version: "1.0.0".to_string(),
        source_dir,
        dependencies: BTreeMap::new(),
    }
}

#[test]
fn creates_guroku_subdir() {
    let tmp = TempDir::new().unwrap();
    let src = fake_pkg(tmp.path());
    let node_modules = tmp.path().join("node_modules");
    let pkgs = vec![foo_pkg(src)];

    populate_node_modules(&pkgs, &node_modules, &["foo".to_string()]).unwrap();

    assert!(node_modules.join(".guroku").is_dir());
}

#[test]
fn materialises_package_under_guroku() {
    let tmp = TempDir::new().unwrap();
    let src = fake_pkg(tmp.path());
    let node_modules = tmp.path().join("node_modules");
    let pkgs = vec![foo_pkg(src)];

    populate_node_modules(&pkgs, &node_modules, &["foo".to_string()]).unwrap();

    let pkg_json = node_modules
        .join(".guroku")
        .join("foo@1.0.0")
        .join("node_modules")
        .join("foo")
        .join("package.json");
    assert!(pkg_json.is_file(), "{} should exist", pkg_json.display());
    let contents = std::fs::read_to_string(&pkg_json).unwrap();
    assert_eq!(contents, r#"{"name":"foo","version":"1.0.0"}"#);
}

#[test]
fn creates_top_level_symlink_for_direct_dep() {
    let tmp = TempDir::new().unwrap();
    let src = fake_pkg(tmp.path());
    let node_modules = tmp.path().join("node_modules");
    let pkgs = vec![foo_pkg(src)];

    populate_node_modules(&pkgs, &node_modules, &["foo".to_string()]).unwrap();

    let top = node_modules.join("foo");
    assert!(top.is_symlink(), "{} should be a symlink", top.display());
}

#[test]
fn top_level_symlink_resolves_to_pkg_dir() {
    let tmp = TempDir::new().unwrap();
    let src = fake_pkg(tmp.path());
    let node_modules = tmp.path().join("node_modules");
    let pkgs = vec![foo_pkg(src)];

    populate_node_modules(&pkgs, &node_modules, &["foo".to_string()]).unwrap();

    let top = node_modules.join("foo");
    let resolved = std::fs::canonicalize(&top).unwrap();
    let expected = std::fs::canonicalize(
        node_modules
            .join(".guroku")
            .join("foo@1.0.0")
            .join("node_modules")
            .join("foo"),
    )
    .unwrap();
    assert_eq!(resolved, expected);
}

#[test]
fn top_level_symlink_omitted_when_not_in_direct_deps() {
    let tmp = TempDir::new().unwrap();
    let src = fake_pkg(tmp.path());
    let node_modules = tmp.path().join("node_modules");
    let pkgs = vec![foo_pkg(src)];

    populate_node_modules(&pkgs, &node_modules, &[]).unwrap();

    let top = node_modules.join("foo");
    assert!(
        !top.exists() && !top.is_symlink(),
        "{} should not exist as a top-level entry",
        top.display()
    );
    assert!(node_modules.join(".guroku").join("foo@1.0.0").is_dir());
}
