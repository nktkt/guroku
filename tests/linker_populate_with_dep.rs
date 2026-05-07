use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

use guroku::linker::{populate_node_modules, LinkedPackage};

fn make_pkg_src(root: &Path, name: &str, version: &str) -> PathBuf {
    let dir = root.join(format!("src-{}-{}", name, version));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("package.json"),
        format!(r#"{{"name":"{}","version":"{}"}}"#, name, version),
    )
    .unwrap();
    dir
}

fn pkg(src_root: &Path, name: &str, version: &str, deps: &[(&str, &str)]) -> LinkedPackage {
    let mut dependencies = BTreeMap::new();
    for (k, v) in deps {
        dependencies.insert((*k).to_string(), (*v).to_string());
    }
    LinkedPackage {
        name: name.to_string(),
        version: version.to_string(),
        source_dir: make_pkg_src(src_root, name, version),
        dependencies,
    }
}

#[test]
fn creates_sibling_symlink_for_dep() {
    let src_root = TempDir::new().unwrap();
    let nm_root = TempDir::new().unwrap();
    let nm = nm_root.path().join("node_modules");

    let a = pkg(src_root.path(), "A", "1.0.0", &[("B", "1.0.0")]);
    let b = pkg(src_root.path(), "B", "1.0.0", &[]);

    populate_node_modules(&[a, b], &nm, &["A".to_string()]).unwrap();

    let sibling = nm
        .join(".guroku")
        .join("A@1.0.0")
        .join("node_modules")
        .join("B");
    assert!(sibling.exists(), "sibling symlink target should exist");
    let meta = fs::symlink_metadata(&sibling).unwrap();
    assert!(
        meta.file_type().is_symlink(),
        "sibling entry should be a symlink"
    );
}

#[test]
fn sibling_symlink_resolves_to_b_pkg_dir() {
    let src_root = TempDir::new().unwrap();
    let nm_root = TempDir::new().unwrap();
    let nm = nm_root.path().join("node_modules");

    let a = pkg(src_root.path(), "A", "1.0.0", &[("B", "1.0.0")]);
    let b = pkg(src_root.path(), "B", "1.0.0", &[]);

    populate_node_modules(&[a, b], &nm, &["A".to_string()]).unwrap();

    let sibling = nm
        .join(".guroku")
        .join("A@1.0.0")
        .join("node_modules")
        .join("B");
    let resolved = fs::canonicalize(&sibling).unwrap();
    let expected_tail = Path::new(".guroku")
        .join("B@1.0.0")
        .join("node_modules")
        .join("B");
    let resolved_str = resolved.to_string_lossy().to_string();
    let tail_str = expected_tail.to_string_lossy().to_string();
    assert!(
        resolved_str.ends_with(&tail_str),
        "expected {} to end with {}",
        resolved_str,
        tail_str
    );
}

#[test]
fn top_level_only_for_direct_deps_not_transitive() {
    let src_root = TempDir::new().unwrap();
    let nm_root = TempDir::new().unwrap();
    let nm = nm_root.path().join("node_modules");

    let a = pkg(src_root.path(), "A", "1.0.0", &[("B", "1.0.0")]);
    let b = pkg(src_root.path(), "B", "1.0.0", &[]);

    populate_node_modules(&[a, b], &nm, &["A".to_string()]).unwrap();

    let top_a = nm.join("A");
    let meta_a = fs::symlink_metadata(&top_a).unwrap();
    assert!(meta_a.file_type().is_symlink(), "top-level A is a symlink");

    let top_b = nm.join("B");
    assert!(
        fs::symlink_metadata(&top_b).is_err(),
        "top-level B should not exist (transitive only)"
    );
}

#[test]
fn unresolved_dep_skipped_silently() {
    let src_root = TempDir::new().unwrap();
    let nm_root = TempDir::new().unwrap();
    let nm = nm_root.path().join("node_modules");

    let a = pkg(src_root.path(), "A", "1.0.0", &[("C", "1.0.0")]);

    populate_node_modules(&[a], &nm, &["A".to_string()]).unwrap();

    let missing = nm
        .join(".guroku")
        .join("A@1.0.0")
        .join("node_modules")
        .join("C");
    assert!(
        fs::symlink_metadata(&missing).is_err(),
        "unresolved C should not be linked"
    );
}

#[test]
fn dep_with_two_versions_both_materialise() {
    let src_root = TempDir::new().unwrap();
    let nm_root = TempDir::new().unwrap();
    let nm = nm_root.path().join("node_modules");

    let a1 = pkg(src_root.path(), "A", "1.0.0", &[]);
    let a2 = pkg(src_root.path(), "A", "2.0.0", &[]);

    populate_node_modules(&[a1, a2], &nm, &["A".to_string()]).unwrap();

    assert!(nm.join(".guroku").join("A@1.0.0").is_dir());
    assert!(nm.join(".guroku").join("A@2.0.0").is_dir());
}
