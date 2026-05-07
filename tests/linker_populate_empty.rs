use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use guroku::linker::{populate_node_modules, LinkedPackage};
use tempfile::TempDir;

fn fake_pkg(dir: &Path, name: &str) -> PathBuf {
    let pkg = dir.join(format!("src-{name}"));
    std::fs::create_dir_all(&pkg).unwrap();
    std::fs::write(
        pkg.join("package.json"),
        format!(r#"{{"name":"{name}","version":"1.0.0"}}"#).as_bytes(),
    )
    .unwrap();
    pkg
}

fn pkg(name: &str, version: &str, source_dir: PathBuf) -> LinkedPackage {
    LinkedPackage {
        name: name.to_string(),
        version: version.to_string(),
        source_dir,
        dependencies: BTreeMap::new(),
        bin_entries: vec![],
    }
}

#[test]
fn empty_packages_creates_just_guroku_dir() {
    let tmp = TempDir::new().unwrap();
    let node_modules = tmp.path().join("node_modules");

    populate_node_modules(&[], &node_modules, &[]).unwrap();

    let guroku = node_modules.join(".guroku");
    assert!(guroku.exists(), "{} should exist", guroku.display());
    assert!(
        guroku.is_dir(),
        "{} should be a directory",
        guroku.display()
    );
    let count = std::fs::read_dir(&guroku).unwrap().count();
    assert_eq!(count, 0, ".guroku should be empty, got {count} entries");
}

#[test]
fn empty_packages_does_not_panic() {
    let tmp = TempDir::new().unwrap();
    let node_modules = tmp.path().join("node_modules");

    let result = populate_node_modules(&[], &node_modules, &[]);

    assert!(result.is_ok(), "expected Ok, got {result:?}");
}

#[test]
fn direct_deps_referencing_missing_package_no_op() {
    let tmp = TempDir::new().unwrap();
    let node_modules = tmp.path().join("node_modules");

    let result = populate_node_modules(&[], &node_modules, &["nope".to_string()]);

    assert!(result.is_ok(), "expected Ok, got {result:?}");
    let nope = node_modules.join("nope");
    assert!(
        !nope.exists() && !nope.is_symlink(),
        "{} should not exist",
        nope.display()
    );
}

#[test]
fn node_modules_path_does_not_have_to_exist() {
    let tmp = TempDir::new().unwrap();
    let node_modules = tmp.path().join("deep").join("nested").join("node_modules");
    assert!(!node_modules.exists(), "precondition: path must not exist");

    populate_node_modules(&[], &node_modules, &[]).unwrap();

    assert!(
        node_modules.is_dir(),
        "{} should have been created",
        node_modules.display()
    );
    assert!(node_modules.join(".guroku").is_dir());
}

#[test]
fn multiple_packages_no_direct_deps_creates_only_guroku_entries() {
    let tmp = TempDir::new().unwrap();
    let node_modules = tmp.path().join("node_modules");
    let a = fake_pkg(tmp.path(), "a");
    let b = fake_pkg(tmp.path(), "b");
    let c = fake_pkg(tmp.path(), "c");
    let pkgs = vec![
        pkg("a", "1.0.0", a),
        pkg("b", "2.0.0", b),
        pkg("c", "3.0.0", c),
    ];

    populate_node_modules(&pkgs, &node_modules, &[]).unwrap();

    let guroku = node_modules.join(".guroku");
    assert!(guroku.join("a@1.0.0").is_dir());
    assert!(guroku.join("b@2.0.0").is_dir());
    assert!(guroku.join("c@3.0.0").is_dir());
    let guroku_count = std::fs::read_dir(&guroku).unwrap().count();
    assert_eq!(
        guroku_count, 3,
        ".guroku should have exactly three entries, got {guroku_count}"
    );

    let top_entries: Vec<_> = std::fs::read_dir(&node_modules)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(
        top_entries.len(),
        1,
        "node_modules should only contain .guroku, got {top_entries:?}"
    );
    assert_eq!(top_entries[0], ".guroku");
}
