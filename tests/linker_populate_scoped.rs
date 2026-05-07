use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use guroku::linker::{populate_node_modules, LinkedPackage};
use tempfile::TempDir;

fn fake_pkg(root: &Path, slug: &str, name: &str, version: &str) -> PathBuf {
    let pkg = root.join(format!("src-{slug}"));
    std::fs::create_dir_all(&pkg).unwrap();
    let json = format!(r#"{{"name":"{name}","version":"{version}"}}"#);
    std::fs::write(pkg.join("package.json"), json.as_bytes()).unwrap();
    std::fs::write(pkg.join("index.js"), b"module.exports={};\n").unwrap();
    pkg
}

fn linked(name: &str, version: &str, source_dir: PathBuf) -> LinkedPackage {
    LinkedPackage {
        name: name.to_string(),
        version: version.to_string(),
        source_dir,
        dependencies: BTreeMap::new(),
        bin_entries: vec![],
    }
}

#[test]
fn scoped_package_creates_plus_dir() {
    let tmp = TempDir::new().unwrap();
    let src = fake_pkg(tmp.path(), "types-node", "@types/node", "20.0.0");
    let node_modules = tmp.path().join("node_modules");
    let pkgs = vec![linked("@types/node", "20.0.0", src)];

    populate_node_modules(&pkgs, &node_modules, &[]).unwrap();

    let dir = node_modules.join(".guroku").join("@types+node@20.0.0");
    assert!(dir.is_dir(), "{} should exist", dir.display());
}

#[test]
fn scoped_inner_path_preserves_scope() {
    let tmp = TempDir::new().unwrap();
    let src = fake_pkg(tmp.path(), "types-node", "@types/node", "20.0.0");
    let node_modules = tmp.path().join("node_modules");
    let pkgs = vec![linked("@types/node", "20.0.0", src)];

    populate_node_modules(&pkgs, &node_modules, &[]).unwrap();

    let pkg_json = node_modules
        .join(".guroku")
        .join("@types+node@20.0.0")
        .join("node_modules")
        .join("@types")
        .join("node")
        .join("package.json");
    assert!(pkg_json.is_file(), "{} should exist", pkg_json.display());
}

#[test]
fn scoped_top_level_symlink_preserves_scope() {
    let tmp = TempDir::new().unwrap();
    let src = fake_pkg(tmp.path(), "types-node", "@types/node", "20.0.0");
    let node_modules = tmp.path().join("node_modules");
    let pkgs = vec![linked("@types/node", "20.0.0", src)];

    populate_node_modules(&pkgs, &node_modules, &["@types/node".to_string()]).unwrap();

    let top = node_modules.join("@types").join("node");
    assert!(top.is_symlink(), "{} should be a symlink", top.display());
    let resolved = std::fs::canonicalize(&top).unwrap();
    let expected = std::fs::canonicalize(
        node_modules
            .join(".guroku")
            .join("@types+node@20.0.0")
            .join("node_modules")
            .join("@types")
            .join("node"),
    )
    .unwrap();
    assert_eq!(resolved, expected);
}

#[test]
fn scoped_with_unscoped_dep() {
    let tmp = TempDir::new().unwrap();
    let src_a = fake_pkg(tmp.path(), "scope-a", "@scope/a", "1.0.0");
    let src_b = fake_pkg(tmp.path(), "b", "b", "1.0.0");
    let node_modules = tmp.path().join("node_modules");

    let mut a_deps = BTreeMap::new();
    a_deps.insert("b".to_string(), "1.0.0".to_string());
    let pkg_a = LinkedPackage {
        name: "@scope/a".to_string(),
        version: "1.0.0".to_string(),
        source_dir: src_a,
        dependencies: a_deps,
        bin_entries: vec![],
    };
    let pkg_b = linked("b", "1.0.0", src_b);

    populate_node_modules(&[pkg_a, pkg_b], &node_modules, &[]).unwrap();

    let sibling = node_modules
        .join(".guroku")
        .join("@scope+a@1.0.0")
        .join("node_modules")
        .join("b");
    assert!(
        sibling.is_symlink() || sibling.exists(),
        "{} should exist as sibling link",
        sibling.display()
    );
    let resolved = std::fs::canonicalize(&sibling).unwrap();
    let expected = std::fs::canonicalize(
        node_modules
            .join(".guroku")
            .join("b@1.0.0")
            .join("node_modules")
            .join("b"),
    )
    .unwrap();
    assert_eq!(resolved, expected);
}

#[test]
fn unscoped_with_scoped_dep() {
    let tmp = TempDir::new().unwrap();
    let src_a = fake_pkg(tmp.path(), "a", "A", "1.0.0");
    let src_b = fake_pkg(tmp.path(), "scope-b", "@scope/b", "1.0.0");
    let node_modules = tmp.path().join("node_modules");

    let mut a_deps = BTreeMap::new();
    a_deps.insert("@scope/b".to_string(), "1.0.0".to_string());
    let pkg_a = LinkedPackage {
        name: "A".to_string(),
        version: "1.0.0".to_string(),
        source_dir: src_a,
        dependencies: a_deps,
        bin_entries: vec![],
    };
    let pkg_b = linked("@scope/b", "1.0.0", src_b);

    populate_node_modules(&[pkg_a, pkg_b], &node_modules, &[]).unwrap();

    let sibling = node_modules
        .join(".guroku")
        .join("A@1.0.0")
        .join("node_modules")
        .join("@scope")
        .join("b");
    assert!(
        sibling.is_symlink() || sibling.exists(),
        "{} should exist as sibling link",
        sibling.display()
    );
    let resolved = std::fs::canonicalize(&sibling).unwrap();
    let expected = std::fs::canonicalize(
        node_modules
            .join(".guroku")
            .join("@scope+b@1.0.0")
            .join("node_modules")
            .join("@scope")
            .join("b"),
    )
    .unwrap();
    assert_eq!(resolved, expected);
}
