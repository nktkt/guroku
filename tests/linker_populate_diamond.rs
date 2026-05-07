use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use guroku::linker::{populate_node_modules, LinkedPackage};
use tempfile::TempDir;

fn fake_src(root: &Path, name: &str) -> PathBuf {
    let pkg = root.join(format!("src-{name}"));
    std::fs::create_dir_all(&pkg).unwrap();
    std::fs::write(
        pkg.join("package.json"),
        format!(r#"{{"name":"{name}","version":"1.0.0"}}"#).as_bytes(),
    )
    .unwrap();
    std::fs::write(pkg.join("index.js"), b"module.exports={};\n").unwrap();
    pkg
}

fn pkg(name: &str, source_dir: PathBuf, deps: &[(&str, &str)]) -> LinkedPackage {
    let mut dependencies = BTreeMap::new();
    for (k, v) in deps {
        dependencies.insert((*k).to_string(), (*v).to_string());
    }
    LinkedPackage {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        source_dir,
        dependencies,
        bin_entries: vec![],
    }
}

/// Build the diamond: A -> C, B -> C, C standalone; direct = [A, B].
fn setup() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let a_src = fake_src(tmp.path(), "A");
    let b_src = fake_src(tmp.path(), "B");
    let c_src = fake_src(tmp.path(), "C");
    let node_modules = tmp.path().join("node_modules");
    let pkgs = vec![
        pkg("A", a_src, &[("C", "1.0.0")]),
        pkg("B", b_src, &[("C", "1.0.0")]),
        pkg("C", c_src, &[]),
    ];
    populate_node_modules(&pkgs, &node_modules, &["A".to_string(), "B".to_string()]).unwrap();
    (tmp, node_modules)
}

#[test]
fn c_materialised_only_once() {
    let (_tmp, node_modules) = setup();

    let c_pkg_dir = node_modules
        .join(".guroku")
        .join("C@1.0.0")
        .join("node_modules")
        .join("C");
    assert!(
        c_pkg_dir.is_dir(),
        "{} should exist as a directory",
        c_pkg_dir.display()
    );

    let c_root = node_modules.join(".guroku").join("C@1.0.0");
    assert!(
        c_root.is_dir(),
        "{} should be a directory",
        c_root.display()
    );
    let meta = std::fs::symlink_metadata(&c_root).unwrap();
    assert!(
        !meta.file_type().is_symlink(),
        "{} should not be a symlink",
        c_root.display()
    );
}

#[test]
fn a_has_sibling_symlink_to_c() {
    let (_tmp, node_modules) = setup();

    let sibling = node_modules
        .join(".guroku")
        .join("A@1.0.0")
        .join("node_modules")
        .join("C");
    assert!(
        sibling.is_symlink(),
        "{} should be a symlink",
        sibling.display()
    );
}

#[test]
fn b_has_sibling_symlink_to_c() {
    let (_tmp, node_modules) = setup();

    let sibling = node_modules
        .join(".guroku")
        .join("B@1.0.0")
        .join("node_modules")
        .join("C");
    assert!(
        sibling.is_symlink(),
        "{} should be a symlink",
        sibling.display()
    );
}

#[test]
fn both_sibling_symlinks_resolve_to_same_inode_pkg_dir() {
    let (_tmp, node_modules) = setup();

    let a_sibling = node_modules
        .join(".guroku")
        .join("A@1.0.0")
        .join("node_modules")
        .join("C");
    let b_sibling = node_modules
        .join(".guroku")
        .join("B@1.0.0")
        .join("node_modules")
        .join("C");
    let canonical_c = std::fs::canonicalize(
        node_modules
            .join(".guroku")
            .join("C@1.0.0")
            .join("node_modules")
            .join("C"),
    )
    .unwrap();

    let a_resolved = std::fs::canonicalize(&a_sibling).unwrap();
    let b_resolved = std::fs::canonicalize(&b_sibling).unwrap();
    assert_eq!(a_resolved, canonical_c);
    assert_eq!(b_resolved, canonical_c);
    assert_eq!(a_resolved, b_resolved);
}

#[test]
fn top_level_only_a_and_b() {
    let (_tmp, node_modules) = setup();

    let a_top = node_modules.join("A");
    let b_top = node_modules.join("B");
    let c_top = node_modules.join("C");

    assert!(
        a_top.is_symlink(),
        "{} should be a symlink",
        a_top.display()
    );
    assert!(
        b_top.is_symlink(),
        "{} should be a symlink",
        b_top.display()
    );
    assert!(
        !c_top.exists() && !c_top.is_symlink(),
        "{} should not exist at top level",
        c_top.display()
    );
}
